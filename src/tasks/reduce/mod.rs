use crate::tasks::{DEFAULT_VERSION, MapReduceVersion, RESULT_PATH, saver::save_one_map_one_file};
use rustc_hash::FxHashMap;

pub mod default;
mod defaultwithlanguagesplit;

/// ## The Reduce task
/// Combines all the maps present in the files located at each path in the `paths` arg.
pub fn run_reduce_task_version(
    directory_path: &str,
    reduce_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<()> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();

    match version {
        MapReduceVersion::Default => default::reduce_directory(directory_path, &mut map).unwrap(),
        MapReduceVersion::DefaultWithLanguageSplit => {
            defaultwithlanguagesplit::reduce_directory(directory_path, &mut map).unwrap()
        }
    };

    save_one_map_one_file(&map, &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata")).unwrap();
    Ok(())
}

pub fn run_reduce_task(directory_path: &str, reduce_id: usize) -> std::io::Result<()> {
    run_reduce_task_version(directory_path, reduce_id, DEFAULT_VERSION)
}
