use rustc_hash::FxHashMap; //Faster than base HashMap
use std::fs::File;
use std::io::{BufRead, BufReader, Read};

static WORD_TO_TEST: &str = "the";

pub fn run(path: &str) -> std::io::Result<()> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();

    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

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
        chunk_bytes.resize(content_length + 6, 0);
        reader.read_exact(&mut chunk_bytes[..content_length+6]).unwrap();

        let contents: &mut str =
            str::from_utf8_mut(&mut chunk_bytes[2..content_length + 2]).unwrap();
        split_single_chunk(contents, &mut map).unwrap();
    }

    if let Some(count) = map.get(WORD_TO_TEST) {
        println!(
            "As an example, the word '{WORD_TO_TEST}' was present {} times",
            count
        );
    }

    Ok(())
}

pub fn split_single_chunk(
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
