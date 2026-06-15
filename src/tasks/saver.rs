use core::panic;
use rustc_hash::FxHashMap;
use std::{
    fs::{self, File},
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufReader, BufWriter},
    path::Path,
};

/// ### Used to save a map created by a call to one of the run functions of the map module.
/// This function simply saves the entire map to a single binary file, provided by the `save_path` arg.
pub fn save_one_map_one_file<T>(map: &FxHashMap<String, T>, save_path: &str) -> std::io::Result<f64>
where
    T: serde::Serialize,
{
    let path = Path::new(save_path);
    let save_directory = path.parent().unwrap();
    fs::create_dir_all(save_directory)?;

    {
        let write_file = File::create(save_path)?;
        let writer = BufWriter::new(write_file);

        if let Err(e) = serde_json::to_writer_pretty(writer, &map) {
            panic!("Error writing : {:?}", e);
        }
    }

    let file_size = fs::metadata(save_path)?.len();

    Ok(file_size as f64)
}

/// ### Used to save a map created by a call to one of the run functions of the map module.
/// This function saves the entire map to R binary files, corresponding to each reduce tasks.
pub fn save_one_map_r_files<T>(
    map: &FxHashMap<String, T>,
    r: usize,
    save_directory: &str,
    map_id: usize,
) -> std::io::Result<f64>
where
    T: serde::Serialize + Clone + Copy,
{
    fs::create_dir_all(save_directory)?;
    let mut maps: Vec<FxHashMap<String, T>> = vec![FxHashMap::default(); r];

    for (key, val) in map {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let map_number: usize = (hasher.finish() as usize) % r;
        maps[map_number].insert(key.clone(), *val);
    }

    let mut ret: f64 = 0.;

    for (i, map_to_save) in maps.iter().enumerate().take(r) {
        let save_path = format!("{save_directory}data_{i}_map_{map_id}.mapdata");
        ret += save_one_map_one_file(map_to_save, &save_path).unwrap();
    }

    Ok(ret)
}

/// ### Used to load a map from memory that was saved from one of the save funcions of this module.
pub fn load_map<T>(file_path: &str) -> std::io::Result<(FxHashMap<String, T>, u64)>
where
    T: serde::de::DeserializeOwned,
{
    let read_file = File::open(file_path)?;
    let size = read_file.metadata()?.len();
    let reader = BufReader::new(read_file);

    let loaded_map = serde_json::from_reader(reader);
    if loaded_map.is_err() {
        panic!("Error loading : {:?}", file_path)
    }

    Ok((loaded_map.unwrap(), size))
}
