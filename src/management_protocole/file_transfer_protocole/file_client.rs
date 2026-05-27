use std::fs::File;
use std::io::Write;

use crate::management_protocole::client::ClientHandler;
use crate::management_protocole::{Packet, ProtocolError};
use tokio::sync::mpsc::Sender;

pub struct FileClient {
    begin_time: Option<std::time::Instant>,
    file_content: Option<Vec<u8>>,
}

impl FileClient {
    pub fn new() -> Self {
        FileClient {
            begin_time: None,
            file_content: None,
        }
    }
}

impl Default for FileClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientHandler for FileClient {
    async fn on_connection_established(&mut self, tx: Sender<Packet>) -> Result<(), ProtocolError> {
        self.begin_time = Some(std::time::Instant::now());
        tx.send(Packet::AskMapResultFile(0)).await.ok();
        Ok(())
    }

    fn handle_packet(
        &mut self,
        packet: Packet,
        _tx: Sender<Packet>,
    ) -> Result<Option<Packet>, ProtocolError> {
        match packet {
            Packet::MapResultFile {
                end_offset,
                file_size,
                content,
            } => {
                println!(
                    "Received MapResultFile: end_offset={}, file_size={}, content_length={}",
                    end_offset,
                    file_size,
                    content.len()
                );
                if let Some(vec) = self.file_content.as_mut() {
                    vec.extend_from_slice(&content);
                } else {
                    let mut vec = content.to_vec();
                    vec.reserve(file_size as usize);
                    self.file_content = Some(vec);
                }

                if end_offset < file_size {
                    Ok(Some(Packet::AskMapResultFile(end_offset)))
                } else {
                    println!("File transfer complete");
                    write_file(
                        "received_map_result_file.txt",
                        &self.file_content.as_mut().unwrap(),
                    )?;
                    println!("File saved as received_map_result_file.txt");
                    println!(
                        "Total time taken: {:.2?}",
                        self.begin_time.unwrap().elapsed()
                    );
                    Ok(None)
                }
            }
            _ => Err(ProtocolError::UnexpectedPacket(packet)),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<Packet>) -> Result<(), ProtocolError> {
        Ok(())
    }
}

fn write_file(path: &str, content: &[u8]) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    file.write_all(content)?;
    Ok(())
}
