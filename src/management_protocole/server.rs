use crate::management_protocole::server::OutMsg::MsgClose;
use crate::management_protocole::{CommandCodec, Packet, ProtocolError};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;
use tokio_util::codec::Framed;

pub trait ServerHandler: Send {
    fn new_instance(&self) -> Self;
    fn before_start(&mut self) -> impl Future<Output = Result<(), ProtocolError>>;
    fn on_connection_established(
        &mut self,
        tx: Sender<OutMsg>,
        addr: SocketAddr,
    ) -> impl Future<Output = Result<(), ProtocolError>>;
    fn handle_packet(
        &mut self,
        packet: Packet,
        tx: Sender<OutMsg>,
        addr: SocketAddr,
    ) -> impl Future<Output = Result<Option<Packet>, ProtocolError>> + Send;
    fn on_connection_ended(
        &mut self,
        tx: Sender<OutMsg>,
    ) -> impl Future<Output = Result<(), ProtocolError>> + Send;
}

pub enum OutMsg {
    MsgPacket(Packet),
    MsgClose,
}

pub async fn start_server(
    addr: &str,
    mut server: impl ServerHandler + 'static,
) -> Result<(), ProtocolError> {
    let listener = TcpListener::bind(addr).await?;
    server.before_start().await?;

    loop {
        let mut server = server.new_instance();
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

        server
            .on_connection_established(tx.clone(), addr)
            .await
            .ok();

        let send_back_tx = tx.clone();
        tokio::spawn(async move {
            println!("New connection from {}", addr);

            while let Some(result) = receiver.next().await {
                let response = match result {
                    Ok(cmd) => match server.handle_packet(cmd, tx.clone(), addr).await {
                        Ok(res) => Ok(res),
                        Err(e) => {
                            eprintln!("Protocol error: {}", e);
                            Err(e)
                        }
                    },

                    Err(e) => {
                        eprintln!("Protocol error: {}", e);
                        Err(e)
                    }
                };

                if let Ok(Some(packet)) = response
                    && let Err(e) = send_back_tx.send(OutMsg::MsgPacket(packet)).await
                {
                    eprintln!("Failed to send response: {}", e);
                    break;
                }
            }

            send_back_tx.send(MsgClose).await.ok();
            server.on_connection_ended(tx.clone()).await.ok();
            drop(tx);
            let _ = writer_task.await;

            println!("Connection from {} closed", addr);
        });
    }
}
