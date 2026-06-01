use std::collections::HashMap;
use std::time::Duration;

use crate::management_protocole::client::{ClientHandler, start_client};
use crate::management_protocole::file_transfer_protocole::file_client::FileClient;
use crate::management_protocole::{Packet, ProtocolError, Task};
use crate::tasks::{INITIAL_DATA_PATH, REDUCE_TASKS_AMOUNT};
use tokio::sync::mpsc::Sender;

pub struct MainClient {
    file_server_port: u16,
    connected_clients: Option<Vec<(String, u16)>>,
}

impl MainClient {
    pub fn new(file_server_port: u16) -> Self {
        MainClient {
            file_server_port,
            connected_clients: None,
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
                tokio::spawn(async move {
                    do_task(
                        task,
                        tx.clone(),
                        connected_clients,
                        file_server_port,
                        files_hosts,
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
) {
    match task {
        Task::Map(key, _nkeys) => {
            // Keep CPU-heavy and blocking filesystem work off Tokio runtime workers.
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
                let begin_time = std::time::Instant::now();
                crate::tasks::run_map_task(path.to_str().unwrap(), REDUCE_TASKS_AMOUNT, key as usize)?;
                println!("Finished Map task {} in {:?}", key, begin_time.elapsed());
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
            }

            let mut reduce_files = vec![];
            for i in 0..REDUCE_TASKS_AMOUNT {
                reduce_files.push(i as u32);
            }

            tx.send(Packet::TaskFinished { task, reduce_files })
                .await
                .ok();
            tx.send(Packet::AskForTask).await.ok();
        }
        Task::Reduce(key, _nkeys) => {
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
                crate::tasks::run_reduce_task(crate::tasks::REDUCE_INITIAL_DATA_PATH, key as usize)?;
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
            tx.send(Packet::TaskFinished {
                task,
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
