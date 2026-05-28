use super::map::{run, split_single_chunk};
use rustc_hash::FxHashMap;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

static WORD_TO_TEST: &str = "the";

pub fn test_map(path: &str, number_of_tests: u32) -> std::io::Result<()> {
    println!("Starting Map WITH headers taken into account...");
    let start = Instant::now();
    for _ in 0..number_of_tests {
        run(path).unwrap();
    }
    let delay_with = start.elapsed().as_secs_f64();
    println!("Executions finished! They took {:}s to run.\n", delay_with);

    println!("Starting Map WITHOUT headers taken into account...");
    let start = Instant::now();
    for _ in 0..number_of_tests {
        let mut map: FxHashMap<String, u32> = FxHashMap::default();
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents.make_ascii_lowercase();

        split_single_chunk(&mut contents, &mut map).unwrap();

        if let Some(count) = map.get(WORD_TO_TEST) {
            println!(
                "As an example, the word '{WORD_TO_TEST}' was present {} times",
                count
            );
        }
    }
    let delay_without = start.elapsed().as_secs_f64();
    println!(
        "Executions finished! They took {:}s to run.\n",
        delay_without
    );

    println!(
        "   WITH headers  : {:}s",
        delay_with / (number_of_tests as f64)
    );
    println!(
        "WITHOUT headers  : {:}s",
        delay_without / (number_of_tests as f64)
    );

    Ok(())
}
