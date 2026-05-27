use std::net::SocketAddr;

use crate::management_protocole::server::{OutMsg, ServerHandler};
use crate::management_protocole::{Packet, ProtocolError, Task};
use tokio::sync::mpsc::Sender;

use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;

static MAP_TASKS_AMOUNT: usize = 20;
static REDUCE_TASKS_AMOUNT: usize = 4;

static CONNECTED_FILE_PORT: LazyLock<RwLock<HashMap<String, u16>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static LAST_RECEIVED_PING: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static TASK_QUEUE: LazyLock<RwLock<Vec<Task>>> = LazyLock::new(|| RwLock::new(Vec::new()));
static MAP_TASKS_FINISHED: LazyLock<RwLock<(Vec<bool>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![false; MAP_TASKS_AMOUNT], 0)));
static REDUCE_TASKS_FINISHED: LazyLock<RwLock<(Vec<bool>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![false; REDUCE_TASKS_AMOUNT], 0)));

pub struct MainServer {
    ping_task: Option<tokio::task::JoinHandle<()>>,
    address: Option<String>,
}

impl MainServer {
    pub fn new() -> Self {
        MainServer { ping_task: None, address: None }
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
                println!("Received Pong from {}", addr);
                LAST_RECEIVED_PING.write().await.insert(addr.to_string(), 0);
                Ok(None)
            }
            Packet::Connect(server_port) => {
                println!(
                    "Received Connect from {} with server port {}",
                    addr, server_port
                );
                CONNECTED_FILE_PORT.write().await.insert(addr.to_string(), server_port);
                Ok(None)
            }
            Packet::AskForTask => {
                println!("Received AskForTask from {}", addr);
                let mut queue = TASK_QUEUE.write().await;
                if queue.is_empty() {
                    if REDUCE_TASKS_FINISHED.read().await.1 == REDUCE_TASKS_AMOUNT as u32 {
                        println!("All tasks are finished, sending None to {}", addr);
                        return Ok(Some(Packet::GiveTask(Task::Finished)));
                    }
                    println!("No more tasks available for {}, sending None", addr);
                    return Ok(Some(Packet::GiveTask(Task::None)));
                }
                let task = queue.swap_remove(0);
                if let Task::Reduce(_, _) = task {
                    let list = CONNECTED_FILE_PORT.read().await.clone();
                    tx.send(OutMsg::MsgPacket(Packet::ConnectedWorkersList(list.into_iter().collect()))).await.ok();
                }
                println!("Assigning task {:?} to {}", task, addr);
                Ok(Some(Packet::GiveTask(task)))
            }
            Packet::TaskFinished(task) => {
                println!("Received TaskFinished from {} for task: {:?}", addr, task);
                match task {
                    Task::Map(key, _) => {
                        let mut tuple = MAP_TASKS_FINISHED.write().await; // (vec, count)
                        if tuple.0[key as usize] {
                            println!("Task Map {} was already marked as finished, ignoring", key);
                        } else {
                            println!("Marking Task Map {} as finished", key);
                            tuple.0[key as usize] = true;
                            tuple.1 += 1;

                            if tuple.1 == MAP_TASKS_AMOUNT as u32 {
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
                        } else {
                            println!("Marking Task Reduce {} as finished", key);
                            tuple.0[key as usize] = true;
                            tuple.1 += 1;

                            if tuple.1 == REDUCE_TASKS_AMOUNT as u32 {
                                println!("All Reduce tasks finished");
                            }
                        }
                    }
                    _ => {}
                }
                Ok(None)
            }
            Packet::AskWorkersList => {
                println!("Received AskWorkersList from {}", addr);
                let list = CONNECTED_FILE_PORT.read().await.clone();
                Ok(Some(Packet::ConnectedWorkersList(list.into_iter().collect())))
            }
            _ => Ok(None),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<OutMsg>) -> Result<(), ProtocolError> {
        if let Some(task) = &self.ping_task {
            task.abort();
        }
        CONNECTED_FILE_PORT.write().await.remove(&self.address.as_ref().unwrap().to_string());
        Ok(())
    }
}

async fn server_ping_task(tx: &mut Sender<OutMsg>, addr: &std::net::SocketAddr) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(10));
    loop {
        ticker.tick().await;
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
