use std::{fs, path::Path};

use rustc_hash::{FxHashMap, FxHashSet};
use crate::tasks::saver::load_map;

pub fn reduce_directory(
    directory_path: &str,
    map: &mut FxHashMap<String, FxHashSet<String>>,
) -> std::io::Result<()> {
    let dir_path = Path::new(directory_path);
    if dir_path.is_dir() {
        for path in fs::read_dir(dir_path)? {
            let path: fs::DirEntry = path?;
            if let Some(file_path) = path.file_name().to_str() {
                add_file_to_map(&format!("{directory_path}{file_path}"), map);
            }
        }
    }
    Ok(())
}

fn add_file_to_map(path: &str, map: &mut FxHashMap<String, FxHashSet<String>>) {
    let (temp_map, _) = load_map::<FxHashSet<String>>(path).unwrap();
    for (key, val) in temp_map {
        if let Some(vector) = map.get_mut(&key) {
            for s in val {
                vector.insert(s);
            }
            
        } else {
            map.insert(key, val);
        }
    }
}
