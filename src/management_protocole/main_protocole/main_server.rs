use std::net::SocketAddr;

use crate::management_protocole::server::{OutMsg, ServerHandler};
use crate::management_protocole::{Packet, ProtocolError, Task};
use tokio::sync::mpsc::Sender;

use crate::tasks::{MAP_TASKS_AMOUNT, REDUCE_TASKS_AMOUNT};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, atomic};
use tokio::sync::RwLock;

static CONNECTED_FILE_PORT: LazyLock<RwLock<HashMap<String, u16>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static LAST_RECEIVED_PING: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static TASK_QUEUE: LazyLock<RwLock<Vec<Task>>> = LazyLock::new(|| RwLock::new(Vec::new()));
static MAP_TASKS_FINISHED: LazyLock<RwLock<(Vec<bool>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![false; MAP_TASKS_AMOUNT], 0)));
static REDUCE_TASKS_FINISHED: LazyLock<RwLock<(Vec<bool>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![false; REDUCE_TASKS_AMOUNT], 0)));

static MAP_RESULT_FILES: LazyLock<RwLock<HashMap<u32, HashSet<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static RESULT_FILES_SENT: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

static AVERAGE_ELAPSED_MAP_TIME: atomic::AtomicU64 = atomic::AtomicU64::new(0);
static AVERAGE_ELAPSED_REDUCE_TIME: atomic::AtomicU64 = atomic::AtomicU64::new(0);
static AVERAGE_ELAPSED_SAVE_TIME: atomic::AtomicU64 = atomic::AtomicU64::new(0);

pub struct MainServer {
    ping_task: Option<tokio::task::JoinHandle<()>>,
    address: Option<String>,
}

impl MainServer {
    pub fn new() -> Self {
        MainServer {
            ping_task: None,
            address: None,
        }
    }
}

impl Default for MainServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for MainServer {
    fn new_instance(&self) -> Self {
        MainServer::new()
    }
    async fn before_start(&mut self) -> Result<(), ProtocolError> {
        generate_map_tasks().await;
        Ok(())
    }
    async fn on_connection_established(
        &mut self,
        tx: Sender<OutMsg>,
        addr: SocketAddr,
    ) -> Result<(), ProtocolError> {
        let mut ping_tx = tx.clone();
        let ping_task = tokio::spawn(async move {
            server_ping_task(&mut ping_tx, &addr).await;
        });
        self.ping_task = Some(ping_task);
        self.address = Some(addr.to_string());
        Ok(())
    }

    async fn handle_packet(
        &mut self,
        packet: Packet,
        tx: Sender<OutMsg>,
        addr: SocketAddr,
    ) -> Result<Option<Packet>, ProtocolError> {
        match packet {
            Packet::Ping => {
                println!("Received Ping from {}, sending Pong...", addr);
                Ok(Some(Packet::Pong))
            }
            Packet::Pong => {
                // println!("Received Pong from {}", addr);
                LAST_RECEIVED_PING.write().await.insert(addr.to_string(), 0);
                Ok(None)
            }
            Packet::Connect(server_port) => {
                println!(
                    "Received Connect from {} with server port {}",
                    addr, server_port
                );
                CONNECTED_FILE_PORT
                    .write()
                    .await
                    .insert(addr.to_string(), server_port);
                Ok(None)
            }
            Packet::AskForTask => {
               //  println!("Received AskForTask from {}", addr);
                let mut queue = TASK_QUEUE.write().await;
                if queue.is_empty() {
                    if RESULT_FILES_SENT.read().await.len()
                        == CONNECTED_FILE_PORT.read().await.len()
                    {
                        println!(
                            "All result files have been sent, sending Finished task to {}",
                            addr
                        );
                        return Ok(Some(Packet::GiveTask {
                            task: Task::Finished,
                            files_hosts: Vec::new(),
                        }));
                    }
                    if REDUCE_TASKS_FINISHED.read().await.1 == REDUCE_TASKS_AMOUNT as u32
                        && !RESULT_FILES_SENT.read().await.contains(&addr.to_string())
                    {
                        println!("All tasks are finished, sending SaveFiles to {}", addr);
                        return Ok(Some(Packet::GiveTask {
                            task: Task::SaveFiles,
                            files_hosts: Vec::new(),
                        }));
                    }
                    //println!("No more tasks available for {}, sending None", addr);
                    return Ok(Some(Packet::GiveTask {
                        task: Task::None,
                        files_hosts: Vec::new(),
                    }));
                }
                let task = queue.swap_remove(0);
                let files_hosts = if let Task::Reduce(key, _) = task {
                    let list = CONNECTED_FILE_PORT.read().await.clone();
                    println!(
                        "Map result files for Reduce task {}: {:?}",
                        key,
                        MAP_RESULT_FILES.read().await.get(&key)
                    );
                    tx.send(OutMsg::MsgPacket(Packet::ConnectedWorkersList(
                        list.into_iter().collect(),
                    )))
                    .await
                    .ok();
                    MAP_RESULT_FILES
                        .read()
                        .await
                        .get(&key)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect()
                } else {
                    vec![]
                };
                println!("Assigning task {:?} to {}", task, addr);
                Ok(Some(Packet::GiveTask { task, files_hosts }))
            }
            Packet::TaskFinished {
                task,
                elapsed_time_millis,
                reduce_files,
            } => {
                println!("Received TaskFinished from {} for task: {:?}", addr, task);
                println!(
                    "Elapsed time (ms) for task {:?}: {}",
                    task, elapsed_time_millis
                );
                match task {
                    Task::Map(key, _) => {
                        let mut tuple = MAP_TASKS_FINISHED.write().await; // (vec, count)
                        if tuple.0[key as usize] {
                            println!("Task Map {} was already marked as finished, ignoring", key);
                            tx.send(OutMsg::MsgPacket(Packet::TaskValidation {
                                validated: false,
                                task,
                            }))
                            .await
                            .ok();
                        } else {
                            println!("Marking Task Map {} as finished", key);
                            AVERAGE_ELAPSED_MAP_TIME
                                .fetch_add(elapsed_time_millis as u64, atomic::Ordering::SeqCst);
                            tuple.0[key as usize] = true;
                            tuple.1 += 1;
                            let count = tuple.1;
                            drop(tuple);

                            println!(
                                "Storing resulting files for Map task {}: {:?}",
                                key, reduce_files
                            );
                            let mut map = MAP_RESULT_FILES.write().await;
                            for reduce_key in reduce_files {
                                if let Some(set) = map.get_mut(&reduce_key) {
                                    set.insert(addr.to_string());
                                } else {
                                    map.insert(reduce_key, HashSet::from([addr.to_string()]));
                                }
                            }

                            tx.send(OutMsg::MsgPacket(Packet::TaskValidation {
                                validated: true,
                                task,
                            }))
                            .await
                            .ok();

                            if count == MAP_TASKS_AMOUNT as u32 {
                                println!("All Map tasks finished, generating Reduce tasks...");
                                generate_reduce_tasks().await;
                            }
                        }
                    }
                    Task::Reduce(key, _) => {
                        let mut tuple = REDUCE_TASKS_FINISHED.write().await; // (vec, count)
                        if tuple.0[key as usize] {
                            println!(
                                "Task Reduce {} was already marked as finished, ignoring",
                                key
                            );
                            tx.send(OutMsg::MsgPacket(Packet::TaskValidation {
                                validated: false,
                                task,
                            }))
                            .await
                            .ok();
                        } else {
                            println!("Marking Task Reduce {} as finished", key);
                            AVERAGE_ELAPSED_REDUCE_TIME
                                .fetch_add(elapsed_time_millis as u64, atomic::Ordering::SeqCst);
                            tuple.0[key as usize] = true;
                            tuple.1 += 1;
                            let count = tuple.1;
                            drop(tuple);

                            tx.send(OutMsg::MsgPacket(Packet::TaskValidation {
                                validated: true,
                                task,
                            }))
                            .await
                            .ok();

                            if count == REDUCE_TASKS_AMOUNT as u32 {
                                println!("===============================");
                                println!("All Reduce tasks finished");
                                println!(
                                    "Average elapsed time (ms) for all map tasks: {}",
                                    AVERAGE_ELAPSED_MAP_TIME.load(atomic::Ordering::SeqCst)
                                        / MAP_TASKS_AMOUNT as u64
                                );
                                println!(
                                    "Average elapsed time (ms) for all reduce tasks: {}",
                                    AVERAGE_ELAPSED_REDUCE_TIME.load(atomic::Ordering::SeqCst)
                                        / REDUCE_TASKS_AMOUNT as u64
                                );
                                println!("===============================");
                            }
                        }
                    }
                    Task::SaveFiles => {
                        RESULT_FILES_SENT.write().await.insert(addr.to_string());
                        AVERAGE_ELAPSED_SAVE_TIME
                            .fetch_add(elapsed_time_millis as u64, atomic::Ordering::SeqCst);
                        println!(
                            "Received result files from {} in {} ms",
                            addr, elapsed_time_millis
                        );
                        println!(
                            "Average elapsed time (ms) for all save files tasks: {}",
                            AVERAGE_ELAPSED_SAVE_TIME.load(atomic::Ordering::SeqCst)
                                / RESULT_FILES_SENT.read().await.len() as u64
                        );
                    }
                    _ => {}
                }
                Ok(None)
            }
            Packet::AskWorkersList => {
                println!("Received AskWorkersList from {}", addr);
                let list = CONNECTED_FILE_PORT.read().await.clone();
                Ok(Some(Packet::ConnectedWorkersList(
                    list.into_iter().collect(),
                )))
            }
            _ => Ok(None),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<OutMsg>) -> Result<(), ProtocolError> {
        if let Some(task) = &self.ping_task {
            task.abort();
        }
        CONNECTED_FILE_PORT
            .write()
            .await
            .remove(&self.address.as_ref().unwrap().to_string());
        RESULT_FILES_SENT
            .write()
            .await
            .remove(&self.address.as_ref().unwrap().to_string());
        Ok(())
    }
}

async fn server_ping_task(tx: &mut Sender<OutMsg>, addr: &std::net::SocketAddr) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(10));
    loop {
        ticker.tick().await;
        // println!(
        //     "Sending Ping to {} at {:?}",
        //     addr,
        //     std::time::SystemTime::now()
        // );
        if tx.send(OutMsg::MsgPacket(Packet::Ping)).await.is_err() {
            break;
        }
        let value;
        {
            let mut map = LAST_RECEIVED_PING.write().await;
            let key = addr.to_string();
            value = map.get(&key).cloned().unwrap_or(0);
            map.insert(key.clone(), value + 1);
        }
        if value == 3 {
            println!(
                "No Pong received from {} after 3 Pings, closing connection",
                addr
            );
            tx.send(OutMsg::MsgClose).await.ok();
            break;
        }
    }
}

async fn generate_map_tasks() {
    let mut tasks = TASK_QUEUE.write().await;

    for i in 0..MAP_TASKS_AMOUNT {
        tasks.push(Task::Map(i as u32, MAP_TASKS_AMOUNT as u32));
    }
}

async fn generate_reduce_tasks() {
    let mut tasks = TASK_QUEUE.write().await;

    for i in 0..REDUCE_TASKS_AMOUNT {
        tasks.push(Task::Reduce(i as u32, REDUCE_TASKS_AMOUNT as u32));
    }

    println!("Tasks in queue: {}", tasks.len());
}
