use super::MapReduceVersion;
use super::saver::save_one_map_r_files;
use crate::tasks::{DEFAULT_VERSION, MAP_DATA_PATH};
use rustc_hash::FxHashMap;
use std::time::Instant;

pub mod default;
pub mod defaultwithlanguagesplit;
pub mod languagecount;
pub use default::{map_file, map_single_chunk};

/// ## The Map task
/// Maps the file at `path` : counts the number time each word appears into the `map` arg.
pub fn run_map_task_version(
    path: &str,
    r: usize,
    map_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut u32_map: FxHashMap<String, u32> = FxHashMap::default();
    let mut u128_map: FxHashMap<String, u128> = FxHashMap::default();

    let mut ret = match version {
        MapReduceVersion::Default => map_file(path, &mut u32_map).unwrap(),
        MapReduceVersion::DefaultWithLanguageSplit => {
            defaultwithlanguagesplit::map_file(path, &mut u32_map).unwrap()
        }
        MapReduceVersion::LanguageCount => languagecount::map_file(path, &mut u32_map).unwrap(),
    };
    let start = Instant::now();
    match version {
        MapReduceVersion::Default => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::DefaultWithLanguageSplit => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::LanguageCount => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
    }
    let end = start.elapsed().as_secs_f64();
    ret.push(("saving".to_string(), end));
    Ok(ret)
}

pub fn run_map_task(path: &str, r: usize, map_id: usize) -> std::io::Result<Vec<(String, f64)>> {
    run_map_task_version(path, r, map_id, DEFAULT_VERSION)
}
