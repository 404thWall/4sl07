use crate::tasks::versions::TaskVersion;

pub struct LanguageSizeVersion {}

impl TaskVersion for LanguageSizeVersion {
    type Intermediate = u128;
    type Final = u128;
    const NEEDS_LANGUAGE: bool = true;
    const NEEDS_URL: bool = false;

    fn map_single_chunk(
        _raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        content_length: usize,
        languages: Vec<String>,
        _site: Option<String>,
    ) {
        if languages.is_empty() {
            // CommonCrawl did not provide a language.
            // We push unknown to the map.
            if let Some(count) = map.get_mut("unknown") {
                *count += content_length as u128;
            } else {
                map.insert("unknown".to_string(), content_length as u128);
            }
        } else {
            // CommonCrawl did provide at least one language.
            // We push those to the map.
            for l in languages {
                if let Some(count) = map.get_mut(&l) {
                    *count += content_length as u128;
                } else {
                    map.insert(l, content_length as u128);
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
