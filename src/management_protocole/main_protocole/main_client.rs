use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use crate::management_protocole::client::{ClientHandler, start_client};
use crate::management_protocole::file_transfer_protocole::file_client::FileClient;
use crate::management_protocole::{Packet, ProtocolError, Task};
use crate::tasks::{INITIAL_DATA_PATH, REDUCE_TASKS_AMOUNT};
use futures::future::join_all;
use tokio::sync::mpsc::Sender;

pub static HANDLED_MAP_TASKS: LazyLock<RwLock<HashMap<u32, bool>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

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
                    do_task(
                        task,
                        tx.clone(),
                        connected_clients,
                        file_server_port,
                        files_hosts,
                        user,
                        host_address,
                    )
                    .await;
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
                match task {
                    Task::Map(key, _) => {
                        HANDLED_MAP_TASKS.write().unwrap().insert(key, validated);
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
) {
    match task {
        Task::Map(key, _nkeys) => {
            // Keep CPU-heavy and blocking filesystem work off Tokio runtime workers.
            let begin_time = std::time::Instant::now();
            let map_result = tokio::task::spawn_blocking(move || {
                let paths = std::fs::read_dir(INITIAL_DATA_PATH)?;
                let mut candidates = vec![];
                for path in paths {
                    let path = path?.path();
                    if path.is_file()
                        && path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with("CC-MAIN-")
                    {
                        candidates.push(path);
                    }
                }
                candidates.sort();

                let path = candidates
                    .get((key as usize) % candidates.len())
                    .ok_or_else(|| std::io::Error::other("No candidate input files found"))?;

                println!("Starting Map task {} on file {}", key, path.display());
                crate::tasks::run_map_task(
                    path.to_str().unwrap(),
                    REDUCE_TASKS_AMOUNT,
                    key as usize,
                )?;
                Ok::<(), std::io::Error>(())
            })
            .await;

            match map_result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    eprintln!("Map task {} failed: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return;
                }
                Err(e) => {
                    eprintln!("Map task {} join error: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return;
                }
            };

            let mut reduce_files = vec![];
            for i in 0..REDUCE_TASKS_AMOUNT {
                reduce_files.push(i as u32);
            }

            let elapsed_time = begin_time.elapsed();
            println!("Finished Map task {} in {:?}", key, elapsed_time);
            let elapsed_time_millis = elapsed_time.as_millis();

            tx.send(Packet::TaskFinished {
                task,
                elapsed_time_millis,
                reduce_files,
            })
            .await
            .ok();
            tx.send(Packet::AskForTask).await.ok();
        }
        Task::Reduce(key, _nkeys) => {
            let begin_time = std::time::Instant::now();
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
                    }));
                }
                join_all(tasks).await;
            } else {
                println!("No connected clients");
            }

            let reduce_result = tokio::task::spawn_blocking(move || {
                crate::tasks::run_reduce_task(
                    crate::tasks::REDUCE_INITIAL_DATA_PATH,
                    key as usize,
                )?;
                println!("Finished Reduce task {}", key);

                let temp_data_folder = std::path::Path::new(crate::tasks::REDUCE_INITIAL_DATA_PATH);
                if temp_data_folder.exists() {
                    std::fs::remove_dir_all(temp_data_folder).ok();
                }
                Ok::<(), std::io::Error>(())
            })
            .await;

            match reduce_result {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    eprintln!("Reduce task {} failed: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return;
                }
                Err(e) => {
                    eprintln!("Reduce task {} join error: {}", key, e);
                    tx.send(Packet::AskForTask).await.ok();
                    return;
                }
            }
            let elapsed_time_millis = begin_time.elapsed().as_millis();
            tx.send(Packet::TaskFinished {
                task,
                elapsed_time_millis,
                reduce_files: vec![],
            })
            .await
            .ok();
            tx.send(Packet::AskForTask).await.ok();
        }
        Task::None => {
            println!("Nothing to do for now, launching a new AskForTask after 1s...");
            tokio::time::sleep(Duration::from_secs(1)).await;
            tx.send(Packet::AskForTask).await.ok();
        }
        Task::Finished => {
            println!("All tasks are finished, client is done!");
            println!("Sending all files to server...");

            send_result_files(user, host_address).await;

            println!("Cleaning up temporary files...");
            for path in crate::tasks::FOLDERS_TO_DELETE {
                let temp_data_folder: &std::path::Path = std::path::Path::new(path);
                println!("Deleting {}...", temp_data_folder.display());
                if temp_data_folder.exists() {
                    std::fs::remove_dir_all(temp_data_folder).ok();
                }
            }
            println!("Exiting...");
            std::process::exit(0);
        }
    }
}

async fn send_result_files(user: String, host_address: String) {
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
