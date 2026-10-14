//! Mwmbl (`api.mwmbl.org`) — keyless JSON API, general web index.
//! `https://api.mwmbl.org/api/v1/search/?s=..`
//! Response: array of `{url, title: [{value, is_bold}], extract: [{value}]}`.
//! Fixture: `tests-fixture-mwmbl-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Mwmbl;

impl Engine for Mwmbl {
    fn name(&self) -> &'static str {
        "mwmbl"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://api.mwmbl.org/api/v1/search/?s={}", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v.as_array().cloned().unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let Some(url) = r.get("url").and_then(|x| x.as_str()) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }
            let title = join_parts(r.get("title"));
            let extract = join_parts(r.get("extract"));
            if title.is_empty() {
                continue;
            }
            out.push(SearchResult::basic("mwmbl", url.into(), title, extract));
        }
        out
    }
}

/// `[{value, is_bold}]` → concatenated text (bold markers are presentation).
fn join_parts(v: Option<&serde_json::Value>) -> String {
    let parts = v
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let s: String = parts
        .iter()
        .filter_map(|p| p.get("value").and_then(|x| x.as_str()))
        .collect();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
