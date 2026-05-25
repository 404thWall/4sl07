use crate::management_protocole::{Packet, CommandCodec, ProtocolError};
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

    tx.send(Packet::Ping).await.ok();

    while let Some(incoming) = receiver.next().await {
        match incoming {
            Ok(packet) => {
                if let Some(response) = client_handle_packet(packet)? {
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

fn client_handle_packet(packet: Packet) -> Result<Option<Packet>, ProtocolError> {
    match packet {
        Packet::Ping => {
            println!("Received Ping, sending Pong...");
            Ok(Some(Packet::Pong))
        }
        Packet::Pong => {
            println!("Received Pong");
            // Handle Pong if needed
            Ok(None)
        }
        Packet::Connect(server_port) => {
            println!("Received Connect with server port {}", server_port);
            // Handle Connect if needed
            Ok(None)
        }
        Packet::AskForTask => {
            println!("Received AskForTask");
            // Handle AskForTask if needed
            Ok(None)
        }
        Packet::GiveTask(task) => {
            println!("Received GiveTask with task: {:?}", task);
            // Handle GiveTask if needed
            Ok(None)
        }
    }
}
