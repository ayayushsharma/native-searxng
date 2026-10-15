//! Merge: dedup by normalized URL, round-robin across engines (preserves
//! configured engine order/weight). No result cap: the full fan-out is
//! returned like the reference (40-115/q typical). Positions assigned
//! in merged order (SearXNG-compatible `positions` field).

use crate::engine::{normalize_url, SearchResult};
use std::collections::HashSet;

pub fn merge(lists: Vec<Vec<SearchResult>>) -> Vec<SearchResult> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut idx = vec![0usize; lists.len()];
    loop {
        let mut progressed = false;
        for (li, list) in lists.iter().enumerate() {
            while idx[li] < list.len() {
                let r = &list[idx[li]];
                idx[li] += 1;
                let key = normalize_url(&r.url);
                if key.is_empty() || !seen.insert(key) {
                    continue;
                }
                let mut r = r.clone();
                r.positions = vec![out.len() + 1];
                out.push(r);
                progressed = true;
                break;
            }
        }
        if !progressed {
            break;
        }
    }
    out
}
