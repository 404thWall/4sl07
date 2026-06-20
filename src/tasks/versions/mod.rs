use std::{
    fs,
    io::{BufRead, Cursor, Read},
    path::Path,
    time::Instant,
};

use clap::ValueEnum;
use rustc_hash::FxHashMap;

use crate::tasks::saver::load_map;

pub mod default;
pub mod defaultwithlanguagesplit;
pub mod inoutlinks;
pub mod languagecount;
pub mod languagesize;
pub mod reverseweblink;
pub mod sitepagecount;
pub mod sitesize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum MapReduceVersion {
    Default,
    DefaultWithLanguageSplit,
    InOutLinks,
    LanguageCount,
    LanguageSize,
    ReverseWebLink,
    SitePageCount,
    SiteSize,
}

/// Macro to not have to update this big match everywhere each time a new version is added.
#[macro_export]
macro_rules! versioned {
    ($version:expr, $func:ident ($($args:expr),* $(,)?)) => {
        match $version {
            MapReduceVersion::Default => $func::<DefaultVersion>($($args),*),
            MapReduceVersion::DefaultWithLanguageSplit => $func::<DefaultWithLanguageSplitVersion>($($args),*),
            MapReduceVersion::InOutLinks => $func::<InOutLinksVersion>($($args),*),
            MapReduceVersion::LanguageCount => $func::<LanguageCountVersion>($($args),*),
            MapReduceVersion::LanguageSize => $func::<LanguageSizeVersion>($($args),*),
            MapReduceVersion::ReverseWebLink => $func::<ReverseWebLinkVersion>($($args),*),
            MapReduceVersion::SitePageCount => $func::<SitePageCountVersion>($($args),*),
            MapReduceVersion::SiteSize => $func::<SiteSizeVersion>($($args),*),
            
        }
    };
}

/// The trait to be implemented for new versions of the MapReduce algorithm.
pub trait TaskVersion {
    type Intermediate: serde::Serialize
        + serde::de::DeserializeOwned
        + Clone
        + std::fmt::Debug
        + PartialEq;
    type Final: serde::Serialize + serde::de::DeserializeOwned + Clone + std::fmt::Debug + PartialEq;
    const NEEDS_LANGUAGE: bool;
    const NEEDS_URL: bool;

    /// The actual logic of the _Map Task_. Uses the contents and/or the
    /// information contained in the header to run the Map logic.
    ///
    /// Note : Consider using [str::from_utf8_mut] to retrieve the contents
    ///  as a `&mut str`, which may be easier to use than a `&mut [u8]`.
    fn map_single_chunk(
        raw_chunk_bytes: &mut [u8],
        map: &mut FxHashMap<String, Self::Intermediate>,
        content_length: usize,
        languages: Vec<String>,
        site: Option<String>,
    );

    /// The actual logic of the _Reduce Task_. Is used to merge the output map of the
    /// reduce task with the multiple input maps.
    fn reduce_merge_maps(
        source_map: &mut FxHashMap<String, Self::Final>,
        other: FxHashMap<String, Self::Intermediate>,
    );

    /// Default implementation of the logic for parsing the WARC file
    /// and finding useful information from the headers.
    fn map_file(path: &str, map: &mut FxHashMap<String, Self::Intermediate>) -> Vec<(String, f64)> {
        let mut ret: Vec<(String, f64)> = vec![];

        let start = Instant::now();
        let file_bytes = fs::read(path).unwrap();
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
            if reader.read_line(&mut line).unwrap() == 0 {
                break;
            }
            //Else we start a new chunk of data

            //First line should be a version type. We can ignore it.
            //Though let's check if it is just in case :
            assert_eq!(line, "WARC/1.0\r\n");
            let content_length;
            let mut languages: Vec<String> = vec![];
            let mut site: Option<String> = None;
            //We need to find the size of the chunk as well as its languages now :
            loop {
                line.clear();
                reader.read_line(&mut line).unwrap();
                let trimmed_line = line.trim();

                if let Some((key, value)) = trimmed_line.split_once(":") {
                    let key = key.trim().to_ascii_lowercase();
                    if Self::NEEDS_LANGUAGE && key == "warc-identified-content-language" {
                        for language in value.trim().split(",") {
                            languages.push(language.to_string());
                        }
                    }
                    if Self::NEEDS_URL && key == "warc-target-uri" {
                        let url = value.trim();
                        let temp = match url.split_once("//") {
                            Some((_, rest)) => rest.split("/").next().unwrap(),
                            None => url.split("/").next().unwrap(),
                        };
                        site = Some(
                            match temp.split_once("www.") {
                                Some((_, rest)) => rest,
                                None => temp,
                            }
                            .to_string(),
                        );
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
                Self::map_single_chunk(
                    &mut chunk_bytes[2..content_length + 2],
                    map,
                    content_length,
                    languages,
                    site,
                );
            } else {
                skip_first_body = false;
            }
        }
        let end = start.elapsed().as_secs_f64();
        ret.push(("mapping_time".to_string(), end));
        ret.push(("input_size".to_string(), size as f64));

        ret
    }

    /// Default implementation of the logic for loading the files in
    /// the directory to be reduced. Should no be changed in theory.
    fn reduce_directory(
        directory_path: &str,
        source_map: &mut FxHashMap<String, Self::Final>,
    ) -> u64 {
        let mut ret: u64 = 0;
        let dir_path = Path::new(directory_path);
        if dir_path.is_dir() {
            for path in fs::read_dir(dir_path).unwrap() {
                let path: fs::DirEntry = path.unwrap();
                if let Some(file_path) = path.file_name().to_str() {
                    let (other, size) = load_map(&format!("{directory_path}{file_path}")).unwrap();
                    ret += size;

                    Self::reduce_merge_maps(source_map, other);
                }
            }
        }
        ret
    }
}

impl std::fmt::Display for MapReduceVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = self.to_possible_value().unwrap();
        write!(f, "{}", text.get_name())
    }
}
