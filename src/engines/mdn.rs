//! MDN Web Docs (`developer.mozilla.org`) — keyless JSON API, it.
//! `https://developer.mozilla.org/api/v1/search?q=..&page=..`
//! Response: `{"documents": [{mdn_url, title, summary, ...}]}`.
//! Fixture: `tests-fixture-mdn-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Mdn;

impl Engine for Mdn {
    fn name(&self) -> &'static str {
        "mdn"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://developer.mozilla.org/api/v1/search?q={}&page={}",
            encode(q),
            pageno.clamp(1, 3)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("documents")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let (Some(path), Some(title)) = (
                r.get("mdn_url").and_then(|x| x.as_str()),
                r.get("title").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if path.is_empty() || title.is_empty() {
                continue;
            }
            let summary = r
                .get("summary")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            out.push(SearchResult::basic(
                "mdn",
                format!("https://developer.mozilla.org{path}"),
                title.split_whitespace().collect::<Vec<_>>().join(" "),
                summary.split_whitespace().collect::<Vec<_>>().join(" "),
            ));
        }
        out
    }
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
