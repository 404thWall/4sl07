use core::panic;
use rustc_hash::FxHashMap;
use std::{
    fs::{self, File},
    hash::{DefaultHasher, Hash, Hasher},
    io::{BufReader, BufWriter, Write},
    path::Path,
};

/// ### Used to save a map created by a call to one of the run functions of the map module.
/// This function simply saves the entire map to a single binary file, provided by the `save_path` arg.
pub fn save_one_map_one_file(map: &FxHashMap<String, u32>, save_path: &str) -> std::io::Result<()> {
    let path = Path::new(save_path);
    let save_directory = path.parent().unwrap();
    fs::create_dir_all(save_directory)?;

    let write_file = File::create(save_path)?;
    let mut writer = BufWriter::new(write_file);

    //Serialize the FxHashMap directly into the file
    let e = bincode::serialize_into(&mut writer, map);
    if e.is_err() {
        panic!("Error writing : {:?}", e);
    }
    writer.flush()?;

    Ok(())
}

/// ### Used to save a map created by a call to one of the run functions of the map module.
/// This function saves the entire map to R binary files, corresponding to each reduce tasks.
pub fn save_one_map_r_files(
    map: &FxHashMap<String, u32>,
    r: usize,
    save_directory: &str,
    map_id: usize,
) -> std::io::Result<()> {
    fs::create_dir_all(save_directory)?;
    let mut maps: Vec<FxHashMap<String, u32>> = vec![FxHashMap::default(); r];

    for (key, val) in map {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let map_number: usize = (hasher.finish() as usize) % r;
        maps[map_number].insert(key.clone(), *val);
    }

    for (i, map_to_save) in maps.iter().enumerate().take(r) {
        let save_path = format!("{save_directory}data_{i}_map_{map_id}.mapdata");
        save_one_map_one_file(map_to_save, &save_path).unwrap();
    }

    Ok(())
}

/// ### Used to load a map from memory that was saved from one of the save funcions of this module.
pub fn load_map(file_path: &str) -> std::io::Result<FxHashMap<String, u32>> {
    let read_file = File::open(file_path)?;
    let reader = BufReader::new(read_file);

    let loaded_map = bincode::deserialize_from(reader);
    if loaded_map.is_err() {
        panic!("Error loading : {:?}", loaded_map)
    }

    Ok(loaded_map.unwrap())
}
