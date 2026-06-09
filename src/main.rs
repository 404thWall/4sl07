use clap::{Parser, Subcommand};
use management_protocole::main_protocole::main_client::MainClient;
use slr07::management_protocole;
use slr07::management_protocole::file_transfer_protocole::file_client::FileClient;
use slr07::management_protocole::file_transfer_protocole::file_server::FileServer;
use slr07::management_protocole::main_protocole::main_server::MainServer;
use slr07::tasks::{
    MAP_TASKS_AMOUNT, MapReduceVersion, REDUCE_TASKS_AMOUNT, get_test_word_count_from_result,
    run_map_task_version, run_reduce_task_version, test_all, test_map, test_reduce, test_result,
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
    TestMap {
        /// Path to the _file_ to evaluate the map performance on.
        path: String,
        #[arg(short, long, default_value_t = 20)]
        /// Number of times both implementations will be ran. Used to
        /// reduce randomness at the cost of computing time.
        number_of_tests: u32,
    },
    TestReduce {
        /// Path to the _file_ to map. After the map is complete,
        /// the mapped files will be reduced, and the result will
        /// be tested.
        path: String,
    },
    TestResult,
    /// Chooses `number_of_maps` random splits from the default directory
    /// and maps them into `number_of_reduces` files. Theses mapped files
    /// are then reduced, and a test to check the integrity of the result
    /// files is then ran.
    TestAll {
        #[arg(short, long, default_value_t = MAP_TASKS_AMOUNT)]
        number_of_maps: usize,
        #[arg(short, long, default_value_t = REDUCE_TASKS_AMOUNT)]
        number_of_reduces: usize,
        #[arg(short, long, value_enum, default_value_t = MapReduceVersion::DefaultWithLanguageSplit)]
        version: MapReduceVersion,
    },
    GetWordCount {
        /// Path to the result _directory_ from which we want to get the
        /// word count of the word `word`.
        path: String,
        #[arg(short, long, default_value = "the")]
        /// Word that we want to have the number of in the results.
        /// Should be in lowercase.
        word: String,
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
        Commands::TestMap {
            path,
            number_of_tests,
        } => {
            println!("Testing the Map Implementation...");
            if let Err(e) = test_map(&path, number_of_tests) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::TestReduce { path } => {
            println!("Testing the Reduce Implementation...");
            if let Err(e) = test_reduce(&path) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::GetWordCount { path, word } => {
            println!("Fetching the test word count from the result...");
            if let Err(e) = get_test_word_count_from_result(&path, &word) {
                eprintln!("Error: {}", e);
            }
        }
        Commands::TestResult => {
            println!("Testing the result from the deployement...");
            if let Err(e) = test_result() {
                eprintln!("Error: {}", e);
            }
        }
        Commands::TestAll {
            number_of_maps,
            number_of_reduces,
            version
        } => {
            if let Err(e) = test_all(Some(number_of_maps), Some(number_of_reduces), version) {
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
