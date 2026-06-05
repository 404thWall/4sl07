use crate::tasks::{MAP_DATA_PATH, REDUCE_TASKS_AMOUNT};
use rustc_hash::FxHashMap;

//Faster than base HashMap
use super::saver::save_one_map_r_files;
use std::fs;
use std::io::{BufRead, Cursor, Read};
use std::time::Instant;

/// ## The Map task
/// Maps the file at `path` : counts the number time each word appears into the `map` arg.
pub fn run_map_task(path: &str, r: usize, map_id: usize) -> std::io::Result<Vec<(String, f64)>> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();

    let mut ret = map_file(path, &mut map).unwrap();
    let start = Instant::now();
    save_one_map_r_files(&map, r, MAP_DATA_PATH, map_id).unwrap();
    let end = start.elapsed().as_secs_f64();
    ret.push(("saving".to_string(), end));
    Ok(ret)
}

pub fn run_map_task_default(path: &str) -> std::io::Result<Vec<(String, f64)>> {
    run_map_task(path, REDUCE_TASKS_AMOUNT, 0)
}

pub fn map_file(
    path: &str,
    map: &mut FxHashMap<String, u32>,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut ret: Vec<(String, f64)> = vec![];
    let start = Instant::now();
    let file_bytes = fs::read(path)?;
    let mut reader = Cursor::new(file_bytes);
    let end = start.elapsed().as_secs_f64();
    ret.push(("reader".to_string(), end));

    let start = Instant::now();
    let mut skip_first_body: bool = true;

    // Parsing buffers :
    let mut line = String::new();
    let mut chunk_bytes: Vec<u8> = Vec::with_capacity(5000);

    loop {
        line.clear();
        // Reading the lines
        // If zero bytes are read, we hit EOF
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        //Else we start a new chunk of data

        //First line should be a version type. We can ignore it. Though let's check if it is just in case :
        assert_eq!(line, "WARC/1.0\r\n");
        let content_length;
        //We need to find the size of the chunk :
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            let trimmed_line = line.trim();

            if let Some((key, value)) = trimmed_line.split_once(":") {
                let key = key.trim().to_ascii_lowercase();
                if key == "content-length" {
                    content_length = value.trim().parse::<usize>().unwrap();
                    //This also marks the end of the header
                    break;
                }
            }
        }

        //There are 2 additionnal bytes between header and body : \r and \n
        //There are 4 additionnal bytes between body and next header : \r and \n repeated twice
        //We can simply discard them
        //We also now know the size of data to read, which gives :
        let total_to_read = content_length + 6;
        if chunk_bytes.len() < total_to_read {
            chunk_bytes.resize(total_to_read, 0);
        }
        reader
            .read_exact(&mut chunk_bytes[..total_to_read])
            .unwrap();

        if !skip_first_body {
            let contents: &mut str =
                str::from_utf8_mut(&mut chunk_bytes[2..content_length + 2]).unwrap();
            map_single_chunk(contents, map).unwrap();
        } else {
            skip_first_body = false;
        }
    }
    let end = start.elapsed().as_secs_f64();
    ret.push(("mapping".to_string(), end));
    Ok(ret)
}

pub fn map_single_chunk(
    contents: &mut str,
    map: &mut FxHashMap<String, u32>,
) -> std::io::Result<()> {
    contents.make_ascii_lowercase();

    let words = contents.split(|c: char| {
        c == ' '
            || c == '\n'
            || c == '\r'
            || c == '.'
            || c == ','
            || c == '?'
            || c == ':'
            || c == '!'
            || c == '('
            || c == ')'
            || c == ';'
    });

    for word in words {
        if word.is_empty() {
            continue;
        }
        if let Some(count) = map.get_mut(word) {
            *count += 1;
        } else {
            map.insert(word.to_string(), 1);
        }
    }

    Ok(())
}
