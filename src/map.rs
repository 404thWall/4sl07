use rustc_hash::FxHashMap; //Faster than base HashMap
use std::fs::{File, read};
use std::io::{BufRead, BufReader, Read};

static WORD_TO_TEST: &str = "the";
//static MAX_CHUNK_NUMBER :usize = 5;

pub fn run(path : &str) -> std::io::Result<()> {
    let mut map: FxHashMap<String, u32> = FxHashMap::default();
    //let mut current_chunk = 0;

    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    loop {
        let mut line = String::new();
        // Reading the lines
        // If zero bytes are read, we hit EOF
        if reader.read_line(&mut line)? == 0 {
            break; 
        }
        //Else we start a new chunk of data
        
        //First line should be a version type. We can ignore it.
        //println!("{current_chunk} : Should be 'WARC/1.0\\r\\n' : {line:?}");
        let content_length;
        //We need to find the size of the chunk : 
        loop {
            let mut line = String::new();
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
        //We discard them :
        let mut buf = vec![0; 2];
        reader.read_exact(&mut buf).unwrap();

        // We now know the size of data to read : 
        let mut chunk_bytes = vec![0; content_length];
        reader.read_exact(&mut chunk_bytes).unwrap();
        //println!("Chunk of data :\n{:?}\n", chunk_bytes);
        //println!("Readable : \n{:?}", str::from_utf8_mut(&mut chunk_bytes).unwrap());
        let contents: &mut str = str::from_utf8_mut(&mut chunk_bytes).unwrap();
        split_single_chunk(contents, &mut map).unwrap();
        
        //if MAX_CHUNK_NUMBER == current_chunk {
        //    return Ok(())
        //}
        //current_chunk +=1;

        //There are 4 additionnal bytes between header and body : \r and \n repeated twice
        //We discard them :
        let mut buf = vec![0; 4];
        reader.read_exact(&mut buf).unwrap();

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
        map.entry(word.to_owned()).and_modify(|count| *count += 1).or_insert(1);
    }
    
    Ok(())
}
