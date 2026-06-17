use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fs;
use std::io::{BufRead, Cursor, Read};
use std::sync::LazyLock;
use std::time::Instant;

static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let pattern = r#"(?:(?:https?|ftp)://(?:www\.)?|www\.)[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z]{2,}(?::[0-9]{1,5})?(?:[/?#][^\s<>"'{};|\\^\[\]`]*)?|\b(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+(?:com|net|org|edu|gov|io|co|app|dev|ai|uk|de|fr|ca|au|jp|it|es|info|biz|me|tv|us|xyz|online|site|tech|blog|shop)\b(?::[0-9]{1,5})?(?:[/?#][^\s<>"'{};|\\^\[\]`]*)?"#;
    Regex::new(pattern).expect("Failed to compile URL regex")
});

pub fn map_file(
    path: &str,
    map: &mut FxHashMap<String, FxHashSet<String>>,   
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
        let mut site: String = "Placeholder".to_string();
        //We need to find the size of the chunk as well as its languages now :
        loop {
            line.clear();
            reader.read_line(&mut line)?;
            let trimmed_line = line.trim();

            if let Some((key, value)) = trimmed_line.split_once(":") {
                let key = key.trim().to_ascii_lowercase();
                if key == "warc-target-uri" {
                    let url = value.trim();
                    let temp = match url.split_once("//") {
                        Some((_, rest)) => rest.split("/").next().unwrap(),
                        None => url.split("/").next().unwrap(),
                    };
                    site = match temp.split_once("www.") {
                        Some((_, rest)) => rest,
                        None => temp,
                    }
                    .to_string();
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
            let contents: &mut str =
                str::from_utf8_mut(&mut chunk_bytes[2..content_length + 2]).unwrap();

            map_single_chunk(contents, map, site).unwrap();
        } else {
            skip_first_body = false;
        }
    }
    let end = start.elapsed().as_secs_f64();
    ret.push(("mapping_time".to_string(), end));
    ret.push(("input_size".to_string(), size as f64));
    Ok(ret)
}

fn extract_domain(url: &str) -> &str {
    let s = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("ftp://"))
        .unwrap_or(url);
        
    let s = s.strip_prefix("www.").unwrap_or(s);

    // Everything before the first path / query / fragment
    let host_and_port = s.split(['/', '?', '#']).next().unwrap_or(s);
    // Strip port if present
    host_and_port.split(':').next().unwrap_or(host_and_port)
}

pub fn map_single_chunk(
    contents: &str,
    map: &mut FxHashMap<String, FxHashSet<String>>,
    original_url: String,
) -> std::io::Result<()> {
    
    for mat in URL_REGEX.find_iter(contents) {
        let url = mat.as_str().trim_end_matches(|c: char| {
            matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']')
        });

        if url.is_empty() {
            continue;
        }

        let domain = extract_domain(url);
        
        // Most domains in the wild are already lowercase. We can check this cheaply.
        let is_lower = !domain.bytes().any(|b| b.is_ascii_uppercase());
        
        // If it's already lowercase, we can try to look it up in the map using the `&str` 
        // without allocating a brand new `String` key first.
        if is_lower {
            if let Some(set) = map.get_mut(domain) {
                // 4. DEFER CLONING
                // If the set doesn't have the URL, clone and insert.
                // If it DOES have it, we just saved a costly String::clone() allocation!
                if !set.contains(&original_url) {
                    set.insert(original_url.clone());
                }
                continue; // Skip the map.entry logic entirely
            }
        }

        // Fallback: The domain is either new to our map, or contains uppercase characters.
        let key = if is_lower {
            domain.to_owned() // Exact copy, avoids case conversion overhead
        } else {
            domain.to_ascii_lowercase() // Allocates and converts
        };

        let set = map.entry(key).or_insert_with(FxHashSet::default);
        if !set.contains(&original_url) {
            set.insert(original_url.clone());
        }
    }

    Ok(())
}