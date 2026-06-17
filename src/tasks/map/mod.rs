use super::MapReduceVersion;
use super::saver::save_one_map_r_files;
use crate::tasks::{DEFAULT_VERSION, MAP_DATA_PATH};
use rustc_hash::FxHashMap;
use std::time::Instant;

pub mod default;
pub mod defaultwithlanguagesplit;
pub mod languagecount;
pub mod languagesize;
pub mod sitepagecount;
pub mod sitesize;
pub mod reverseweblink;
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
    let mut wtf_map: FxHashMap<String, Vec<String>> = FxHashMap::default();

    let mut ret = match version {
        MapReduceVersion::Default => map_file(path, &mut u32_map).unwrap(),
        MapReduceVersion::DefaultWithLanguageSplit => {
            defaultwithlanguagesplit::map_file(path, &mut u32_map).unwrap()
        }
        MapReduceVersion::LanguageCount => languagecount::map_file(path, &mut u32_map).unwrap(),
        MapReduceVersion::LanguageSize => languagesize::map_file(path, &mut u128_map).unwrap(),
        MapReduceVersion::SitePageCount => sitepagecount::map_file(path, &mut u32_map).unwrap(),
        MapReduceVersion::SiteSize => sitesize::map_file(path, &mut u128_map).unwrap(),
        MapReduceVersion::ReverseWebLink => reverseweblink::map_file(path, &mut wtf_map).unwrap(),
    };
    let start = Instant::now();
    let size = match version {
        MapReduceVersion::Default => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::DefaultWithLanguageSplit => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::LanguageCount => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::LanguageSize => {
            save_one_map_r_files(&u128_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::SitePageCount => {
            save_one_map_r_files(&u32_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::SiteSize => {
            save_one_map_r_files(&u128_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
        MapReduceVersion::ReverseWebLink => {
            save_one_map_r_files(&wtf_map, r, MAP_DATA_PATH, map_id).unwrap()
        }
    };
    let end = start.elapsed().as_secs_f64();
    ret.push(("saving_time".to_string(), end));
    ret.push(("output_size".to_string(), size));
    Ok(ret)
}

pub fn run_map_task(path: &str, r: usize, map_id: usize) -> std::io::Result<Vec<(String, f64)>> {
    run_map_task_version(path, r, map_id, DEFAULT_VERSION)
}
