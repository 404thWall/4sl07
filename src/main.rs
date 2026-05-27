use management_protocole::main_protocole::main_client::MainClient;
use slr07::management_protocole;
use slr07::management_protocole::file_transfer_protocole::file_client::FileClient;
use slr07::management_protocole::file_transfer_protocole::file_server::FileServer;
use slr07::management_protocole::main_protocole::main_server::MainServer;
use std::env;
use std::time::Instant;
pub mod map;
use map::run;

use crate::map::test_map;

enum Mode {
    Server,
    Client,
    FileReader,
    FileTransferServer,
    FileTransferClient,
    TestMap,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut server = Mode::FileReader;

    if args.len() >= 2 {
        if args[1] == "server" {
            server = Mode::Server;
        } else if args[1] == "client" {
            server = Mode::Client;
        } else if args[1] == "file_transfer_server" {
            server = Mode::FileTransferServer;
        } else if args[1] == "file_transfer_client" {
            server = Mode::FileTransferClient;
        } else if args[1] == "testmap" {
            server = Mode::TestMap;
        }
    }
    let path = if args.len() < 2 {
        "/cal/commoncrawl/CC-MAIN-20230321002050-20230321032050-00486.warc.wet"
    } else if args.len() == 2 {
        if args[1] == "testmap" {
            "/cal/commoncrawl/CC-MAIN-20230321002050-20230321032050-00486.warc.wet"
        } else {
            &args[1]
        }
    } else if args.len() == 3 && args[1] == "testmap" {
        &args[2]
    } else {
        panic!("Too many args.")
    };

    match server {
        Mode::Server => {
            println!("Starting in server mode...");
            if let Err(e) =
                management_protocole::server::start_server("127.0.0.1:9000", MainServer::new())
                    .await
            {
                eprintln!("Server error: {}", e);
            }
        }
        Mode::Client => {
            println!("Starting in client mode...");
            if let Err(e) =
                management_protocole::client::start_client("127.0.0.1:9000", MainClient::new())
                    .await
            {
                eprintln!("Client error: {}", e);
            }
        }
        Mode::FileReader => {
            println!("Starting in file reader mode...");
            let start = Instant::now();
            if let Err(e) = run(path) {
                eprintln!("Error: {}", e);
            }
            println!(
                "Program finished! It took {:}s to run.",
                start.elapsed().as_secs_f64()
            );
        }
        Mode::FileTransferClient => {
            println!("Starting in file transfer client mode...");
            if let Err(e) = management_protocole::client::start_client(
                "137.194.140.198:9001",
                FileClient::new(),
            )
            .await
            {
                eprintln!("File transfer client error: {}", e);
            }
        }
        Mode::FileTransferServer => {
            println!("Starting in file transfer server mode..");
            if let Err(e) =
                management_protocole::server::start_server("0.0.0.0:9001", FileServer::new()).await
            {
                eprintln!("File transfer server error: {}", e);
            }
        }
        Mode::TestMap => {
            println!("Testing the Map Implementation...");
            if let Err(e) = test_map(path, 20) {
                eprintln!("Error: {}", e);
            }
        }
    }
}
