use management_protocole::main_protocole::main_client::MainClient;
use rustc_hash::FxHashMap;
use slr07::management_protocole;
use slr07::management_protocole::main_protocole::main_server::MainServer;
use std::env;
use std::time::Instant;
pub mod map;
use map::run;

enum Mode {
    Server,
    Client,
    FileReader,
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
        }
    }
    let path = if args.len() < 2 {
        "/cal/commoncrawl/CC-MAIN-20230321002050-20230321032050-00486.warc.wet"
    } else if args.len() == 2 {
        &args[1]
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
    }
}
