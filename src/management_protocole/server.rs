use crate::management_protocole::server::OutMsg::MsgClose;
use crate::management_protocole::{Packet, ProtocolError, CommandCodec};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::{Framed};
use tokio::sync::mpsc::Sender;
use std::sync::LazyLock;
use std::collections::HashMap;
use tokio::sync::RwLock;

static LAST_RECEIVED_PING: LazyLock<RwLock<HashMap<String, u32>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

enum OutMsg {
    MsgPacket(Packet),
    MsgClose,
}

pub async fn start_server(addr: &str) -> Result<(), ProtocolError> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (socket, addr) = listener.accept().await?;
        
        // Wrap the socket with our codec
        let framed = Framed::new(socket, CommandCodec);
        
        let (mut sender, mut receiver) = framed.split();
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        let writer_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    OutMsg::MsgPacket(packet) => {
                        if let Err(e) = sender.send(packet).await {
                            eprintln!("send error: {}", e);
                            break;
                        }
                    }
                    OutMsg::MsgClose => {
                        println!("Closing connection to {}", addr);
                        sender.close().await.ok();
                        break;
                    }
                }
            }
        });

        let mut ping_tx = tx.clone();
        let ping_task = tokio::spawn(async move {
            server_ping_task(&mut ping_tx, &addr).await;
        });

        let send_back_tx = tx.clone();
        tokio::spawn(async move {
            println!("New connection from {}", addr);

            while let Some(result) = receiver.next().await {
                let response = match result {
                    Ok(cmd) => server_handle_packet(cmd, &addr).await,
                    Err(e) => {
                        eprintln!("Protocol error: {}", e);
                        Err(e)
                    }
                };

                if let Ok(Some(packet)) = response {
                    if let Err(e) = send_back_tx.send(OutMsg::MsgPacket(packet)).await {
                        eprintln!("Failed to send response: {}", e);
                        break;
                    }
                }
            }

            send_back_tx.send(MsgClose).await.ok();
            drop(tx);
            ping_task.abort();
            let _ = writer_task.await;

            println!("Connection from {} closed", addr);
        });
    }
}

async fn server_ping_task(tx: &mut Sender<OutMsg>, addr: &std::net::SocketAddr) {
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(10));
    loop {
        ticker.tick().await;
        if tx.send(OutMsg::MsgPacket(Packet::Ping)).await.is_err() {
            break;
        }
        let value;
        {
            let mut map = LAST_RECEIVED_PING.write().await;
            let key = addr.to_string();
            value = map.get(&key).cloned().unwrap_or(0);
            map.insert(key.clone(), value + 1);
        }
        if value == 3 {
            println!("No Pong received from {} after 3 Pings, closing connection", addr);
            tx.send(OutMsg::MsgClose).await.ok();
            break;
        }
    }
}

pub async fn server_handle_packet(packet: Packet, addr: &std::net::SocketAddr) -> Result<Option<Packet>, ProtocolError> {
    match packet {
        Packet::Ping => {
            println!("Received Ping from {}, sending Pong...", addr);
            Ok(Some(Packet::Pong))
        }
        Packet::Pong => {
            println!("Received Pong from {}", addr);
            // Handle Pong if needed
            LAST_RECEIVED_PING.write().await.insert(addr.to_string(), 0);
            Ok(None)
        }
        Packet::Connect(server_port) => {
            println!("Received Connect from {} with server port {}", addr, server_port);
            // Handle Connect if needed
            Ok(None)
        }
        Packet::AskForTask => {
            println!("Received AskForTask from {}", addr);
            // Handle AskForTask if needed
            Ok(None)
        }
        Packet::GiveTask(task) => {
            println!("Received GiveTask from {} with task: {:?}", addr, task);
            // Handle GiveTask if needed
            Ok(None)
        }
    }
}