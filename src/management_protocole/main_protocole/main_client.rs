use std::collections::HashMap;
use std::time::Duration;

use crate::management_protocole::client::{ClientHandler, start_client};
use crate::management_protocole::file_transfer_protocole::file_client::FileClient;
use crate::management_protocole::main_protocole::main_server;
use crate::management_protocole::{Packet, ProtocolError, Task};
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
        Task::Map(_key, _nkeys) => {
            // Replace with actual map function
            tokio::time::sleep(Duration::from_secs(2)).await;

            let mut reduce_files = vec![];
            for i in 0..main_server::REDUCE_TASKS_AMOUNT {
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
                for (i, addr) in files_hosts.iter().enumerate() {
                    let port = *map.get(addr).unwrap();
                    let addr = addr.split(":").next().unwrap_or("127.0.0.1").to_owned()
                        + ":"
                        + &port.to_string();
                    println!("Connecting to worker at {}", addr);
                    let res: Result<(), ProtocolError> = start_client(
                        &addr,
                        FileClient::new(Some(format!("./reduce_data/data_{}_{}", key, i)), key),
                    )
                    .await;
                    println!("Finished connecting to worker at {}: {:?}", addr, res);
                }
            } else {
                println!("No connected clients");
            }

            // Replace with actual reduce function
            tokio::time::sleep(Duration::from_secs(3)).await;

            let temp_data_folder = std::path::Path::new("./reduce_data/");
            if temp_data_folder.exists() {
                std::fs::remove_dir_all(temp_data_folder).ok();
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
            std::process::exit(0);
        }
    }
}
