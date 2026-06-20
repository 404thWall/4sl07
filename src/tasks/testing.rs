use crate::tasks::versions::TaskVersion;
use crate::tasks::versions::default::DefaultVersion;
use crate::tasks::versions::defaultwithlanguagesplit::DefaultWithLanguageSplitVersion;
use crate::tasks::versions::languagecount::LanguageCountVersion;
use crate::tasks::versions::languagesize::LanguageSizeVersion;
use crate::tasks::versions::reverseweblink::ReverseWebLinkVersion;
use crate::tasks::versions::sitepagecount::SitePageCountVersion;
use crate::tasks::versions::sitesize::SiteSizeVersion;
use crate::tasks::{
    MAP_DATA_PATH, MapReduceVersion, run_map_task_version, run_reduce_task_version,
};

use super::{INITIAL_DATA_PATH, RESULT_PATH};
use rand::seq::SliceRandom;
use rustc_hash::FxHashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn test_all(
    number_of_splits: Option<usize>,
    number_of_reduces: Option<usize>,
    version: MapReduceVersion,
) -> std::io::Result<()> {
    match version {
        MapReduceVersion::Default => {
            test_all_generic::<DefaultVersion>(number_of_splits, number_of_reduces, version)
        }
        MapReduceVersion::DefaultWithLanguageSplit => {
            test_all_generic::<DefaultWithLanguageSplitVersion>(
                number_of_splits,
                number_of_reduces,
                version,
            )
        }
        MapReduceVersion::LanguageCount => {
            test_all_generic::<LanguageCountVersion>(number_of_splits, number_of_reduces, version)
        }
        MapReduceVersion::LanguageSize => {
            test_all_generic::<LanguageSizeVersion>(number_of_splits, number_of_reduces, version)
        }
        MapReduceVersion::SitePageCount => {
            test_all_generic::<SitePageCountVersion>(number_of_splits, number_of_reduces, version)
        }
        MapReduceVersion::SiteSize => {
            test_all_generic::<SiteSizeVersion>(number_of_splits, number_of_reduces, version)
        }
        MapReduceVersion::ReverseWebLink => {
            test_all_generic::<ReverseWebLinkVersion>(number_of_splits, number_of_reduces, version)
        }
    }
    Ok(())
}

fn assert_maps_match<I, F>(manual_map: &FxHashMap<String, I>, result_map: &FxHashMap<String, F>)
where
    I: serde::Serialize + std::fmt::Debug,
    F: serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
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
    let folder_to_delete = Path::new("/tmp/4sl07_grp3");
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
        run_reduce_task_version(&format!("/tmp/4sl07_grp3/tests/reduce{r}/"), r, version).unwrap();
        println!("Done.");
    }
    println!("Finished reduce tasks.");

    print!("Reforming the map from the results and starting comparison... ");
    io::stdout().flush().unwrap();

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
    let folder_to_delete = Path::new("/tmp/4sl07_grp3");
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
    match version {
        MapReduceVersion::Default => {
            test_result_generic::<DefaultVersion>(initial_data_path, result_path, map_tasks_amount)
        }
        MapReduceVersion::DefaultWithLanguageSplit => {
            test_result_generic::<DefaultWithLanguageSplitVersion>(
                initial_data_path,
                result_path,
                map_tasks_amount,
            )
        }
        MapReduceVersion::LanguageCount => test_result_generic::<LanguageCountVersion>(
            initial_data_path,
            result_path,
            map_tasks_amount,
        ),
        MapReduceVersion::LanguageSize => test_result_generic::<LanguageSizeVersion>(
            initial_data_path,
            result_path,
            map_tasks_amount,
        ),
        MapReduceVersion::SitePageCount => test_result_generic::<SitePageCountVersion>(
            initial_data_path,
            result_path,
            map_tasks_amount,
        ),
        MapReduceVersion::SiteSize => {
            test_result_generic::<SiteSizeVersion>(initial_data_path, result_path, map_tasks_amount)
        }
        MapReduceVersion::ReverseWebLink => test_result_generic::<ReverseWebLinkVersion>(
            initial_data_path,
            result_path,
            map_tasks_amount,
        ),
    }
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
