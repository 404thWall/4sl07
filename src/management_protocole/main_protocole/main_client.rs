use std::time::Duration;

use crate::management_protocole::client::{ClientHandler, start_client};
use crate::management_protocole::file_transfer_protocole::file_client::FileClient;
use crate::management_protocole::{Packet, ProtocolError, Task};
use tokio::sync::mpsc::Sender;

pub struct MainClient{
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
            Packet::GiveTask(task) => {
                println!("Received GiveTask with task: {:?}", task);
                let connected_clients = self.connected_clients.clone();
                let file_server_port = self.file_server_port;
                tokio::spawn(async move {
                    do_task(task, tx.clone(), connected_clients, file_server_port).await;
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

async fn do_task(task: Task, tx: Sender<Packet>, connected_clients: Option<Vec<(String, u16)>>, file_server_port: u16) {
    match task {
        Task::Map(_key, _nkeys) => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            tx.send(Packet::TaskFinished(task)).await.ok();
            tx.send(Packet::AskForTask).await.ok();
        }
        Task::Reduce(_key, _nkeys) => {
            if let Some(clients) = connected_clients {
                println!("Connected clients: {:?}", clients);
                let mut i = 0;
                for (addr, port) in clients {
                    if port == file_server_port {
                        continue; // Skip the file server itself
                    }
                    let addr = addr.split(":").next().unwrap_or("127.0.0.1").to_owned() + ":" + &port.to_string();
                    println!("Connecting to worker at {}", addr);
                    let res = start_client(&addr, FileClient::new(Some(format!("data_{}.txt", i)))).await;
                    println!("Finished connecting to worker at {}: {:?}", addr, res);
                    i += 1;
                }
            } else {
                println!("No connected clients");
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
            tx.send(Packet::TaskFinished(task)).await.ok();
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
