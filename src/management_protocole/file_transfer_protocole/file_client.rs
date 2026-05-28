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
    count: u32,
}

impl FileClient {
    pub fn new(target_file: Option<String>, key: u32) -> Self {
        FileClient {
            target_file: target_file.unwrap_or_else(|| "map_result_file".to_string()),
            begin_time: None,
            file_content: None,
            key,
            count: 0,
        }
    }
}

impl ClientHandler for FileClient {
    async fn on_connection_established(&mut self, tx: Sender<Packet>) -> Result<(), ProtocolError> {
        self.begin_time = Some(std::time::Instant::now());
        tx.send(Packet::AskMapResultFile(self.key)).await.ok();
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
                    let file_name = format!("{}_{}", self.target_file, self.count);
                    println!("Saving file to {}", file_name);
                    write_file(&file_name, self.file_content.as_mut().unwrap())?;
                    println!("File saved as {}", file_name);

                    // Prepare for next file if needed
                    self.file_content = None;
                    self.count += 1;
                }

                Ok(None)
            }
            Packet::AllFilesSent => {
                println!(
                    "Total time taken: {:.2?}",
                    self.begin_time.unwrap().elapsed()
                );
                println!("All files received, closing connection");
                Err(ProtocolError::ClosingConnection)
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
