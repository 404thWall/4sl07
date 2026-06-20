use clap::{Parser, Subcommand};
use management_protocole::main_protocole::main_client::MainClient;
use slr07::management_protocole;
use slr07::management_protocole::file_transfer_protocole::file_client::FileClient;
use slr07::management_protocole::file_transfer_protocole::file_server::FileServer;
use slr07::management_protocole::main_protocole::main_server::MainServer;
use slr07::tasks::{
    MAP_TASKS_AMOUNT, MapReduceVersion, REDUCE_TASKS_AMOUNT, run_map_task_version,
    run_reduce_task_version, test_all,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Server,
    Client {
        #[arg(default_value_t = 9001)]
        file_server_port: u16,
        #[arg(default_value = "127.0.0.1")]
        main_host_address: String,
        user: String,
    },
    FileTransferServer,
    FileTransferClient,
    Map {
        /// Path to the _file_ to map.
        path: String,
        #[arg(short, long, default_value_t = REDUCE_TASKS_AMOUNT)]
        /// Indicates the number of reduce tasks that will be run.
        /// Allows the map task to create `reduce_number` files
        /// containing the relevant keys for each reduce task.
        reduce_number: usize,
        /// The id of the map task.
        map_id: usize,
        #[arg(short, long, value_enum, default_value_t = MapReduceVersion::DefaultWithLanguageSplit)]
        version: MapReduceVersion,
    },
    Reduce {
        /// Path to the _directory_ to reduce. Must end in a '/'.
        path: String,
        /// The id of the reduce task.
        reduce_id: usize,
        #[arg(short, long, value_enum, default_value_t = MapReduceVersion::DefaultWithLanguageSplit)]
        version: MapReduceVersion,
    },
    /// Chooses `map_count` random splits from the default directory
    /// and maps them into `reduce_count` files. Theses mapped files
    /// are then reduced, and a test to check the integrity of the result
    /// files is then ran.
    TestAll {
        #[arg(short, long, default_value_t = MAP_TASKS_AMOUNT)]
        map_count: usize,
        #[arg(short, long, default_value_t = REDUCE_TASKS_AMOUNT)]
        reduce_count: usize,
        #[arg(short, long, value_enum, default_value_t = MapReduceVersion::DefaultWithLanguageSplit)]
        version: MapReduceVersion,
    },
    TestDownload,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.cmd {
        Commands::Server => {
            println!("Starting in server mode...");
            println!("Cleaning up temporary files before starting the server...");
            management_protocole::main_protocole::clean_temporary_files();
            println!("Starting main server on 0.0.0.0:9000...");
            if let Err(e) =
                management_protocole::server::start_server("0.0.0.0:9000", MainServer::new()).await
            {
                eprintln!("Server error: {}", e);
            }
        }
        Commands::Client {
            file_server_port,
            main_host_address,
            user,
        } => {
            println!("Starting in client mode...");
            println!("Cleaning up temporary files before starting the client...");
            management_protocole::main_protocole::clean_temporary_files();
            println!("Starting file transfer server for client...");
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
            let copied_address = main_host_address.clone();
            if let Err(e) = management_protocole::client::start_client(
                &format!("{}:9000", main_host_address),
                MainClient::new(file_server_port, user, copied_address),
            )
            .await
            {
                eprintln!("Client error: {}", e);
            }
        }
        Commands::FileTransferClient => {
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
        Commands::FileTransferServer => {
            println!("Starting in file transfer server mode..");
            if let Err(e) =
                management_protocole::server::start_server("0.0.0.0:9001", FileServer::new()).await
            {
                eprintln!("File transfer server error: {}", e);
            }
        }
        Commands::Map {
            path,
            reduce_number,
            map_id,
            version,
        } => {
            println!("Running the Map Task...");
            match run_map_task_version(&path, reduce_number, map_id, version) {
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
                Ok(v) => {
                    println!("Returned {v:#?}")
                }
            }
        }
        Commands::Reduce {
            path,
            reduce_id,
            version,
        } => {
            println!("Running the Reduce Task...");
            match run_reduce_task_version(&path, reduce_id, version) {
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
                Ok(v) => {
                    println!("Returned {v:#?}")
                }
            }
        }
        Commands::TestAll {
            map_count,
            reduce_count,
            version,
        } => {
            if let Err(e) = test_all(Some(map_count), Some(reduce_count), version) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::TestDownload => {
            println!("Testing the download of the commoncrawl files...");
            if let Err(e) = management_protocole::main_protocole::downloader::test_download().await
            {
                eprintln!("Error: {:?}", e);
            }
        }
    }
}
