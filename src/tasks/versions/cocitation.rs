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
    const NEEDS_URL: bool = true;

    fn map_single_chunk(
        raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        _content_length: usize,
        _languages: Vec<String>,
        _site: Option<String>,
    ) {
        let contents = str::from_utf8_mut(raw_chunk_bytes).unwrap();
        let stuff = URL_REGEX.find_iter(contents);
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
        }

        let mut filtered_vec: Vec<&String> = filtered.iter().collect();

        // Fail safe to prevent using too much memory : 
        if filtered_vec.len() > 2000  {
            return;
        }

        filtered_vec.sort();
        for i in 0..filtered_vec.len() {
            let first_site = filtered_vec[i];
            for j in (i + 1)..filtered_vec.len() {
                let second_site = filtered_vec[j];
                {
                    // 1. Construct a temporary borrowed string slice representation on the stack.
                    // We format it into a stack-allocated buffer or use an optimized conditional lookup.
                    let lookup_key = format!("{first_site}|{second_site}");

                    // To completely avoid allocating a String when the key exists,
                    // we first perform a cheap read-only lookup:
                    if let Some(count) = map.get_mut(&lookup_key) {
                        // Key exists! We mutate it in place. Zero new allocations kept.
                        *count += 1;
                    } else {
                        // Key doesn't exist! Only now do we hand ownership over to the map.
                        map.insert(lookup_key, 1);
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
