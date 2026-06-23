use crate::tasks::versions::*;
use crate::tasks::{
    MAP_DATA_PATH, MapReduceVersion, run_map_task_version, run_reduce_task_version,
};
use crate::versioned;

use super::{INITIAL_DATA_PATH, RESULT_PATH};
use rand::seq::SliceRandom;
use rustc_hash::FxHashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::Instant;

pub fn test_all(
    number_of_splits: Option<usize>,
    number_of_reduces: Option<usize>,
    version: MapReduceVersion,
) -> std::io::Result<()> {
    versioned!(
        version,
        test_all_generic(number_of_splits, number_of_reduces, version)
    );
    Ok(())
}

fn assert_maps_match<I, F>(manual_map: &FxHashMap<String, I>, result_map: &FxHashMap<String, F>)
where
    I: serde::Serialize + std::fmt::Debug,
    F: serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    print!("Starting comparison of the two maps...");
    io::stdout().flush().unwrap();
    assert_eq!(
        manual_map.len(),
        result_map.len(),
        "Maps have different sizes. Manual: {}, Result: {}",
        manual_map.len(),
        result_map.len()
    );

    for (key, manual_value) in manual_map {
        // Ensure the key exists in the result map
        let result_value = result_map.get(key).unwrap_or_else(|| {
            panic!("Result map did not contain key '{key}'");
        });

        // Convert Intermediate (I) to Final (F) via JSON roundtrip
        let serialized =
            serde_json::to_string(manual_value).expect("Failed to serialize Intermediate value");
        let converted_manual_value: F = serde_json::from_str(&serialized)
            .expect("Failed to deserialize Intermediate structure into Final type");

        // Assert equality between the two F types
        assert_eq!(
            &converted_manual_value, result_value,
            "Mismatch for key '{key}': expected {manual_value:?} (converted), got {result_value:?}"
        );
    }
    println!("Done.")
}

fn test_all_generic<T: TaskVersion>(
    number_of_splits: Option<usize>,
    number_of_reduces: Option<usize>,
    version: MapReduceVersion,
) {
    let mut map: FxHashMap<String, T::Intermediate> = FxHashMap::default();
    let number_of_splits = number_of_splits.unwrap_or(2);
    let number_of_reduces = number_of_reduces.unwrap_or(5);

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
    let folder_to_delete = Path::new("/tmp/4sl07g3");
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

            T::map_file(&name, &mut map);

            print!("50%... ");
            io::stdout().flush().unwrap();
            run_map_task_version(&name, number_of_reduces, i, version).unwrap();
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
        fs::create_dir_all(format!("/tmp/4sl07g3/tests/reduce{r}/")).unwrap();
        for i in 0..number_of_splits {
            fs::copy(
                format!("/tmp/4sl07g3/map_data/data_{r}_map_{i}.mapdata"),
                format!("/tmp/4sl07g3/tests/reduce{r}/data_{r}_map_{i}.mapdata"),
            )
            .unwrap();
        }
    }
    println!("Done.");

    println!("Starting reduce tasks...");
    for r in 0..number_of_reduces {
        print!("Starting {r}th reduce task... ");
        io::stdout().flush().unwrap();
        run_reduce_task_version(&format!("/tmp/4sl07g3/tests/reduce{r}/"), r, version).unwrap();
        println!("Done.");
    }
    println!("Finished reduce tasks.");

    println!("Reforming the map from the results... ");

    let mut result_map: FxHashMap<String, T::Final> = FxHashMap::default();
    T::reduce_directory(RESULT_PATH, &mut result_map);
    assert_maps_match(&map, &result_map);
    println!("Done.");

    println!();
    println!("===============================================");
    println!("          Test finished successfully!          ");
    println!("===============================================");

    print!("Cleaning up files... ");
    io::stdout().flush().unwrap();
    let folder_to_delete = Path::new(MAP_DATA_PATH);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    let folder_to_delete = Path::new(RESULT_PATH);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    let folder_to_delete = Path::new("/tmp/4sl07g3");
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    println!("Done.");
}

/// Tests the result of the `version` implementation of MapReduce.
/// Do note that it is assumed that the result were obtained using
/// the first `map_tasks_amount` files in alphabetical order located
/// in the `initial_data_path` folder. Both folders must also end in
/// a '/'.
pub fn test_result(
    initial_data_path: &str,
    result_path: &str,
    map_tasks_amount: usize,
    version: MapReduceVersion,
) -> std::io::Result<()> {
    versioned!(
        version,
        test_result_generic(initial_data_path, result_path, map_tasks_amount)
    );
    Ok(())
}

fn test_result_generic<T: TaskVersion>(
    initial_data_path: &str,
    result_path: &str,
    map_tasks_amount: usize,
) {
    println!("Starting manual map of the {map_tasks_amount} first files in {initial_data_path}...");
    let paths = std::fs::read_dir(initial_data_path).unwrap();
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

    let mut map: FxHashMap<String, T::Intermediate> = FxHashMap::default();
    for (i, file) in candidates.iter().enumerate().take(map_tasks_amount) {
        if let Some(file_path) = file.file_name() {
            let name = format!("{}{}", initial_data_path, file_path.to_str().unwrap());
            println!("Starting {i}th map task : {name}");
            T::map_file(&name, &mut map);
        } else {
            panic!("Failed to start the {i}th map task.")
        }
    }

    println!("Finished manual map of the {map_tasks_amount} first files...");
    println!("Starting manual reduce of the result files in {result_path}...");

    let mut result_map: FxHashMap<String, T::Final> = FxHashMap::default();
    T::reduce_directory(result_path, &mut result_map);
    assert_maps_match(&map, &result_map);

    println!();
    println!("===============================================");
    println!("          Test finished successfully!          ");
    println!("===============================================");
}

pub fn run_all(map_count: usize, reduce_count: usize, version: MapReduceVersion) {
    println!("Running with: ");
    println!("  map_count    = {map_count}");
    println!("  reduce_count = {reduce_count}");
    println!("  version      = {}", version);
    print!("Deleting previous files... ");
    io::stdout().flush().unwrap();
    let folder_to_delete = Path::new(MAP_DATA_PATH);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    let truc = format!("{MAP_DATA_PATH}../tests/");
    let folder_to_delete = Path::new(&truc);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    let folder_to_delete = Path::new(RESULT_PATH);
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
    candidates.sort();
    println!("Done.");

    let mut map_input_size: f64 = 0.;
    let mut map_output_size: f64 = 0.;
    let mut map_reading_time: f64 = 0.;
    let mut map_mapping_time: f64 = 0.;
    let mut map_saving_time: f64 = 0.;

    let mut reduce_input_size: f64 = 0.;
    let mut reduce_output_size: f64 = 0.;
    let mut reduce_reducing_time: f64 = 0.;

    let start = Instant::now();

    println!("Starting the map tasks...");
    for (i, file) in candidates.iter().enumerate().take(map_count) {
        if let Some(file_path) = file.file_name() {
            let name = format!("{}{}", INITIAL_DATA_PATH, file_path.to_str().unwrap());
            print!("Map task {i}: ");
            io::stdout().flush().unwrap();
            let ret = run_map_task_version(&name, reduce_count, i, version).unwrap();
            map_reading_time += ret.first().unwrap().1;
            map_mapping_time += ret.get(1).unwrap().1;
            map_input_size += ret.get(2).unwrap().1;
            map_saving_time += ret.get(3).unwrap().1;
            map_output_size += ret.get(4).unwrap().1;
            println!(
                "{ret:?}. ETA: {:.1}s",
                (map_mapping_time + map_reading_time + map_saving_time)
                    * ((map_count - i - 1) as f64 / (i + 1) as f64)
            );
        } else {
            panic!("Failed to start map task {i}.")
        }
    }
    println!("Finished map tasks at {}s.", start.elapsed().as_secs_f64());

    println!();
    println!("Total input size:   {map_input_size}");
    println!("Total output size:  {map_output_size}");
    println!("Total reading time: {map_reading_time}");
    println!("Total mapping time: {map_mapping_time}");
    println!("Total saving time:  {map_saving_time}");
    println!();

    print!(
        "Starting copying the outputs to temporary reduces folder (to simulate the exchange)... "
    );
    io::stdout().flush().unwrap();
    for r in 0..reduce_count {
        fs::create_dir_all(format!("{MAP_DATA_PATH}../tests/reduce{r}/")).unwrap();
        for i in 0..map_count {
            fs::rename(
                format!("{MAP_DATA_PATH}data_{r}_map_{i}.mapdata"),
                format!("{MAP_DATA_PATH}../tests/reduce{r}/data_{r}_map_{i}.mapdata"),
            )
            .unwrap();
        }
    }
    println!("Done at {}s.", start.elapsed().as_secs_f64());

    println!("Starting reduce tasks...");
    for r in 0..reduce_count {
        print!("Starting reduce task {r}... ");
        io::stdout().flush().unwrap();
        let ret =
            run_reduce_task_version(&format!("{MAP_DATA_PATH}../tests/reduce{r}/"), r, version)
                .unwrap();
        reduce_input_size += ret.first().unwrap().1;
        reduce_output_size += ret.get(1).unwrap().1;
        reduce_reducing_time += ret.get(2).unwrap().1;
        println!(
            "Done: {ret:?}. Reduce ETA: {:.1}s",
            (reduce_reducing_time) * ((reduce_count - r - 1) as f64 / (r + 1) as f64)
        );
    }
    println!(
        "Finished reduce tasks at {}s.",
        start.elapsed().as_secs_f64()
    );

    println!();
    println!("Total input size:   {reduce_input_size}");
    println!("Total output size:  {reduce_output_size}");
    println!("Total reading time: {reduce_reducing_time}");
    println!();

    print!("Deleting intermediate files... ");
    io::stdout().flush().unwrap();
    let truc = format!("{MAP_DATA_PATH}../tests/");
    let folder_to_delete = Path::new(&truc);
    if folder_to_delete.exists() {
        fs::remove_dir_all(folder_to_delete).unwrap();
    }
    println!("Done at {}s.", start.elapsed().as_secs_f64());

    println!("Finished everything in {}s", start.elapsed().as_secs_f64())
}
