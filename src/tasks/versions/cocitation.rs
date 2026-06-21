use rustc_hash::FxHashSet;

use crate::tasks::versions::{
    TaskVersion,
    reverseweblink::{URL_REGEX, extract_domain},
};

pub struct CoCitationVersion {}

impl TaskVersion for CoCitationVersion {
    type Intermediate = u128;
    type Final = u128;
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
        let stuff: Vec<_> = URL_REGEX.find_iter(contents).collect();
        let mut filtered: FxHashSet<String> = FxHashSet::default();

        for mat in stuff {
            let url = mat.as_str().trim_end_matches(|c: char| {
                matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']')
            });
            if url.is_empty() {
                continue;
            }
            let domain = extract_domain(url).to_ascii_lowercase();
            filtered.insert(domain);
        };
        let mut seen : FxHashSet<&String> = FxHashSet::default();
        for first_site in &filtered {
            seen.insert(first_site);
            for second_site in &filtered {
                if !seen.contains(second_site) {
                    let (f, s) = if first_site < second_site {
                        (first_site.clone(), second_site.clone())
                    } else {
                        (second_site.clone(), first_site.clone())
                    };
                    let key = format!("{f}|{s}");
                    if let Some(count) = map.get_mut(&key) {
                        *count += 1;
                    } else {
                        map.insert(key, 1);
                    }

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
