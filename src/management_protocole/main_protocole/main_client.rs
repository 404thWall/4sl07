use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use super::downloader;
use crate::management_protocole::client::{ClientHandler, start_client};
use crate::management_protocole::file_transfer_protocole::file_client::FileClient;
use crate::management_protocole::{Packet, ProtocolError, Task};
use crate::tasks::REDUCE_TASKS_AMOUNT;
use futures::future::join_all;
use tokio::sync::OnceCell;
use tokio::sync::mpsc::Sender;

pub static HANDLED_MAP_TASKS: LazyLock<RwLock<HashMap<u32, bool>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
pub static HANDLED_REDUCE_TASKS: LazyLock<RwLock<HashMap<u32, bool>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub static FILES_LINK_LIST: OnceCell<Vec<String>> = OnceCell::const_new();

pub struct MainClient {
    file_server_port: u16,
    connected_clients: Option<Vec<(String, u16)>>,
    user: String,
    host_address: String,
}

impl MainClient {
    pub fn new(file_server_port: u16, user: String, host_address: String) -> Self {
        MainClient {
            file_server_port,
            connected_clients: None,
            user,
            host_address,
        }
    }
}

impl ClientHandler for MainClient {
    async fn on_connection_established(&mut self, tx: Sender<Packet>) -> Result<(), ProtocolError> {
        tx.send(Packet::Connect(self.file_server_port)).await.ok();
        tx.send(Packet::AskForTask).await.ok();
        Ok(())
    }

    fn handle_packet(
        &mut self,
        packet: Packet,
        tx: Sender<Packet>,
    ) -> Result<Option<Packet>, ProtocolError> {
        match packet {
            Packet::Ping => {
                println!("Received Ping, sending Pong...");
                Ok(Some(Packet::Pong))
            }
            Packet::Pong => {
                println!("Received Pong");
                Ok(None)
            }
            Packet::GiveTask { task, files_hosts } => {
                println!(
                    "Received GiveTask with task: {:?} and files_hosts: {:?}",
                    task, files_hosts
                );
                let connected_clients = self.connected_clients.clone();
                let file_server_port = self.file_server_port;
                let user = self.user.clone();
                let host_address = self.host_address.clone();
                tokio::spawn(async move {
                    if let Err(e) = do_task(
                        task.clone(),
                        tx.clone(),
                        connected_clients,
                        file_server_port,
                        files_hosts,
                        user,
                        host_address,
                    )
                    .await
                    {
                        eprintln!("Error executing a task: {}", e);
                        // TODO: maybe send a specific packet to the server to notify about the failure, so it can decide to retry or not
                        // Otherwise, we can disconnect, and the server will reassign the task to another client
                        tx.send(Packet::TaskAborted { task }).await.ok();
                        tokio::time::sleep(Duration::from_secs(30 + rand::random_range(0..=15)))
                            .await;
                        tx.send(Packet::AskForTask).await.ok();
                    }
                });
                println!("Launched task in background");
                Ok(None)
            }
            Packet::ConnectedWorkersList(list) => {
                println!("Received ConnectedWorkersList with list: {:?}", list);
                self.connected_clients = Some(list);
                Ok(None)
            }
            Packet::TaskValidation { validated, task } => {
                println!(
                    "Received TaskValidation for task {:?} with validated = {}",
                    task, validated
                );
                match task {
                    Task::Map(key, _) => {
                        HANDLED_MAP_TASKS.write().unwrap().insert(key, validated);
                    }
                    Task::Reduce(key, _) => {
                        HANDLED_REDUCE_TASKS.write().unwrap().insert(key, validated);
                    }
                    _ => {}
                }
                Ok(None)
            }
            p => Err(ProtocolError::UnexpectedPacket(p)),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<Packet>) -> Result<(), ProtocolError> {
        Ok(())
    }
}

async fn do_task(
    task: Task,
    tx: Sender<Packet>,
    connected_clients: Option<Vec<(String, u16)>>,
    _file_server_port: u16,
    files_hosts: Vec<String>,
    user: String,
    host_address: String,
) -> Result<(), ProtocolError> {
    match task {
        Task::Map(key, _nkeys) => {
            let begin_time = std::time::Instant::now();

            let path = tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    // Get the file link corresponding to the key and download it
                    println!("Starting Map task {}: downloading file...", key);
                    let links = FILES_LINK_LIST
                        .get_or_init(async || {
                            downloader::list_commoncrawl_files(crate::tasks::TMP_DIR)
                                .await
                                .unwrap()
                        })
                        .await;
                    let link = links.get((key as usize) % links.len()).unwrap();
                    let path = format!("{}CC-MAIN-{}", crate::tasks::TMP_DIR, key);
                    downloader::get_commoncrawl_file(link, &path).await
                })
            })
            .await
            .map_err(|e| {
                ProtocolError::TaskFailed(format!("Map task {} download join error: {}", key, e))
            })?
            .map_err(|e| {
                ProtocolError::TaskFailed(format!("Map task {} download error: {:?}", key, e))
            })?;

            let download_time = begin_time.elapsed();
            println!(
                "File downloaded for Map task {} in {:?}",
                key, download_time
            );

            // Keep CPU-heavy and blocking filesystem work off Tokio runtime workers.
            let map_result = tokio::task::spawn_blocking(move || {
                println!(
                    "Starting Map task {} on file {} after {:?} passed to list files",
                    key,
                    path,
                    begin_time.elapsed()
                );
                let timings = crate::tasks::run_map_task(&path, REDUCE_TASKS_AMOUNT, key as usize)?;
                for (phase, time) in timings.iter() {
                    println!(
                        "[Time] Map task {} - Phase {}: {} seconds",
                        key, phase, time
                    );
                }
                std::fs::remove_file(path).ok();
                Ok::<Vec<(String, f64)>, std::io::Error>(timings)
            })
            .await;

            let mut timing_analysis = match map_result {
                Ok(Ok(timings)) => timings,
                Ok(Err(e)) => {
                    eprintln!("Map task {} failed: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return Err(ProtocolError::TaskFailed(format!(
                        "Map task {} failed: {}",
                        key, e
                    )));
                }
                Err(e) => {
                    eprintln!("Map task {} join error: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return Err(ProtocolError::TaskFailed(format!(
                        "Map task {} join error: {}",
                        key, e
                    )));
                }
            };

            let mut reduce_files = vec![];
            for i in 0..REDUCE_TASKS_AMOUNT {
                reduce_files.push(i as u32);
            }

            let elapsed_time = begin_time.elapsed();
            println!("Finished Map task {} in {:?}", key, elapsed_time);
            let elapsed_time_millis = elapsed_time.as_millis();

            timing_analysis.push(("download_time".to_string(), download_time.as_secs_f64()));
            timing_analysis.push(("total_time".to_string(), elapsed_time.as_secs_f64()));

            tx.send(Packet::TaskFinished {
                task,
                elapsed_time_millis,
                timing_analysis,
                reduce_files,
            })
            .await
            .ok();
            tx.send(Packet::AskForTask).await.ok();
            Ok(())
        }
        Task::Reduce(key, _nkeys) => {
            let begin_time = std::time::Instant::now();
            let mut timing_analysis: Vec<(String, f64)> = vec![];

            let temp_data_folder = std::path::Path::new(crate::tasks::REDUCE_INITIAL_DATA_PATH);
            if temp_data_folder.exists() {
                std::fs::remove_dir_all(temp_data_folder).ok();
            }
            std::fs::create_dir_all(temp_data_folder).unwrap();

            if let Some(clients) = connected_clients {
                println!("Connected clients: {:?}", clients);
                let map: HashMap<String, u16> = HashMap::from_iter(
                    clients.iter().map(|(addr, port)| (addr.to_string(), *port)),
                );
                let mut tasks = Vec::new();
                for (i, addr) in files_hosts.iter().enumerate() {
                    let port = *map.get(addr).unwrap();
                    let addr = addr.split(":").next().unwrap_or("127.0.0.1").to_owned()
                        + ":"
                        + &port.to_string();
                    tasks.push(tokio::spawn(async move {
                        println!("Connecting to worker at {}", addr);
                        let res: Result<(), ProtocolError> = start_client(
                            &addr,
                            FileClient::new(
                                Some(format!(
                                    "{}data_{}_{}",
                                    crate::tasks::REDUCE_INITIAL_DATA_PATH,
                                    key,
                                    i
                                )),
                                key,
                            ),
                        )
                        .await;
                        println!("Finished connecting to worker at {}: {:?}", addr, res);
                        res
                    }));
                }
                let joined_results = join_all(tasks).await;
                for res in joined_results {
                    if let Err(e) = res {
                        eprintln!("Error in Reduce task {}: {}", key, e);
                        return Err(ProtocolError::TaskFailed(format!(
                            "Reduce task {} error: {}",
                            key, e
                        )));
                    } else if let Err(e) = res.unwrap() {
                        match e {
                            ProtocolError::ClosingConnection => {
                                // This error is expected when the file transfer is done, so we can ignore it
                                println!(
                                    "File transfer completed for Reduce task {}, closing connection",
                                    key
                                );
                            }
                            _ => {
                                eprintln!(
                                    "Connection Protocole Error in Reduce task {}: {}",
                                    key, e
                                );
                                return Err(ProtocolError::TaskFailed(format!(
                                    "Reduce task {} connection protocole error: {}",
                                    key, e
                                )));
                            }
                        }
                    }
                }
            } else {
                println!("No connected clients");
            }
            timing_analysis.push((
                "file_transfer".to_string(),
                begin_time.elapsed().as_secs_f64(),
            ));

            let reduce_begin = std::time::Instant::now();
            let reduce_result = tokio::task::spawn_blocking(move || {
                let stats = crate::tasks::run_reduce_task(
                    crate::tasks::REDUCE_INITIAL_DATA_PATH,
                    key as usize,
                )?;
                println!("Finished Reduce task {}", key);

                let temp_data_folder = std::path::Path::new(crate::tasks::REDUCE_INITIAL_DATA_PATH);
                if temp_data_folder.exists() {
                    std::fs::remove_dir_all(temp_data_folder).ok();
                }
                Ok::<Vec<(String, f64)>, std::io::Error>(stats)
            })
            .await;

            let timings = match reduce_result {
                Ok(Ok(stats)) => stats,
                Ok(Err(e)) => {
                    eprintln!("Reduce task {} failed: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return Err(ProtocolError::TaskFailed(format!(
                        "Reduce task {} failed: {}",
                        key, e
                    )));
                }
                Err(e) => {
                    eprintln!("Reduce task {} join error: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return Err(ProtocolError::TaskFailed(format!(
                        "Reduce task {} join error: {}",
                        key, e
                    )));
                }
            };
            let elapsed_time = begin_time.elapsed();

            timing_analysis.push(("reduce_time".to_string(), reduce_begin.elapsed().as_secs_f64()));
            timing_analysis.push(("total_time".to_string(), elapsed_time.as_secs_f64()));
            for t in timings {
                timing_analysis.push(t);
            }

            tx.send(Packet::TaskFinished {
                task,
                elapsed_time_millis: elapsed_time.as_millis(),
                timing_analysis,
                reduce_files: vec![],
            })
            .await
            .ok();
            tx.send(Packet::AskForTask).await.ok();
            Ok(())
        }
        Task::None => {
            println!("Nothing to do for now, launching a new AskForTask after 1s...");
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Packet::AskForTask).await.ok();
            Ok(())
        }
        Task::SaveFiles => {
            println!("Received SaveFiles task, preparing files for sending...");
            let begin_time = std::time::Instant::now();
            tokio::task::spawn_blocking(move || {
                tokio::runtime::Handle::current().block_on(async {
                    prepare_files_for_sending().await;
                    send_result_files(user, host_address).await;
                });
            })
            .await
            .unwrap();
            let elapsed_time = begin_time.elapsed();
            let timing_analysis = vec![("total_time".to_string(), elapsed_time.as_secs_f64())];
            println!("Finished SaveFiles task, asking for next task...");
            tx.send(Packet::TaskFinished {
                task,
                elapsed_time_millis: elapsed_time.as_millis(),
                timing_analysis,
                reduce_files: vec![],
            })
            .await
            .ok();
            tx.send(Packet::AskForTask).await.ok();
            Ok(())
        }
        Task::Finished => {
            println!("All tasks are finished, client is done!");
            println!("Cleaning up temporary files...");
            super::clean_temporary_files();
            println!("Exiting...");
            std::process::exit(0);
        }
    }
}

async fn prepare_files_for_sending() {
    std::fs::create_dir_all(crate::tasks::RESULT_PATH).unwrap();
    let paths = std::fs::read_dir(crate::tasks::RESULT_PATH).unwrap();
    for path in paths {
        let path = path.unwrap();

        // Assuming files are named like "result_{reduce_key}.mapdata"
        let id = path
            .file_name()
            .to_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap()
            .split('_')
            .nth(1)
            .unwrap()
            .parse::<u32>()
            .unwrap();

        while !HANDLED_REDUCE_TASKS.read().unwrap().contains_key(&id) {
            println!("Waiting for reduce task {} to be validated...", id);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        if !HANDLED_REDUCE_TASKS.read().unwrap().get(&id).unwrap() {
            println!("Reduce task {} was not validated, deleting file...", id);
            std::fs::remove_file(path.path()).ok();
        }
    }
}

async fn send_result_files(user: String, host_address: String) {
    if user == "test" {
        println!("Test user detected, skipping sending files.");
        return;
    }
    let mut tries = 0;
    loop {
        let command_str = format!(
            "scp -r {} {}@{}:/tmp/4sl07_grp3/",
            crate::tasks::RESULT_PATH,
            user,
            host_address
        );

        if let Ok(c_command) = CString::new(command_str) {
            unsafe {
                // Appelle directement le système pour lancer la commande via /bin/sh
                let status = libc::system(c_command.as_ptr());
                if status == 0 {
                    println!("Files successfully sent !");
                    return;
                } else {
                    println!("Error executing scp: {}", status);
                }
            }
        } else {
            eprintln!("Error creating scp command");
        }

        tries += 1;
        if tries >= 10 {
            eprintln!("Failed after 10 attempts, giving up.");
            return;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
