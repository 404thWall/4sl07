use std::time::Duration;

use crate::management_protocole::{CommandCodec, Packet, ProtocolError, Task};
use futures::{SinkExt, StreamExt};

pub async fn start_client(addr: &str) -> Result<(), ProtocolError> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    println!("Connected to {}", addr);

    let mut framed = tokio_util::codec::Framed::new(stream, CommandCodec);
    framed.send(Packet::Connect(25565u16)).await?;
    let (mut sender, mut receiver) = framed.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let writer_task = tokio::spawn(async move {
        while let Some(packet) = rx.recv().await {
            if let Err(e) = sender.send(packet).await {
                eprintln!("send error: {}", e);
                break;
            }
        }
    });

    tx.send(Packet::Connect(25565u16)).await.ok();
    tx.send(Packet::AskForTask).await.ok();

    while let Some(incoming) = receiver.next().await {
        match incoming {
            Ok(packet) => {
                if let Some(response) = client_handle_packet(packet, tx.clone())? {
                    tx.send(response).await.ok();
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    drop(tx);
    let _ = writer_task.await;

    Ok(())
}

fn client_handle_packet(
    packet: Packet,
    tx: tokio::sync::mpsc::Sender<Packet>,
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
            tokio::spawn(async move {
                do_task(task, tx.clone()).await;
            });
            Ok(None)
        }
        p => Err(ProtocolError::UnexpectedPacket(p)),
    }
}

async fn do_task(task: Task, tx: tokio::sync::mpsc::Sender<Packet>) {
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
