use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, Cursor, Read};
use std::time::Instant;
use unicode_script::{Script, UnicodeScript};

const LANGUAGES_TO_SPLIT_BY_CHAR: &[&str] = &["zho", "jpn", "kor"];

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
        let mut languages_to_split: Vec<String> = vec![];
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
                            languages_to_split.push(language.to_string());
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
            let contents: &mut str =
                str::from_utf8_mut(&mut chunk_bytes[2..content_length + 2]).unwrap();
            let true_languages_to_split: Vec<String> = if languages_to_split.is_empty() {
                vec!["all".to_string()]
            } else {
                let mut temp = vec![];
                for l in languages_to_split {
                    if LANGUAGES_TO_SPLIT_BY_CHAR.contains(&l.as_str()) {
                        temp.push(l.to_string());
                    }
                }
                temp
            };

            map_single_chunk(contents, map, true_languages_to_split).unwrap();
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
    languages_to_split: Vec<String>,
) -> std::io::Result<()> {
    if languages_to_split.is_empty() {
        super::default::map_single_chunk(contents, map)
    } else {
        let mut list_of_languages = HashSet::new();

        let mut true_languages_to_split = vec![];
        if languages_to_split.contains(&"all".to_string()) {
            for l in LANGUAGES_TO_SPLIT_BY_CHAR {
                true_languages_to_split.push(l.to_string());
            }
        } else {
            true_languages_to_split = languages_to_split;
        }

        for language in &true_languages_to_split {
            if language == "zho" {
                list_of_languages.insert(Script::Han);
            } else if language == "jpn" {
                list_of_languages.insert(Script::Han);
                list_of_languages.insert(Script::Hiragana);
                list_of_languages.insert(Script::Katakana);
            } else if language == "kor" {
                list_of_languages.insert(Script::Hangul);
            }
        }

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
                || c == '-'
                || c == '_'
                || c == '"'
                || c == '{'
                || c == '}'
                || c == '['
                || c == ']'
                || c == '+'
                || c == '='
                || c == '/'
                || c == '\\'
        });

        let mut split_words = vec![];
        for word in words {
            let mut last_index = 0;
            for (index, matched_char) in
                word.match_indices(|c: char| list_of_languages.contains(&c.script()))
            {
                // Push the text before the match (if any)
                if index > last_index {
                    split_words.push(&word[last_index..index]);
                }
                // Push the matching character itself
                split_words.push(matched_char);
                last_index = index + matched_char.len();
            }
        }

        for word in split_words {
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
}
