use rustc_hash::FxHashMap;

pub fn reduce_directory(
    directory_path: &str,
    map: &mut FxHashMap<String, u32>,
) -> std::io::Result<()> {
    super::default::reduce_directory(directory_path, map)
}
