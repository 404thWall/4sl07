use std::time::Duration;

use crate::management_protocole::client::ClientHandler;
use crate::management_protocole::{Packet, ProtocolError, Task};
use tokio::sync::mpsc::Sender;

pub struct MainClient;
pub struct MainServer;

impl MainClient {
    pub fn new() -> Self {
        MainClient
    }
}

impl Default for MainClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientHandler for MainClient {
    async fn on_connection_established(
        &mut self,
        tx: tokio::sync::mpsc::Sender<super::Packet>,
    ) -> Result<(), super::ProtocolError> {
        tx.send(Packet::Connect(25565u16)).await.ok();
        tx.send(Packet::AskForTask).await.ok();
        Ok(())
    }

    fn handle_packet(
        &mut self,
        packet: super::Packet,
        tx: tokio::sync::mpsc::Sender<super::Packet>,
    ) -> Result<Option<super::Packet>, super::ProtocolError> {
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
                tokio::spawn(async move {
                    do_task(task, tx.clone()).await;
                });
                Ok(None)
            }
            p => Err(ProtocolError::UnexpectedPacket(p)),
        }
    }

    async fn on_connection_ended(
        &mut self,
        _tx: Sender<super::Packet>,
    ) -> Result<(), super::ProtocolError> {
        Ok(())
    }
}

async fn do_task(task: Task, tx: Sender<Packet>) {
    match task {
        Task::Map(_key, _nkeys) => {
            tokio::time::sleep(Duration::from_secs(2)).await;
            tx.send(Packet::TaskFinished(task)).await.ok();
            tx.send(Packet::AskForTask).await.ok();
        }
        Task::Reduce(_key, _nkeys) => {
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
