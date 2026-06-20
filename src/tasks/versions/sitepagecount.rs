use crate::tasks::versions::TaskVersion;

pub struct SitePageCountVersion {}

impl TaskVersion for SitePageCountVersion {
    type Intermediate = u32;
    type Final = u32;
    const NEEDS_LANGUAGE: bool = false;
    const NEEDS_URL: bool = true;

    fn map_single_chunk(
        _raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        _content_length: usize,
        _languages: Vec<String>,
        site: Option<String>,
    ) {
        let site = site.unwrap();
        if let Some(count) = map.get_mut(&site) {
            *count += 1;
        } else {
            map.insert(site.to_string(), 1);
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
