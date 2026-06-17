use crate::tasks::{DEFAULT_VERSION, MapReduceVersion, RESULT_PATH, saver::save_one_map_one_file};
use rustc_hash::FxHashMap;

pub mod default;
mod defaultwithlanguagesplit;
mod languagecount;
mod languagesize;
mod sitepagecount;
mod sitesize;
mod reverseweblink;

/// ## The Reduce task
/// Combines all the maps present in the files located at each path in the `paths` arg.
pub fn run_reduce_task_version(
    directory_path: &str,
    reduce_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut u32_map: FxHashMap<String, u32> = FxHashMap::default();
    let mut u128_map: FxHashMap<String, u128> = FxHashMap::default();
    let mut wtf_map: FxHashMap<String, Vec<String>> = FxHashMap::default();

    let mut input_size = 0;
    let output_size = match version {
        MapReduceVersion::Default => {
            input_size += default::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
        MapReduceVersion::DefaultWithLanguageSplit => {
            input_size +=
                defaultwithlanguagesplit::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
        MapReduceVersion::LanguageCount => {
            input_size += languagecount::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
        MapReduceVersion::LanguageSize => {
            languagesize::reduce_directory(directory_path, &mut u128_map).unwrap();
            save_one_map_one_file(
                &u128_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
        MapReduceVersion::SitePageCount => {
            input_size += sitepagecount::reduce_directory(directory_path, &mut u32_map).unwrap();
            save_one_map_one_file(
                &u32_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
        MapReduceVersion::SiteSize => {
            sitesize::reduce_directory(directory_path, &mut u128_map).unwrap();
            save_one_map_one_file(
                &u128_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
        MapReduceVersion::ReverseWebLink => {
            reverseweblink::reduce_directory(directory_path, &mut wtf_map).unwrap();
            save_one_map_one_file(
                &wtf_map,
                &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata"),
            )
            .unwrap()
        }
    };
    let ret: Vec<(String, f64)> = vec![
        ("input_size".to_string(), input_size as f64),
        ("output_size".to_string(), output_size),
    ];

    Ok(ret)
}

pub fn run_reduce_task(
    directory_path: &str,
    reduce_id: usize,
) -> std::io::Result<Vec<(String, f64)>> {
    run_reduce_task_version(directory_path, reduce_id, DEFAULT_VERSION)
}
