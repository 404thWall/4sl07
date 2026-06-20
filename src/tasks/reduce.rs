use rustc_hash::FxHashMap;

use crate::{tasks::{
    DEFAULT_VERSION, MapReduceVersion, RESULT_PATH,
    saver::save_one_map_one_file,
    versions::{
        TaskVersion, default::DefaultVersion,
        defaultwithlanguagesplit::DefaultWithLanguageSplitVersion, inoutlinks::InOutLinksVersion,
        languagecount::LanguageCountVersion, languagesize::LanguageSizeVersion,
        reverseweblink::ReverseWebLinkVersion, sitepagecount::SitePageCountVersion,
        sitesize::SiteSizeVersion,
    },
}, versioned};

pub fn run_reduce_task(
    directory_path: &str,
    reduce_id: usize,
) -> std::io::Result<Vec<(String, f64)>> {
    run_reduce_task_version(directory_path, reduce_id, DEFAULT_VERSION)
}

pub fn run_reduce_task_version(
    directory_path: &str,
    reduce_id: usize,
    version: MapReduceVersion,
) -> std::io::Result<Vec<(String, f64)>> {
    versioned!(version, run_generic_reduce_task(directory_path, reduce_id))
}

fn run_generic_reduce_task<T: TaskVersion>(
    directory_path: &str,
    reduce_id: usize,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut map: FxHashMap<String, T::Final> = FxHashMap::default();
    let input_size = T::reduce_directory(directory_path, &mut map);
    let output_size =
        save_one_map_one_file(&map, &format!("{RESULT_PATH}reduce_{reduce_id}.mapdata")).unwrap();
    let ret: Vec<(String, f64)> = vec![
        ("input_size".to_string(), input_size as f64),
        ("output_size".to_string(), output_size),
    ];
    Ok(ret)
}
