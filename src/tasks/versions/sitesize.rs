use crate::tasks::versions::TaskVersion;

pub struct SiteSizeVersion {}

impl TaskVersion for SiteSizeVersion {
    type Intermediate = u128;
    type Final = u128;
    const NEEDS_LANGUAGE: bool = false;
    const NEEDS_URL: bool = true;

    fn map_single_chunk(
        _raw_chunk_bytes: &mut [u8],
        map: &mut rustc_hash::FxHashMap<String, Self::Intermediate>,
        content_length: usize,
        _languages: Vec<String>,
        site: Option<String>,
    ) {
        let site = site.unwrap();
        if let Some(count) = map.get_mut(&site) {
            *count += content_length as u128;
        } else {
            map.insert(site.to_string(), content_length as u128);
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
