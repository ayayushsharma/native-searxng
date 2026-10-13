//! Wikipedia via the opensearch JSON API — no scraping, near-free port.
//! `https://en.wikipedia.org/w/api.php?action=opensearch&search=..&format=json`
//! Response: [query, [titles], [descriptions], [urls]].

use crate::engine::{Engine, SearchResult};

pub struct Wikipedia;

impl Engine for Wikipedia {
    fn name(&self) -> &'static str {
        "wikipedia"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://en.wikipedia.org/w/api.php?action=opensearch&search={}&limit=10&format=json",
            url_encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let titles = v.get(1).and_then(|x| x.as_array()).cloned().unwrap_or_default();
        let descs = v.get(2).and_then(|x| x.as_array()).cloned().unwrap_or_default();
        let urls = v.get(3).and_then(|x| x.as_array()).cloned().unwrap_or_default();
        let mut out = Vec::new();
        for i in 0..titles.len().min(10) {
            let (Some(t), Some(u)) = (titles[i].as_str(), urls.get(i).and_then(|x| x.as_str())) else {
                continue;
            };
            let d = descs.get(i).and_then(|x| x.as_str()).unwrap_or("");
            out.push(SearchResult::basic("wikipedia", u.into(), t.into(), d.into()));
        }
        out
    }
}

fn url_encode(s: &str) -> String {
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
