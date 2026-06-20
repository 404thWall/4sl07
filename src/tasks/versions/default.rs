use crate::tasks::versions::TaskVersion;

pub struct DefaultVersion {}

impl TaskVersion for DefaultVersion {
    type Intermediate = u32;
    type Final = u32;
    const NEEDS_LANGUAGE: bool = false;
    const NEEDS_URL: bool = false;

    fn map_single_chunk(
        raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        _content_length: usize,
        _languages: Vec<String>,
        _site: Option<String>,
    ) {
        let contents = str::from_utf8_mut(raw_chunk_bytes).unwrap();
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
