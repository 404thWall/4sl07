use crate::management_protocole::{Packet, ProtocolError, CommandCodec};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{Framed};
use tokio::sync::mpsc::Sender;

pub async fn start_server(addr: &str) -> Result<(), ProtocolError> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        
        // Wrap the socket with our codec
        let framed = Framed::new(socket, CommandCodec);
        
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

        let mut ping_tx = tx.clone();
        tokio::spawn(async move {
            server_ping_task(&mut ping_tx).await;
        });

        let send_back_tx = tx.clone();
        tokio::spawn(async move {
            println!("New connection from {}", addr);

            while let Some(result) = receiver.next().await {
                let response = match result {
                    Ok(cmd) => server_handle_packet(cmd).await,
                    Err(e) => {
                        eprintln!("Protocol error: {}", e);
                        Err(e)
                    }
                };

                if let Ok(Some(packet)) = response {
                    if let Err(e) = send_back_tx.send(packet).await {
                        eprintln!("Failed to send response: {}", e);
                        break;
                    }
                }
            }

            drop(tx);
            let _ = writer_task.await;

            println!("Connection from {} closed", addr);
        });
    }
}

async fn server_ping_task(tx: &mut Sender<Packet>) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(10));
    loop {
        ticker.tick().await;
        if tx.send(Packet::Ping).await.is_err() {
            break;
        }
    }
}

pub async fn server_handle_packet(packet: Packet) -> Result<Option<Packet>, ProtocolError> {
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