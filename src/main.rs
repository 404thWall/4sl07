pub mod management_protocole;

use rustc_hash::FxHashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

use crate::Mode::{FileReader, Server}; // Faster than HashMap

static WORD_TO_TEST: &str = "the";

enum Mode {
    Server,
    Client,
    FileReader,
}

fn run() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = if args.len() < 2 {
        "/cal/commoncrawl/CC-MAIN-20230321002050-20230321032050-00486.warc.wet"
    } else if args.len() == 2 {
        &args[1]
    } else {
        panic!("Too many args.")
    };

    let mut map: FxHashMap<&str, u32> = FxHashMap::default();

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents.make_ascii_lowercase();

    let words = contents.split(|c: char| {
        c == ' '
            || c == '\n'
            || c == '\r'
            || c == '.'
            || c == ','
            || c == '?'
            || c == ':'
            || c == '!'
            || c == '('
            || c == ')'
            || c == ';'
    });

    for word in words {
        if word.is_empty() {
            continue;
        }
        map.entry(word).and_modify(|count| *count += 1).or_insert(1);
    }

    if let Some(count) = map.get(WORD_TO_TEST) {
        println!(
            "As an example, the word '{WORD_TO_TEST}' was present {} times",
            count
        );
    }
    Ok(())
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

    match server {
        Mode::Server => {
            println!("Starting in server mode...");
            if let Err(e) = management_protocole::start_server("127.0.0.1:9000").await {
                eprintln!("Server error: {}", e);
            }
        },
        Mode::Client => {
            println!("Starting in client mode...");
            if let Err(e) = management_protocole::start_client("127.0.0.1:9000", 5).await {
                eprintln!("Client error: {}", e);
            }
        },
        Mode::FileReader => {
            println!("Starting in file reader mode...");
            let start = Instant::now();
            if let Err(e) = run() {
                eprintln!("Error: {}", e);
            }
            println!(
                "Program finished! It took {:}s to run.",
                start.elapsed().as_secs_f64()
            );
        },
    }
}
