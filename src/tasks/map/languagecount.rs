use rustc_hash::FxHashMap;
use std::fs;
use std::io::{BufRead, Cursor, Read};
use std::time::Instant;

pub fn map_file(
    path: &str,
    map: &mut FxHashMap<String, u32>,
) -> std::io::Result<Vec<(String, f64)>> {
    let mut ret: Vec<(String, f64)> = vec![];
    let start = Instant::now();
    let file_bytes = fs::read(path)?;
    let size = file_bytes.len();
    let mut reader = Cursor::new(file_bytes);
    let end = start.elapsed().as_secs_f64();
    ret.push(("reading_time".to_string(), end));

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
        let mut languages_to_count: Vec<String> = vec![];
        //We need to find the size of the chunk as well as its languages now :
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            let trimmed_line = line.trim();

            if let Some((key, value)) = trimmed_line.split_once(":") {
                let key = key.trim().to_ascii_lowercase();
                if key == "warc-identified-content-language" {
                    let key = key.trim().to_ascii_lowercase();
                    if key == "warc-identified-content-language" {
                        for language in value.trim().split(",") {
                            languages_to_count.push(language.to_string());
                        }
                    }
                }
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
            // We don't care about the contents
            //let contents: &mut str =
            //    str::from_utf8_mut(&mut chunk_bytes[2..content_length + 2]).unwrap();

            if languages_to_count.is_empty() {
                // CommonCrawl did not provide a language.
                // We push unknown to the map.
                if let Some(count) = map.get_mut("unknown") {
                    *count += 1;
                } else {
                    map.insert("unknown".to_string(), 1);
                }
            } else {
                // CommonCrawl did provide at least one language.
                // We push those to the map.
                for l in languages_to_count {
                    if let Some(count) = map.get_mut(&l) {
                        *count += 1;
                    } else {
                        map.insert(l, 1);
                    }
                }
            }
        } else {
            skip_first_body = false;
        }
    }
    let end = start.elapsed().as_secs_f64();
    ret.push(("mapping_time".to_string(), end));
    ret.push(("input_size".to_string(), size as f64));
    Ok(ret)
}
