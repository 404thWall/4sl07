use super::map::{map_file, map_single_chunk};
use super::reduce::reduce_directory;
use super::saver::{save_one_map_one_file, save_one_map_r_files};
use super::{INITIAL_DATA_PATH, MAP_TASKS_AMOUNT, REDUCE_TASKS_AMOUNT, RESULT_PATH};
use rustc_hash::FxHashMap;
use std::fs::File;
use std::io::Read;
use std::time::Instant;

static WORD_TO_TEST: &str = "the";

/// Evaluates the performance of the current Map task implementation.
/// It is compared to the naive approach of neglecting the headers and just parsing the entire file.
pub fn test_map(path: &str, number_of_tests: u32) -> std::io::Result<()> {
    println!("Starting Map WITH headers taken into account...");
    let start = Instant::now();
    for _ in 0..number_of_tests {
        let mut map: FxHashMap<String, u32> = FxHashMap::default();
        map_file(path, &mut map).unwrap();
        if let Some(count) = map.get(WORD_TO_TEST) {
            println!(
                "As an example, the word '{WORD_TO_TEST}' was present {} times",
                count
            );
        }
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

        map_single_chunk(&mut contents, &mut map).unwrap();

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

/// Will run the map implementation on the file at `path`, and then attempt to reduce it.
/// A test to compare the map obtained by the Map task and the one obtained by the Reduce task will also be run.
pub fn test_reduce(path: &str) -> std::io::Result<()> {
    let mut map_from_map_task: FxHashMap<String, u32> = FxHashMap::default();
    map_file(path, &mut map_from_map_task).unwrap();
    save_one_map_r_files(&map_from_map_task, 100, "./tests/", 0).unwrap();

    let mut map_from_reduce_task: FxHashMap<String, u32> = FxHashMap::default();
    reduce_directory("./tests/", &mut map_from_reduce_task).unwrap();

    save_one_map_one_file(&map_from_reduce_task, "./tests/reducemap.mapdata").unwrap();

    for (key, value) in map_from_map_task.clone() {
        assert!(map_from_reduce_task.contains_key(&key));
        assert_eq!(value, *map_from_reduce_task.get(&key).unwrap());
    }
    for (key, value) in map_from_reduce_task {
        assert!(map_from_map_task.contains_key(&key));
        assert_eq!(value, *map_from_map_task.get(&key).unwrap());
    }

    println!("Reduce tests passed successfully!");

    Ok(())
}

pub fn get_test_word_count_from_result(result_directory_path: &str) -> std::io::Result<u32> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();
    reduce_directory(result_directory_path, &mut map).unwrap();

    if let Some(count) = map.get(WORD_TO_TEST) {
        println!(
            "The word '{WORD_TO_TEST}' was present {} times in the result!",
            count
        );
        return Ok(*count);
    }

    println!("The word '{WORD_TO_TEST}' was not present in the result...");

    Ok(0)
}

pub fn test_result() -> std::io::Result<()> {
    println!("Starting manual map of the {MAP_TASKS_AMOUNT} first files...");
    let paths = std::fs::read_dir(INITIAL_DATA_PATH).unwrap();
    let mut candidates = vec![];
    for path in paths {
        let path = path.unwrap().path();
        if path.is_file()
            && path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("CC-MAIN-")
        {
            candidates.push(path);
        }
    }
    candidates.sort();

    let mut map: FxHashMap<String, u32> = FxHashMap::default();
    for (i, file) in candidates.iter().enumerate().take(MAP_TASKS_AMOUNT) {
        if let Some(file_path) = file.file_name() {
            let name = format!("{}{}", INITIAL_DATA_PATH, file_path.to_str().unwrap());
            println!("Starting {i}th map task : {name}");
            map_file(&name, &mut map).unwrap();
        } else {
            panic!("Failed to start the {i}th map task.")
        }
    }

    println!("Finished manual map of the {MAP_TASKS_AMOUNT} first files...");
    println!("Starting manual reduce of the {REDUCE_TASKS_AMOUNT} result files...");

    let mut result_map: FxHashMap<String, u32> = FxHashMap::default();
    reduce_directory(RESULT_PATH, &mut result_map).unwrap();

    for (key, value) in map.clone() {
        println!("Testing : {key} / {value}");
        assert!(result_map.contains_key(&key));
        assert_eq!(value, *result_map.get(&key).unwrap());
    }
    for (key, value) in result_map {
        println!("Testing : {key} / {value}");
        assert!(map.contains_key(&key));
        assert_eq!(value, *map.get(&key).unwrap());
    }

    println!("Test passed successfully!");

    Ok(())
}
