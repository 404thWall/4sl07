use crate::management_protocole::{Packet, CommandCodec, ProtocolError};
use futures::{SinkExt, StreamExt};

pub async fn start_client(addr: &str, ping_count: usize) -> Result<(), ProtocolError> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    println!("Connected to {}", addr);

    let mut framed = tokio_util::codec::Framed::new(stream, CommandCodec);
    framed.send(Packet::Connect(25565u16)).await?;

    for i in 0..ping_count {
        println!("Sending Ping #{}", i + 1);
        framed.send(Packet::Ping).await?;

        match framed.next().await {
            Some(Ok(Packet::Pong)) => {
                println!("Received Pong #{}", i + 1);
            }
            Some(Ok(Packet::Ping)) => {
                eprintln!("Unexpected Ping from server");
            }
            Some(Ok(Packet::Connect(server_port))) => {
                println!("Received Connect with server port {}", server_port);
            }
            Some(Ok(Packet::AskForTask)) => {
                println!("Received AskForTask");
            }
            Some(Ok(Packet::GiveTask(task))) => {
                println!("Received GiveTask with task: {:?}", task);
            }
            Some(Err(e)) => {
                return Err(e);
            }
            None => {
                eprintln!("Server closed connection");
                break;
            }
        }
    }

    Ok(())
}
