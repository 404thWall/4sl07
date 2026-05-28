// This module defines the management protocol for our key-value store.
// It was inspired by https://oneuptime.com/blog/post/2026-01-25-tcp-protocols-tokio-codec-rust/view

use bytes::{Buf, BufMut, BytesMut};
use std::io;
use thiserror::Error;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

pub mod client;
pub mod file_transfer_protocole;
pub mod main_protocole;
pub mod server;

#[derive(Debug, Clone)]
pub enum Packet {
    Connect(u16), // Client port
    Ping,
    Pong,
    AskForTask,
    GiveTask(Task),
    TaskFinished{
        task: Task,
        reduce_files: Vec<u32>, // List of keys for which this task produced a file
    },
    AskMapResultFile,
    MapResultFile {
        end_offset: u64,  // The offset in the file after the content sent in this packet
        file_size: u64,   // Total size of the file in bytes
        content: Vec<u8>, // Contains the content of the file as bytes
    },
    ConnectedWorkersList(Vec<(String, u16)>), // List of (IP, port) of connected workers
    AskWorkersList,
}

#[derive(Debug, Clone)]
pub enum Task {
    None,
    Finished,
    Map(u32, u32),    // Contains the key and the number of keys for this task
    Reduce(u32, u32), // Contains the key and the number of keys for this task
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
    #[error("Unexpected packet: Should not receive {0:?} in this context")]
    UnexpectedPacket(Packet),
    #[error("Unexpected packet format: {0}")]
    UnexpectedPacketFormat(String),
    #[error("Closing connection")]
    ClosingConnection,
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
                payload.put_u8(0x01); // Message type: Ping
            }
            Packet::Pong => {
                payload.put_u8(0x02); // Message type: Pong
            }
            Packet::Connect(port) => {
                payload.put_u8(0x03); // Message type: Connect
                payload.put_u16(port);
            }
            Packet::AskForTask => {
                payload.put_u8(0x04); // Message type: AskForTask
            }
            Packet::GiveTask(task) => {
                payload.put_u8(0x05); // Message type: GiveTask
                encode_task(&task, &mut payload);
            }
            Packet::TaskFinished { task, reduce_files: resulting_files } => {
                payload.put_u8(0x06); // Message type: TaskFinished
                encode_task(&task, &mut payload);
                payload.put_u32(resulting_files.len() as u32);
                for key in resulting_files {
                    payload.put_u32(key);
                }
            }
            Packet::AskMapResultFile => {
                payload.put_u8(0x07); // Message type: AskMapResultFile
            }
            Packet::MapResultFile {
                end_offset,
                file_size,
                content,
            } => {
                payload.put_u8(0x08); // Message type: MapResultFile
                payload.put_u64(end_offset);
                payload.put_u64(file_size);
                payload.put_slice(&content);
            }
            Packet::ConnectedWorkersList(list) => {
                payload.put_u8(0x09); // Message type: SendConnectedWorkers
                payload.put_u32(list.len() as u32);
                for (addr, port) in list {
                    let addr_bytes = addr.as_bytes();
                    if addr_bytes.len() > 255 {
                        return Err(ProtocolError::InvalidUtf8);
                    }
                    payload.put_u8(addr_bytes.len() as u8);
                    payload.put_slice(addr_bytes);
                    payload.put_u16(port);
                }
            }
            Packet::AskWorkersList => {
                payload.put_u8(0x0A); // Message type: AskWorkersList
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
        0x05 => {
            let task = decode_task(payload)?;
            Ok(Some(Packet::GiveTask(task)))
        }
        0x06 => {
            let task = decode_task(payload)?;
            let size = u32::from_be_bytes([payload[9], payload[10], payload[11], payload[12]]) as usize;
            if payload.len() < 13 + size * 4 {
                return Err(ProtocolError::InvalidMessageType(msg_type));
            }
            let mut resulting_files = Vec::new();
            for i in 0..size {
                let key = u32::from_be_bytes([
                    payload[13 + i * 4],
                    payload[14 + i * 4],
                    payload[15 + i * 4],
                    payload[16 + i * 4],
                ]);
                resulting_files.push(key);
            }
            Ok(Some(Packet::TaskFinished { task, reduce_files: resulting_files }))
        }
        0x07 => Ok(Some(Packet::AskMapResultFile)),
        0x08 => {
            if payload.len() < 16 {
                return Err(ProtocolError::UnexpectedPacketFormat(
                    "Payload too short for MapResultFile".to_string(),
                ));
            }

            let end_offset = u64::from_be_bytes([
                payload[0], payload[1], payload[2], payload[3], payload[4], payload[5], payload[6],
                payload[7],
            ]);

            let file_size = u64::from_be_bytes([
                payload[8],
                payload[9],
                payload[10],
                payload[11],
                payload[12],
                payload[13],
                payload[14],
                payload[15],
            ]);

            let content = payload[16..].to_vec();

            Ok(Some(Packet::MapResultFile {
                end_offset,
                file_size,
                content,
            }))
        }
        0x09 => {
            if payload.len() < 4 {
                return Err(ProtocolError::UnexpectedPacketFormat(
                    "Payload too short for SendConnectedWorkers".to_string(),
                ));
            }
            let size = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
            let mut list: Vec<(String, u16)> = Vec::new();
            let mut offset = 4;
            for _ in 0..size {
                let string_size = u8::from_be_bytes([payload[offset]]) as usize;
                let addr = str::from_utf8(&payload[offset+1..offset+1+string_size]).unwrap();
                let port = u16::from_be_bytes([payload[offset + string_size + 1], payload[offset + string_size + 2]]);
                list.push((addr.to_string(), port));
                offset += string_size + 3;
            }
            Ok(Some(Packet::ConnectedWorkersList(list)))
        }
        0x0A => Ok(Some(Packet::AskWorkersList)),
        _ => Err(ProtocolError::InvalidMessageType(msg_type)),
    }
}

fn encode_task(task: &Task, payload: &mut BytesMut) {
    match task {
        Task::None => {
            payload.put_u8(0x00);
            payload.put_u32(0);
            payload.put_u32(0);
        }
        Task::Map(key, nkeys) => {
            payload.put_u8(0x01);
            payload.put_u32(*key);
            payload.put_u32(*nkeys);
        }
        Task::Reduce(key, nkeys) => {
            payload.put_u8(0x02);
            payload.put_u32(*key);
            payload.put_u32(*nkeys);
        }
        Task::Finished => {
            payload.put_u8(0x03);
            payload.put_u32(0);
            payload.put_u32(0);
        }
    }
}

fn decode_task(payload: &[u8]) -> Result<Task, ProtocolError> {
    if payload.is_empty() {
        return Err(ProtocolError::UnexpectedPacketFormat(
            "Empty payload".to_string(),
        ));
    }
    let task_type = payload[0];
    if payload.len() < 9 {
        return Err(ProtocolError::UnexpectedPacketFormat(
            "Payload too short".to_string(),
        ));
    }
    let key = u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]);
    let nkeys = u32::from_be_bytes([payload[5], payload[6], payload[7], payload[8]]);
    match task_type {
        0x00 => Ok(Task::None),
        0x01 => Ok(Task::Map(key, nkeys)),
        0x02 => Ok(Task::Reduce(key, nkeys)),
        0x03 => Ok(Task::Finished),
        _ => Err(ProtocolError::UnexpectedPacketFormat(format!(
            "Invalid task type: {}",
            task_type
        ))),
    }
}
