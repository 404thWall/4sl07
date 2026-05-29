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
        tx: Sender<OutMsg>,
        _addr: SocketAddr,
    ) -> Result<Option<Packet>, ProtocolError> {
        match packet {
            Packet::AskMapResultFile(key) => {
                let paths = std::fs::read_dir(crate::tasks::MAP_DATA_PATH).unwrap();
                for path in paths {
                    let path = path.unwrap().path();
                    // Assuming files are named like "data_{reduce_key}_map_{map_key}"
                    if path.is_file()
                        && path
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .starts_with(&format!("data_{}", key))
                    {
                        println!("Found file for key {}: {}", key, path.display());
                        let mut file = File::open(&path)?;
                        let mut content = vec![0u8; 15 * 1024 * 1024];
                        let mut offset = 0;
                        let size = file.metadata()?.len();

                        while offset < size {
                            file.seek(std::io::SeekFrom::Start(offset))?;
                            let bytes_read = file.read(&mut content)?;
                            tx.send(OutMsg::MsgPacket(Packet::MapResultFile {
                                end_offset: offset + bytes_read as u64,
                                file_size: size,
                                content: content[..bytes_read].to_vec(),
                            }))
                            .await
                            .ok();
                            offset += bytes_read as u64;
                        }
                    }
                }

                Ok(Some(Packet::AllFilesSent))
            }
            _ => Err(ProtocolError::UnexpectedPacket(packet)),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<OutMsg>) -> Result<(), ProtocolError> {
        Ok(())
    }
}
