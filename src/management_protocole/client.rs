use crate::management_protocole::{CommandCodec, Packet, ProtocolError};
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;

pub trait ClientHandler {
    fn on_connection_established(
        &mut self,
        tx: Sender<Packet>,
    ) -> impl Future<Output = Result<(), ProtocolError>>;
    fn handle_packet(
        &mut self,
        packet: Packet,
        tx: Sender<Packet>,
    ) -> Result<Option<Packet>, ProtocolError>;
    fn on_connection_ended(
        &mut self,
        tx: Sender<Packet>,
    ) -> impl Future<Output = Result<(), ProtocolError>>;
}

pub async fn start_client(addr: &str, mut client: impl ClientHandler) -> Result<(), ProtocolError> {
    let stream: TcpStream = TcpStream::connect(addr).await?;
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

    client.on_connection_established(tx.clone()).await.ok();

    while let Some(incoming) = receiver.next().await {
        match incoming {
            Ok(packet) => {
                if let Some(response) = client.handle_packet(packet, tx.clone())? {
                    tx.send(response).await.ok();
                }
            }
            Err(ProtocolError::ClosingConnection) => {
                println!("Received a request to close the connection.");
                break;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    client.on_connection_ended(tx.clone()).await.ok();
    drop(tx);
    let _ = writer_task.await;

    Ok(())
}
