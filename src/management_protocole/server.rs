use crate::management_protocole::{Packet, ProtocolError, CommandCodec};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{Framed};

pub async fn start_server(addr: &str) -> Result<(), ProtocolError> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        tokio::spawn(async move {
            println!("New connection from {}", addr);

            // Wrap the socket with our codec
            let mut framed = Framed::new(socket, CommandCodec);

            while let Some(result) = framed.next().await {
                let response = match result {
                    Ok(cmd) => server_handle_packet(cmd).await,
                    Err(e) => {
                        eprintln!("Protocol error: {}", e);
                        Err(e)
                    }
                };

                if let Ok(Some(packet)) = response {
                    if let Err(e) = framed.send(packet).await {
                        eprintln!("Failed to send response: {}", e);
                        break;
                    }
                }
            }

            println!("Connection from {} closed", addr);
        });
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