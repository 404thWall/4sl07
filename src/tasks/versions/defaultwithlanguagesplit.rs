use rustc_hash::FxHashSet;

use crate::tasks::versions::{TaskVersion, default::DefaultVersion};
use unicode_script::{Script, UnicodeScript};

const LANGUAGES_TO_SPLIT_BY_CHAR: &[&str] = &["zho", "jpn", "kor"];
pub struct DefaultWithLanguageSplitVersion {}

impl TaskVersion for DefaultWithLanguageSplitVersion {
    type Intermediate = u32;
    type Final = u32;
    const NEEDS_LANGUAGE: bool = true;
    const NEEDS_URL: bool = false;

    fn map_single_chunk(
        raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        content_length: usize,
        languages: Vec<String>,
        site: Option<String>,
    ) {
        let languages_to_split: Vec<String> = if languages.is_empty() {
            vec!["all".to_string()]
        } else {
            let mut temp = vec![];
            for l in languages {
                if LANGUAGES_TO_SPLIT_BY_CHAR.contains(&l.as_str()) {
                    temp.push(l.to_string());
                }
            }
            temp
        };
        if languages_to_split.is_empty() {
            DefaultVersion::map_single_chunk(
                raw_chunk_bytes,
                map,
                content_length,
                languages_to_split,
                site,
            );
        } else {
            let contents = str::from_utf8_mut(raw_chunk_bytes).unwrap();
            let mut list_of_languages = FxHashSet::default();

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
        }
    }

    fn reduce_merge_maps(
        source_map: &mut rustc_hash::FxHashMap<String, Self::Final>,
        other: rustc_hash::FxHashMap<String, Self::Intermediate>,
    ) {
        for (key, val) in other {
            if let Some(count) = source_map.get_mut(&key) {
                *count += val;
            } else {
                source_map.insert(key, val);
            }
        }
    }
}
