use super::MapReduceVersion;
use super::saver::save_one_map_r_files;
use crate::tasks::MAP_DATA_PATH;
use rustc_hash::FxHashMap;
use std::time::Instant;

pub mod default;
pub mod defaultwithlanguagesplit;
pub use default::{map_file, map_single_chunk};

/// ## The Map task
/// Maps the file at `path` : counts the number time each word appears into the `map` arg.
pub fn run_map_task_version(
    path: &str,
    r: usize,
    map_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();

    let mut ret = match version {
        MapReduceVersion::Default => map_file(path, &mut map).unwrap(),
        MapReduceVersion::DefaultWithLanguageSplit => {
            defaultwithlanguagesplit::map_file(path, &mut map).unwrap()
        }
    };
    let start = Instant::now();
    save_one_map_r_files(&map, r, MAP_DATA_PATH, map_id).unwrap();
    let end = start.elapsed().as_secs_f64();
    ret.push(("saving".to_string(), end));
    Ok(ret)
}

pub fn run_map_task(path: &str, r: usize, map_id: usize) -> std::io::Result<Vec<(String, f64)>> {
    run_map_task_version(path, r, map_id, MapReduceVersion::DefaultWithLanguageSplit)
}
