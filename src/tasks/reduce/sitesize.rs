use rustc_hash::FxHashMap;

pub fn reduce_directory(
    directory_path: &str,
    map: &mut FxHashMap<String, u128>,
) -> std::io::Result<()> {
    super::languagesize::reduce_directory(directory_path, map)
}
