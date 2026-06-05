use rustc_hash::FxHashMap;
use std::fs;
use std::io::{BufRead, Cursor, Read};
use std::time::Instant;

pub fn map_file(
    path: &str,
    map: &mut FxHashMap<String, u32>,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut ret: Vec<(String, f64)> = vec![];
    Ok(ret)
}

pub fn map_single_chunk(
    contents: &mut str,
    map: &mut FxHashMap<String, u32>,
) -> std::io::Result<()> {

    Ok(())
}
