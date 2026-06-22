use std::fs::{self, File};
use std::io::BufWriter;
use std::net::SocketAddr;
use std::path::Path;

use crate::management_protocole::server::{OutMsg, ServerHandler};
use crate::management_protocole::{Packet, ProtocolError, Task};
use atomic_enum::atomic_enum;
use tokio::sync::mpsc::Sender;

use crate::tasks::{MAP_TASKS_AMOUNT, REDUCE_TASKS_AMOUNT, TIMING_ANALYSIS_FILE_PATH};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{LazyLock, atomic};
use tokio::sync::RwLock;

static CONNECTED_FILE_PORT: LazyLock<RwLock<HashMap<String, u16>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static LAST_RECEIVED_PING: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static MAP_TASK_QUEUE: LazyLock<RwLock<Vec<Task>>> = LazyLock::new(|| RwLock::new(Vec::new()));
static REDUCE_TASK_QUEUE: LazyLock<RwLock<Vec<Task>>> = LazyLock::new(|| RwLock::new(Vec::new()));

static MAP_TASKS_FINISHED: LazyLock<RwLock<(Vec<String>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![String::new(); MAP_TASKS_AMOUNT], 0)));
static REDUCE_TASKS_FINISHED: LazyLock<RwLock<(Vec<String>, u32)>> =
    LazyLock::new(|| RwLock::new((vec![String::new(); REDUCE_TASKS_AMOUNT], 0)));

// Contains for each reduce task (key), the set of worker addresses that have the relevant map files
// for this reduce task. Used to send the list of workers to the worker that will execute the reduce task.
static MAP_RESULT_FILES: LazyLock<RwLock<HashMap<u32, HashSet<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Contains for each client address, the map task currently being executed by the client.
// static MAP_TASKS_IN_PROGRESS: LazyLock<RwLock<HashMap<String, Option<u32>>>> =
//     LazyLock::new(|| RwLock::new(HashMap::new()));

static TASKS_IN_PROGRESS: LazyLock<RwLock<HashMap<String, Option<Task>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

// Contains the set of worker addresses that have already sent their result files to the main server.
static RESULT_FILES_SENT: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

static MAIN_TIME: LazyLock<std::time::Instant> = LazyLock::new(std::time::Instant::now);

// Using a BTreeMap instead of a HashMap to ensure the order of the phases is preserved when serializing to JSON
#[allow(clippy::type_complexity)]
static TIMING_ANALYSIS: LazyLock<RwLock<BTreeMap<ProtocolePhase, Vec<BTreeMap<String, f64>>>>> =
    LazyLock::new(|| RwLock::new(BTreeMap::new()));

static AVERAGE_ELAPSED_MAP_TIME: atomic::AtomicU64 = atomic::AtomicU64::new(0);
static AVERAGE_ELAPSED_REDUCE_TIME: atomic::AtomicU64 = atomic::AtomicU64::new(0);
static AVERAGE_ELAPSED_SAVE_TIME: atomic::AtomicU64 = atomic::AtomicU64::new(0);
static CURRENT_PHASE: AtomicProtocolePhase = AtomicProtocolePhase::new(ProtocolePhase::Map);

#[atomic_enum]
#[derive(PartialEq, Eq, Hash, serde::Serialize, PartialOrd, Ord)]
pub enum ProtocolePhase {
    Map,
    Reduce,
    SaveFiles,
    Finished,
}

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
        generate_reduce_tasks().await;
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

                // Initialize the main time when the first worker connects
                println!("Main time initialized since: {:?}", MAIN_TIME.elapsed());
                Ok(None)
            }
            Packet::AskForTask => on_ask_for_task(addr, tx).await,
            Packet::TaskFinished {
                task,
                elapsed_time_millis,
                timing_analysis,
                reduce_files,
            } => {
                println!("Received TaskFinished from {} for task: {:?}", addr, task);
                println!(
                    "Elapsed time (ms) for task {:?}: {}",
                    task, elapsed_time_millis
                );

                // Remove the task from the in progress map if the worker has not been reassigned another task since then
                println!("Removing task in progress for worker {}", addr);
                TASKS_IN_PROGRESS
                    .write()
                    .await
                    .insert(addr.to_string(), None);

                match task {
                    Task::Map(key, _) => {
                        add_timing_analysis(ProtocolePhase::Map, timing_analysis).await;
                        on_map_task_finished(
                            key,
                            addr,
                            task,
                            elapsed_time_millis,
                            tx.clone(),
                            reduce_files,
                        )
                        .await
                    }
                    Task::Reduce(key, _) => {
                        add_timing_analysis(ProtocolePhase::Reduce, timing_analysis).await;
                        on_reduce_task_finished(key, addr, task, elapsed_time_millis, tx.clone())
                            .await
                    }
                    Task::SaveFiles => {
                        add_timing_analysis(ProtocolePhase::SaveFiles, timing_analysis).await;
                        on_files_saved(addr, elapsed_time_millis, tx.clone()).await
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
            Packet::TaskAborted { task } => {
                println!("Received TaskAborted from {} for task: {:?}", addr, task);
                // Remove from task in progress
                TASKS_IN_PROGRESS
                    .write()
                    .await
                    .insert(addr.to_string(), None);
                // Add to the queue
                println!(
                    "Re-queuing task {:?} since it was aborted by {}",
                    task, addr
                );
                match task {
                    Task::Map(_, _) => MAP_TASK_QUEUE.write().await.push(task),
                    Task::Reduce(_, _) => REDUCE_TASK_QUEUE.write().await.push(task),
                    _ => {}
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<OutMsg>) -> Result<(), ProtocolError> {
        println!("Connection with {} ended", self.address.as_ref().unwrap());
        let addr = self.address.as_ref().unwrap().to_string();

        // Stop the ping task
        if let Some(task) = &self.ping_task {
            task.abort();
        }
        println!("Ping task for {} stopped", addr);

        return end_connection(addr).await;
    }
}

async fn end_connection(addr: String) -> Result<(), ProtocolError> {
    // Remove the worker from the connected workers
    if CONNECTED_FILE_PORT.write().await.remove(&addr).is_none() {
        println!("Worker {} was not in the connected workers list, ignoring", addr);
        return Ok(());
    }
    println!("Worker {} removed from connected workers", addr);

    if CURRENT_PHASE.load(atomic::Ordering::SeqCst) == ProtocolePhase::Finished {
        println!(
            "Worker {} disconnected after the protocole is finished, ignoring",
            addr
        );

        // If there are no more connected workers, stop the server
        if CONNECTED_FILE_PORT.read().await.is_empty() {
            let path = Path::new(TIMING_ANALYSIS_FILE_PATH);
            let save_directory = path.parent().unwrap();
            fs::create_dir_all(save_directory)?;

            let write_file = File::create(path)?;
            let writer = BufWriter::new(write_file);
            let timing_analysis = TIMING_ANALYSIS.read().await;
            let e = serde_json::to_writer_pretty(writer, &*timing_analysis);
            if e.is_err() {
                println!("Error writing : {:?}", e);
            }

            println!("================================");
            println!("All workers disconnected, stopping server...");
            let elapsed_time = MAIN_TIME.elapsed();
            println!("Total elapsed time: {:?}", elapsed_time);
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
            println!("================================");
            std::process::exit(0);
        }

        return Ok(());
    }

    // Mark any map task assigned to this worker as unfinished so that it can be reassigned to another worker
    let mut finished_map_tasks = MAP_TASKS_FINISHED.write().await;
    for i in 0..MAP_TASKS_AMOUNT {
        if finished_map_tasks.0[i] == addr {
            println!(
                "Worker {} disconnected after having done Map task {}, marking it as unfinished",
                addr, i
            );
            finished_map_tasks.0[i] = String::new();
            finished_map_tasks.1 -= 1;
            MAP_TASK_QUEUE
                .write()
                .await
                .push(Task::Map(i as u32, MAP_TASKS_AMOUNT as u32));
            CURRENT_PHASE.store(ProtocolePhase::Map, atomic::Ordering::SeqCst);
            RESULT_FILES_SENT.write().await.clear();
        }
    }
    drop(finished_map_tasks);

    let mut finished_reduce_tasks = REDUCE_TASKS_FINISHED.write().await;
    for i in 0..REDUCE_TASKS_AMOUNT {
        if finished_reduce_tasks.0[i] == addr {
            println!(
                "Worker {} disconnected after having done Reduce task {}, marking it as unfinished",
                addr, i
            );
            finished_reduce_tasks.0[i] = String::new();
            finished_reduce_tasks.1 -= 1;
            REDUCE_TASK_QUEUE
                .write()
                .await
                .push(Task::Reduce(i as u32, REDUCE_TASKS_AMOUNT as u32));
            // Unfortunately, it isn't atomic...
            if CURRENT_PHASE.load(atomic::Ordering::SeqCst) != ProtocolePhase::Map {
                CURRENT_PHASE.store(ProtocolePhase::Reduce, atomic::Ordering::SeqCst);
            }
            RESULT_FILES_SENT.write().await.clear();
        }
    }
    drop(finished_reduce_tasks);

    let in_progress = TASKS_IN_PROGRESS.read().await.get(&addr).cloned();
    if let Some(Some(task)) = in_progress {
        println!(
            "Worker {} disconnected during task {:?}, marking it as unfinished",
            addr, task
        );
        TASKS_IN_PROGRESS
            .write()
            .await
            .insert(addr.to_string(), None);
        match task {
            Task::Map(_, _) => MAP_TASK_QUEUE.write().await.push(task),
            Task::Reduce(_, _) => REDUCE_TASK_QUEUE.write().await.push(task),
            _ => {}
        };
    }

    let mut map_result_files = MAP_RESULT_FILES.write().await;
    for (reduce_key, set) in map_result_files.iter_mut() {
        if set.remove(&addr) {
            println!(
                "Worker {} had result files for Reduce task {}, removing it from the list of workers for this task",
                addr, reduce_key
            );
        }
    }
    drop(map_result_files);

    // Remove the worker from the result files sent
    RESULT_FILES_SENT.write().await.remove(&addr);
    Ok(())
}

async fn add_timing_analysis(protocol_phase: ProtocolePhase, timing_analysis: Vec<(String, f64)>) {
    let mut timing_analysis_map = TIMING_ANALYSIS.write().await;
    let mut phase_timings = timing_analysis_map
        .get_mut(&protocol_phase)
        .cloned()
        .unwrap_or_default();
    let mut task_timings = BTreeMap::new();
    for (phase, time) in timing_analysis {
        task_timings.insert(phase, time);
    }
    task_timings.insert("global_time".to_string(), MAIN_TIME.elapsed().as_secs_f64());
    phase_timings.push(task_timings);
    timing_analysis_map.insert(protocol_phase, phase_timings);
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
            end_connection(addr.to_string()).await.ok();

            break;
        }
    }
}

async fn generate_map_tasks() {
    let mut tasks = MAP_TASK_QUEUE.write().await;

    for i in 0..MAP_TASKS_AMOUNT {
        tasks.push(Task::Map(i as u32, MAP_TASKS_AMOUNT as u32));
    }
}

async fn generate_reduce_tasks() {
    let mut tasks = REDUCE_TASK_QUEUE.write().await;

    for i in 0..REDUCE_TASKS_AMOUNT {
        tasks.push(Task::Reduce(i as u32, REDUCE_TASKS_AMOUNT as u32));
    }

    println!("Tasks in queue: {}", tasks.len());
}

async fn on_map_task_finished(
    key: u32,
    addr: SocketAddr,
    task: Task,
    elapsed_time_millis: u128,
    tx: Sender<OutMsg>,
    reduce_files: Vec<u32>,
) {
    let mut tuple = MAP_TASKS_FINISHED.write().await; // (vec, count)
    if !tuple.0[key as usize].is_empty() {
        println!(
            "Task Map {} was already marked as finished by {}, ignoring",
            key, tuple.0[key as usize]
        );
        tx.send(OutMsg::MsgPacket(Packet::TaskValidation {
            validated: false,
            task,
        }))
        .await
        .ok();
    } else {
        println!("Marking Task Map {} as finished", key);

        // Add the elapsed time for this map task to the average
        AVERAGE_ELAPSED_MAP_TIME.fetch_add(elapsed_time_millis as u64, atomic::Ordering::SeqCst);

        // Mark the map task as finished globally
        tuple.0[key as usize] = addr.to_string();
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
            CURRENT_PHASE.store(ProtocolePhase::Reduce, atomic::Ordering::SeqCst);
            if REDUCE_TASKS_FINISHED.read().await.1 == REDUCE_TASKS_AMOUNT as u32 {
                println!("All Reduce tasks already finished, moving to SaveFiles phase...");
                CURRENT_PHASE.store(ProtocolePhase::SaveFiles, atomic::Ordering::SeqCst);
            }
        }
    }
}

async fn on_reduce_task_finished(
    key: u32,
    addr: SocketAddr,
    task: Task,
    elapsed_time_millis: u128,
    tx: Sender<OutMsg>,
) {
    let mut tuple = REDUCE_TASKS_FINISHED.write().await; // (vec, count)
    if !tuple.0[key as usize].is_empty() {
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
        AVERAGE_ELAPSED_REDUCE_TIME.fetch_add(elapsed_time_millis as u64, atomic::Ordering::SeqCst);
        tuple.0[key as usize] = addr.to_string();
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
            CURRENT_PHASE.store(ProtocolePhase::SaveFiles, atomic::Ordering::SeqCst);
            println!("===============================");
            println!("All Reduce tasks finished");
            println!(
                "Average elapsed time (ms) for all map tasks: {}",
                AVERAGE_ELAPSED_MAP_TIME.load(atomic::Ordering::SeqCst) / MAP_TASKS_AMOUNT as u64
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

async fn on_files_saved(addr: SocketAddr, elapsed_time_millis: u128, _tx: Sender<OutMsg>) {
    println!(
        "Received result files from {} in {} ms",
        addr, elapsed_time_millis
    );
    // Return if the protocol phase has changed
    let phase = CURRENT_PHASE.load(atomic::Ordering::SeqCst);
    if phase != ProtocolePhase::SaveFiles && phase != ProtocolePhase::Finished {
        println!("The current protocole phase has changed, the result was not accepted");
        return;
    }

    RESULT_FILES_SENT.write().await.insert(addr.to_string());
    AVERAGE_ELAPSED_SAVE_TIME.fetch_add(elapsed_time_millis as u64, atomic::Ordering::SeqCst);
    println!(
        "Average elapsed time (ms) for all save files tasks: {}",
        AVERAGE_ELAPSED_SAVE_TIME.load(atomic::Ordering::SeqCst)
            / RESULT_FILES_SENT.read().await.len() as u64
    );
    if RESULT_FILES_SENT.read().await.len() == CONNECTED_FILE_PORT.read().await.len() {
        println!("All result files have been sent, sending Finished task to all workers...");
        CURRENT_PHASE.store(ProtocolePhase::Finished, atomic::Ordering::SeqCst);
        // TODO Broadcast end message here instead of waiting for workers to ask for task again
    }
}

async fn on_ask_for_task(
    addr: SocketAddr,
    tx: Sender<OutMsg>,
) -> Result<Option<Packet>, ProtocolError> {
    let mut tasks_in_progress = TASKS_IN_PROGRESS.write().await;
    match tasks_in_progress.get(&addr.to_string()) {
        Some(Some(_)) => {
            println!(
                "Worker {} is already executing a task, sending None task",
                addr
            );
            Ok(Some(Packet::GiveTask {
                task: Task::None,
                files_hosts: vec![],
            }))
        }
        _ => {
            let phase = CURRENT_PHASE.load(atomic::Ordering::SeqCst);
            // Get the task queue or return if not needed
            let mut queue: tokio::sync::RwLockWriteGuard<'_, Vec<Task>> = match phase {
                ProtocolePhase::Map => MAP_TASK_QUEUE.write().await,
                ProtocolePhase::Reduce => REDUCE_TASK_QUEUE.write().await,
                ProtocolePhase::Finished => {
                    return Ok(Some(Packet::GiveTask {
                        task: Task::Finished,
                        files_hosts: vec![],
                    }));
                }
                ProtocolePhase::SaveFiles => {
                    return Ok(Some(Packet::GiveTask {
                        task: if !RESULT_FILES_SENT.read().await.contains(&addr.to_string()) {
                            Task::SaveFiles
                        } else {
                            Task::None
                        },
                        files_hosts: vec![],
                    }));
                }
            };
            // No more tasks to send
            if queue.is_empty() {
                return Ok(Some(Packet::GiveTask {
                    task: Task::None,
                    files_hosts: Vec::new(),
                }));
            }

            let task = queue.swap_remove(0);
            // Mark task as in progress
            tasks_in_progress.insert(addr.to_string(), Some(task.clone()));
            // Get hosts to send
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
    }
}
