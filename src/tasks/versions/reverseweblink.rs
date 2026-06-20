use std::sync::LazyLock;

use regex::Regex;
use rustc_hash::FxHashSet;

use crate::tasks::versions::TaskVersion;

pub struct ReverseWebLinkVersion {}

pub static URL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    let pattern = r#"(?:(?:https?|ftp)://(?:www\.)?|www\.)[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z]{2,}(?::[0-9]{1,5})?(?:[/?#][^\s<>"'{};|\\^\[\]`]*)?|\b(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+(?:com|net|org|edu|gov|io|co|app|dev|ai|uk|de|fr|ca|au|jp|it|es|info|biz|me|tv|us|xyz|online|site|tech|blog|shop)\b(?::[0-9]{1,5})?(?:[/?#][^\s<>"'{};|\\^\[\]`]*)?"#;
    Regex::new(pattern).expect("Failed to compile URL regex")
});

pub fn extract_domain(url: &str) -> &str {
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

impl TaskVersion for ReverseWebLinkVersion {
    type Intermediate = FxHashSet<String>;
    type Final = FxHashSet<String>;
    const NEEDS_LANGUAGE: bool = false;
    const NEEDS_URL: bool = true;

    fn map_single_chunk(
        raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        _content_length: usize,
        _languages: Vec<String>,
        site: Option<String>,
    ) {
        let contents = str::from_utf8_mut(raw_chunk_bytes).unwrap();
        let site = site.unwrap();
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
            if is_lower && let Some(set) = map.get_mut(domain) {
                // 4. DEFER CLONING
                // If the set doesn't have the URL, clone and insert.
                // If it DOES have it, we just saved a costly String::clone() allocation!
                if !set.contains(&site) {
                    set.insert(site.clone());
                }
                continue; // Skip the map.entry logic entirely
            }

            // Fallback: The domain is either new to our map, or contains uppercase characters.
            let key = if is_lower {
                domain.to_owned() // Exact copy, avoids case conversion overhead
            } else {
                domain.to_ascii_lowercase() // Allocates and converts
            };

            let set = map.entry(key).or_insert_with(FxHashSet::default);
            if !set.contains(&site) {
                set.insert(site.clone());
            }
        }
    }

    fn reduce_merge_maps(
        source_map: &mut rustc_hash::FxHashMap<String, Self::Final>,
        other: rustc_hash::FxHashMap<String, Self::Intermediate>,
    ) {
        for (key, val) in other {
            if let Some(vector) = source_map.get_mut(&key) {
                for s in val {
                    vector.insert(s);
                }
            } else {
                source_map.insert(key, val);
            }
        }
    }
}
