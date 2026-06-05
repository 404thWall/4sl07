use crate::tasks::{MAP_DATA_PATH, run_map_task, run_reduce_task};

use super::map::{map_file, map_single_chunk};
use super::reduce::default::reduce_directory;
use super::saver::{save_one_map_one_file, save_one_map_r_files};
use super::{INITIAL_DATA_PATH, MAP_TASKS_AMOUNT, REDUCE_TASKS_AMOUNT, RESULT_PATH};
use rand::seq::SliceRandom;
use rustc_hash::FxHashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::Instant;

const WORD_TO_TEST: &str = "the";

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

pub fn get_test_word_count_from_result(
    result_directory_path: &str,
    word_to_test: &str,
) -> std::io::Result<u32> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();
    reduce_directory(result_directory_path, &mut map).unwrap();

    if let Some(count) = map.get(word_to_test) {
        println!(
            "The word '{word_to_test}' was present {} times in the result!",
            count
        );
        return Ok(*count);
    }

    println!("The word '{word_to_test}' was not present in the result...");

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
    println!(
        "Starting manual reduce of the {REDUCE_TASKS_AMOUNT} result files in {RESULT_PATH}..."
    );

    let mut result_map: FxHashMap<String, u32> = FxHashMap::default();
    reduce_directory(RESULT_PATH, &mut result_map).unwrap();

    if let Some(count) = map.get(WORD_TO_TEST) {
        println!(
            "The word '{WORD_TO_TEST}' was present {} times in the manual map!",
            count
        );
    } else {
        println!("The word '{WORD_TO_TEST}' was not present in the manual map...")
    }

    if let Some(count) = result_map.get(WORD_TO_TEST) {
        println!(
            "The word '{WORD_TO_TEST}' was present {} times in the result map!",
            count
        );
    } else {
        println!("The word '{WORD_TO_TEST}' was not present in the result map...")
    }

    println!("There are {} keys in the manual map", map.keys().len());
    println!(
        "There are {} keys in the result map",
        result_map.keys().len()
    );

    print!("Starting comparison of result map and manual map... ");
    io::stdout().flush().unwrap();
    for (key, value) in map.clone() {
        assert!(
            result_map.contains_key(&key),
            "Result map did not contain key '{key}'"
        );
        assert_eq!(
            value,
            *result_map.get(&key).unwrap(),
            "Result map had the wrong number of '{key}' : {} instead of {value}",
            *result_map.get(&key).unwrap()
        );
    }
    print!("50%... ");
    io::stdout().flush().unwrap();
    for (key, value) in result_map {
        assert!(
            map.contains_key(&key),
            "Manual map did not contain key '{key}'"
        );
        assert_eq!(
            value,
            *map.get(&key).unwrap(),
            "Manual map had the wrong number of '{key}' : {} instead of {value}",
            *map.get(&key).unwrap()
        );
    }
    println!("Done.");

    println!("Test passed successfully!");

    Ok(())
}

pub fn test_all(
    number_of_splits: Option<usize>,
    number_of_reduces: Option<usize>,
) -> std::io::Result<()> {
    let number_of_splits = number_of_splits.unwrap_or(5);
    let number_of_reduces = number_of_reduces.unwrap_or(REDUCE_TASKS_AMOUNT);
    let mut manual_map: FxHashMap<String, u32> = FxHashMap::default();

    print!("Deleting previous files... ");
    io::stdout().flush().unwrap();
    let folder_to_delete = Path::new(MAP_DATA_PATH);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    let folder_to_delete = Path::new(RESULT_PATH);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    let folder_to_delete = Path::new("/tmp/4sl07_grp3/tests/");
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    println!("Done.");

    print!("Fetching the list of files... ");
    io::stdout().flush().unwrap();
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
    println!("Done.");

    print!("Selecting {number_of_splits} random splits to test...");
    io::stdout().flush().unwrap();
    let mut rng = rand::rng();
    candidates.shuffle(&mut rng);
    println!("Done.");

    println!("Starting the map tasks (as well as a manual map made from all files)...");
    for (i, file) in candidates.iter().enumerate().take(number_of_splits) {
        if let Some(file_path) = file.file_name() {
            let name = format!("{}{}", INITIAL_DATA_PATH, file_path.to_str().unwrap());
            print!("Starting map task {i} : {name}... ");
            io::stdout().flush().unwrap();
            map_file(&name, &mut manual_map).unwrap();
            print!("50%... ");
            io::stdout().flush().unwrap();
            run_map_task(&name, number_of_reduces, i).unwrap();
            println!("Done.");
        } else {
            panic!("Failed to start the {i}th map task.")
        }
    }
    println!("Finished map tasks.");

    print!(
        "Starting copying the outputs to temporary reduces folder (to simulate the exchange)... "
    );
    io::stdout().flush().unwrap();
    for r in 0..number_of_reduces {
        fs::create_dir_all(format!("/tmp/4sl07_grp3/tests/reduce{r}/")).unwrap();
        for i in 0..number_of_splits {
            fs::copy(
                format!("/tmp/4sl07_grp3/map_data/data_{r}_map_{i}.mapdata"),
                format!("/tmp/4sl07_grp3/tests/reduce{r}/data_{r}_map_{i}.mapdata"),
            )
            .unwrap();
        }
    }
    println!("Done.");

    println!("Starting reduce tasks...");
    for r in 0..number_of_reduces {
        print!("Starting {r}th reduce task... ");
        io::stdout().flush().unwrap();
        run_reduce_task(&format!("/tmp/4sl07_grp3/tests/reduce{r}/"), r).unwrap();
        println!("Done.");
    }
    println!("Finished reduce tasks.");

    print!("Reforming the map from the results... ");
    io::stdout().flush().unwrap();
    let mut result_map: FxHashMap<String, u32> = FxHashMap::default();
    reduce_directory(RESULT_PATH, &mut result_map).unwrap();
    println!("Done.");

    print!("Starting comparison of result map and manual map... ");
    io::stdout().flush().unwrap();
    for (key, value) in manual_map.clone() {
        assert!(
            result_map.contains_key(&key),
            "Result map did not contain key '{key}'"
        );
        assert_eq!(
            value,
            *result_map.get(&key).unwrap(),
            "Result map had the wrong number of '{key}' : {} instead of {value}",
            *result_map.get(&key).unwrap()
        );
    }
    print!("50%... ");
    io::stdout().flush().unwrap();
    for (key, value) in result_map {
        assert!(
            manual_map.contains_key(&key),
            "Manual map did not contain key '{key}'"
        );
        assert_eq!(
            value,
            *manual_map.get(&key).unwrap(),
            "Manual map had the wrong number of '{key}' : {} instead of {value}",
            *manual_map.get(&key).unwrap()
        );
    }
    println!("Done.");

    println!();
    println!("===============================================");
    println!("          Test finished successfully!          ");
    println!("===============================================");

    Ok(())
}
