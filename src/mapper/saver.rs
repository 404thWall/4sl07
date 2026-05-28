use rustc_hash::FxHashMap;
use std::{fs::File, io::BufWriter};



pub fn save_map(map : FxHashMap<String, u32>, save_path: &str) -> std::io::Result<()> {
    let write_file = File::create(save_path)?;
    let writer = BufWriter::new(write_file);

    //Serialize the FxHashMap directly into the file
    let e = bincode::serialize_into(writer, &map);
    if e.is_err() {
        eprintln!("Error writing : {:?}", e);
    }

    Ok(())
}