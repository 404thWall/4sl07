use crate::tasks::{DEFAULT_VERSION, MapReduceVersion, RESULT_PATH, saver::save_one_map_one_file};
use rustc_hash::FxHashMap;

pub mod default;
mod defaultwithlanguagesplit;
mod languagecount;
mod languagesize;
mod sitepagecount;
mod sitesize;

/// ## The Reduce task
/// Combines all the maps present in the files located at each path in the `paths` arg.
pub fn run_reduce_task_version(
    directory_path: &str,
    reduce_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<()> {
    let mut u32_map: FxHashMap<String, u32> = FxHashMap::default();
    let mut u128_map: FxHashMap<String, u128> = FxHashMap::default();

    match version {
        MapReduceVersion::Default => {
            default::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap();
        }
        MapReduceVersion::DefaultWithLanguageSplit => {
            defaultwithlanguagesplit::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap();
        }
        MapReduceVersion::LanguageCount => {
            languagecount::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap();
        }
        MapReduceVersion::LanguageSize => {
            languagesize::reduce_directory(directory_path, &mut u128_map).unwrap();
            save_one_map_one_file(
                &u128_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap();
        }
        MapReduceVersion::SitePageCount => {
            sitepagecount::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap();
        }
        MapReduceVersion::SiteSize => {
            sitesize::reduce_directory(directory_path, &mut u128_map).unwrap();
            save_one_map_one_file(
                &u128_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap();
        }
    };

    Ok(())
}

pub fn run_reduce_task(directory_path: &str, reduce_id: usize) -> std::io::Result<()> {
    run_reduce_task_version(directory_path, reduce_id, DEFAULT_VERSION)
}
