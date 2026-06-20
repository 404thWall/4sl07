use rustc_hash::FxHashSet;

use crate::tasks::versions::{
    TaskVersion,
    reverseweblink::{URL_REGEX, extract_domain},
};

pub struct InOutLinksVersion {}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LinksData {
    inlinks: u128,
    outlinks: u128,
}

impl TaskVersion for InOutLinksVersion {
    type Intermediate = LinksData;
    type Final = LinksData;
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
        let stuff: Vec<_> = URL_REGEX.find_iter(contents).collect();
        let outlinks = stuff.len() as u128;
        let mut seen_domains: FxHashSet<String> = FxHashSet::default();
        for mat in stuff {
            let url = mat.as_str().trim_end_matches(|c: char| {
                matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']')
            });
            if url.is_empty() {
                continue;
            }
            let domain = extract_domain(url).to_ascii_lowercase();
            // We don't want to count a site linking itself.
            if domain == site {
                continue;
            }

            // We don't want to count the same url twice.
            if !seen_domains.contains(&domain) {
                if let Some(links_data) = map.get_mut(&domain) {
                    links_data.inlinks += 1;
                } else {
                    map.insert(
                        domain.clone(),
                        LinksData {
                            inlinks: 1,
                            outlinks: 0,
                        },
                    );
                }

                seen_domains.insert(domain);
            }
        }

        if let Some(links_data) = map.get_mut(&site) {
            links_data.outlinks += outlinks;
        } else {
            map.insert(
                site,
                LinksData {
                    inlinks: 0,
                    outlinks,
                },
            );
        }
    }

    fn reduce_merge_maps(
        source_map: &mut rustc_hash::FxHashMap<String, Self::Final>,
        other: rustc_hash::FxHashMap<String, Self::Intermediate>,
    ) {
        for (key, val) in other {
            if let Some(vector) = source_map.get_mut(&key) {
                vector.inlinks += val.inlinks;
                vector.outlinks += val.outlinks;
            } else {
                source_map.insert(key, val);
            }
        }
    }
}
