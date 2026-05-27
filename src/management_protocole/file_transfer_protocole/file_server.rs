use crate::management_protocole::server::{OutMsg, ServerHandler};
use crate::management_protocole::{Packet, ProtocolError};
use std::fs::File;
use std::io::{Read, Seek};
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

pub struct FileServer;

impl FileServer {
    pub fn new() -> Self {
        FileServer
    }
}

impl Default for FileServer {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerHandler for FileServer {
    fn new_instance(&self) -> Self {
        FileServer::new()
    }

    async fn before_start(&mut self) -> Result<(), ProtocolError> {
        Ok(())
    }

    async fn on_connection_established(
        &mut self,
        _tx: Sender<OutMsg>,
        _addr: SocketAddr,
    ) -> Result<(), ProtocolError> {
        Ok(())
    }

    async fn handle_packet(
        &mut self,
        packet: Packet,
        _tx: Sender<OutMsg>,
        _addr: SocketAddr,
    ) -> Result<Option<Packet>, ProtocolError> {
        match packet {
            Packet::AskMapResultFile(offset) => {
                println!("Received AskMapResultFile with offset: {}", offset);
                let path = "CC-MAIN-20230321002050-20230321032050-00486.warc.wet"; // Example file path
                let mut file = File::open(path)?;
                println!("Opened file: {}", path);
                let mut content = vec![0u8; 15 * 1024 * 1024];
                file.seek(std::io::SeekFrom::Start(offset))?;
                let bytes_read = file.read(&mut content)?;
                let size = file.metadata()?.len();

                Ok(Some(Packet::MapResultFile {
                    end_offset: offset + bytes_read as u64,
                    file_size: size,
                    content: content[..bytes_read].to_vec(),
                }))
            }
            _ => Err(ProtocolError::UnexpectedPacket(packet)),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<OutMsg>) -> Result<(), ProtocolError> {
        Ok(())
    }
}
