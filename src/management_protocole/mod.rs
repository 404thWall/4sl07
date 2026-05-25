// This module defines the management protocol for our key-value store.
// It was inspired by https://oneuptime.com/blog/post/2026-01-25-tcp-protocols-tokio-codec-rust/view

use bytes::{Buf, BufMut, BytesMut};
use std::io;
use thiserror::Error;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

pub mod server;
pub mod client;

#[derive(Debug, Clone)]
pub enum Packet {
    Connect(u16), // Client port
    Ping,
    Pong,
    AskForTask,
    GiveTask(Task),
}

#[derive(Debug, Clone)]
pub enum Task {
    None,
    Map(String),
    Reduce(String),
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
            Packet::Connect(port) => {
                payload.put_u8(0x03);  // Message type: Connect
                payload.put_u16(port);
            }
            Packet::AskForTask => {
                payload.put_u8(0x04);  // Message type: AskForTask
            }
            Packet::GiveTask(task) => {
                payload.put_u8(0x05);  // Message type: GiveTask
                match task {
                    Task::None => {
                        payload.put_u8(0x00);
                    }
                    Task::Map(desc) => {
                        payload.put_u8(0x01);
                        payload.put_slice(desc.as_bytes());
                    }
                    Task::Reduce(desc) => {
                        payload.put_u8(0x02);
                        payload.put_slice(desc.as_bytes());
                    }
                }
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
        0x03 => {
            if payload.len() != 2 {
                return Err(ProtocolError::InvalidMessageType(msg_type));
            }
            let port = u16::from_be_bytes([payload[0], payload[1]]);
            Ok(Some(Packet::Connect(port)))
        }
        0x04 => Ok(Some(Packet::AskForTask)),
        _ => Err(ProtocolError::InvalidMessageType(msg_type)),
    }
}
