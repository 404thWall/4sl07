use std::time::Instant;

use rustc_hash::FxHashMap;

use crate::{
    tasks::{
        DEFAULT_VERSION, MAP_DATA_PATH, MapReduceVersion,
        saver::save_one_map_r_files,
        versions::*,
    },
    versioned,
};

pub fn run_map_task(path: &str, r: usize, map_id: usize) -> std::io::Result<Vec<(String, f64)>> {
    run_map_task_version(path, r, map_id, DEFAULT_VERSION)
}

pub fn run_map_task_version(
    path: &str,
    r: usize,
    map_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<Vec<(String, f64)>> {
    versioned!(version, run_generic_map_task(path, r, map_id))
}

fn run_generic_map_task<T: TaskVersion>(
    path: &str,
    r: usize,
    map_id: usize,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut map: FxHashMap<String, T::Intermediate> = FxHashMap::default();
    let mut ret = T::map_file(path, &mut map);

    let start = Instant::now();
    let size = save_one_map_r_files(&map, r, MAP_DATA_PATH, map_id).unwrap();
    let end = start.elapsed().as_secs_f64();

    ret.push(("saving_time".to_string(), end));
    ret.push(("output_size".to_string(), size));

    Ok(ret)
}
