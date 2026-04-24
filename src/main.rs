use std::fs::File;
use std::io::Read;
use std::time::Instant;
use rustc_hash::FxHashMap; // Faster than HashMap

static WORD_TO_TEST: &str = "the";

fn run() -> std::io::Result<()> {
    let mut map: FxHashMap<&str, u32> = FxHashMap::default();
    
    let mut file = File::open("./splits/CC-MAIN-20230321002050-20230321032050-00486.warc.wet")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents.make_ascii_lowercase();

    let words = contents.split(|c: char| {
        c == ' ' || c == '\n' || c == '\r' || c == '.' || c == ',' || 
        c == '?' || c == ':' || c == '!' || c == '(' || c == ')' || c == ';'
    });

    for word in words {
        if word.is_empty() { continue; }
        map.entry(word)
           .and_modify(|count| *count += 1)
           .or_insert(1);
    }
    
    if let Some(count) = map.get(WORD_TO_TEST) {
        println!("As an example, the word '{WORD_TO_TEST}' was present {} times", count);
    }
    Ok(())
}

fn main() {
    let start = Instant::now();
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
    }
    println!("Program finished! It took {:}s to run.", start.elapsed().as_secs_f64());
}