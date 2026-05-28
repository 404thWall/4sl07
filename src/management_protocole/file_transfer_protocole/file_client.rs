use std::fs::File;
use std::io::Write;

use crate::management_protocole::client::ClientHandler;
use crate::management_protocole::{Packet, ProtocolError};
use tokio::sync::mpsc::Sender;

pub struct FileClient {
    target_file: String,
    begin_time: Option<std::time::Instant>,
    file_content: Option<Vec<u8>>,
    key: u32,
}

impl FileClient {
    pub fn new(target_file: Option<String>, key: u32) -> Self {
        FileClient {
            target_file: target_file.unwrap_or_else(|| "map_result_file.txt".to_string()),
            begin_time: None,
            file_content: None,
            key,
        }
    }
}

impl ClientHandler for FileClient {
    async fn on_connection_established(&mut self, tx: Sender<Packet>) -> Result<(), ProtocolError> {
        self.begin_time = Some(std::time::Instant::now());
        tx.send(Packet::AskMapResultFile).await.ok();
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

                if end_offset >= file_size {
                    println!("File transfer complete");
                    write_file(&self.target_file, self.file_content.as_mut().unwrap())?;
                    println!("File saved as {}", self.target_file);
                    println!(
                        "Total time taken: {:.2?}",
                        self.begin_time.unwrap().elapsed()
                    );
                    return Err(ProtocolError::ClosingConnection);
                }

                Ok(None)
            }
            _ => Err(ProtocolError::UnexpectedPacket(packet)),
        }
    }

    async fn on_connection_ended(&mut self, _tx: Sender<Packet>) -> Result<(), ProtocolError> {
        Ok(())
    }
}

fn write_file(path: &str, content: &[u8]) -> std::io::Result<()> {
    let path = std::path::Path::new(path);
    if let Some(folder) = path.parent() {
        if !folder.exists() {
            std::fs::create_dir_all(folder)?;
        }
    }
    let mut file = File::create(path)?;
    file.write_all(content)?;
    Ok(())
}
