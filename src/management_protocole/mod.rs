// This module defines the management protocol for our key-value store.
// It was inspired by https://oneuptime.com/blog/post/2026-01-25-tcp-protocols-tokio-codec-rust/view

use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio::net::TcpListener;
use tokio_util::codec::Framed;
use std::io;
use thiserror::Error;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;
use futures::{SinkExt, StreamExt};

#[derive(Debug, Clone)]
pub enum Packet {
    Ping,
    Pong,
}

// Protocol-specific errors
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid message type: {0}")]
    InvalidMessageType(u8),
    #[error("Invalid UTF-8 in key")]
    InvalidUtf8,
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
}

const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024; // 16 MB limit

pub struct CommandCodec;

impl Decoder for CommandCodec {
    type Item = Packet;
    type Error = ProtocolError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Need at least 4 bytes for the length header
        if src.len() < 4 {
            return Ok(None);
        }

        // Read the length without consuming it yet
        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

        // Sanity check the message size
        if length > MAX_MESSAGE_SIZE {
            return Err(ProtocolError::MessageTooLarge(length));
        }

        // Check if we have the complete message
        if src.len() < 4 + length {
            // Reserve capacity for the incoming message
            src.reserve(4 + length - src.len());
            return Ok(None);
        }

        // Consume the length header
        src.advance(4);

        // Extract the message bytes
        let data = src.split_to(length);

        // Parse the command
        // parse_command(&data)

        parse_packet(&data)
    }
}

impl Encoder<Packet> for CommandCodec {
    type Error = ProtocolError;

    fn encode(&mut self, item: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let mut payload = BytesMut::new();

        match item {
            Packet::Ping => {
                payload.put_u8(0x01);  // Message type: Ping
            }
            Packet::Pong => {
                payload.put_u8(0x02);  // Message type: Pong
            }
        }

        // Write length-prefixed message
        dst.put_u32(payload.len() as u32);
        dst.put_slice(&payload);

        Ok(())
    }
}

fn parse_packet(data: &[u8]) -> Result<Option<Packet>, ProtocolError> {
    if data.is_empty() {
        return Ok(None);
    }

    let msg_type = data[0];
    let payload = &data[1..];

    match msg_type {
        0x01 => Ok(Some(Packet::Ping)),
        0x02 => Ok(Some(Packet::Pong)),
        _ => Err(ProtocolError::InvalidMessageType(msg_type)),
    }
}

async fn handle_packet(packet: Packet) -> Result<Option<Packet>, ProtocolError> {
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
    }
}

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
                    Ok(cmd) => handle_packet(cmd).await,
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

pub async fn start_client(addr: &str, ping_count: usize) -> Result<(), ProtocolError> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    println!("Connected to {}", addr);

    let mut framed = tokio_util::codec::Framed::new(stream, CommandCodec);

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
