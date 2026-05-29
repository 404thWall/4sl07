use management_protocole::main_protocole::main_client::MainClient;
use slr07::management_protocole;
use slr07::management_protocole::file_transfer_protocole::file_client::FileClient;
use slr07::management_protocole::file_transfer_protocole::file_server::FileServer;
use slr07::management_protocole::main_protocole::main_server::MainServer;
use slr07::tasks::{
    get_test_word_count_from_result, run_map_task_default, test_map, test_reduce, test_result,
};
use std::env;
use std::time::Instant;

enum Mode {
    Server,
    Client,
    FileReader,
    FileTransferServer,
    FileTransferClient,
    TestMap,
    TestReduce,
    TestWordCount,
    TestResult,
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
        } else if args[1] == "testreduce" {
            server = Mode::TestReduce
        } else if args[1] == "testwordcount" {
            server = Mode::TestWordCount
        } else if args[1] == "testresult" {
            server = Mode::TestResult
        }
    }
    let path = if args.len() < 2 {
        "/cal/commoncrawl/CC-MAIN-20230321002050-20230321032050-00486.warc.wet"
    } else if args.len() == 2 {
        if (args[1] == "testmap") || (args[1] == "testreduce") || (args[1] == "testwordcount") {
            "/cal/commoncrawl/CC-MAIN-20230321002050-20230321032050-00486.warc.wet"
        } else {
            &args[1]
        }
    } else if args.len() == 3
        && ((args[1] == "testmap") || (args[1] == "testreduce") || (args[1] == "testwordcount"))
    {
        &args[2]
    } else {
        ""
    };

    match server {
        Mode::Server => {
            println!("Starting in server mode...");
            if let Err(e) =
                management_protocole::server::start_server("0.0.0.0:9000", MainServer::new()).await
            {
                eprintln!("Server error: {}", e);
            }
        }
        Mode::Client => {
            println!("Starting in client mode...");
            let file_server_port = if args.len() >= 3 {
                args[2].parse::<u16>().unwrap_or(9001)
            } else {
                9001
            };
            tokio::spawn(async move {
                println!("Starting file transfer server for client...");
                if let Err(e) = management_protocole::server::start_server(
                    &format!("0.0.0.0:{}", file_server_port),
                    FileServer::new(),
                )
                .await
                {
                    eprintln!("File transfer server error: {}", e);
                }
            });
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            println!("Starting main client...");
            if let Err(e) = management_protocole::client::start_client(
                "137.194.140.198:9000",
                MainClient::new(file_server_port),
            )
            .await
            {
                eprintln!("Client error: {}", e);
            }
        }
        Mode::FileReader => {
            println!("Starting in file reader mode...");
            let start = Instant::now();
            if let Err(e) = run_map_task_default(path) {
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
                //"137.194.140.198:9001",
                "127.0.0.1:9001",
                FileClient::new(Some("./reduce_data/temp_0".to_string()), 0),
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
        Mode::TestReduce => {
            println!("Testing the Reduce Implementation...");
            if let Err(e) = test_reduce(path) {
                eprintln!("Error: {}", e);
            }
        }
        Mode::TestWordCount => {
            println!("Fetching the test word count from the result...");
            if let Err(e) = get_test_word_count_from_result(path) {
                eprintln!("Error: {}", e);
            }
        }
        Mode::TestResult => {
            println!("Testing the result from the deployement...");
            if let Err(e) = test_result() {
                eprintln!("Error: {}", e);
            }
        }
    }
}
