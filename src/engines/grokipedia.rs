//! Grokipedia full-text search — keyless JSON API, general.
//! `https://grokipedia.com/api/full-text-search?query=..&limit=10&offset=..`
//! Response: `{"results": [{slug, title, snippet, ...}]}`.
//! URL shape (mirrors SearXNG): `https://grokipedia.com/page/<slug>`.
//! Fixture: `tests/fixtures/tests-fixture-grokipedia-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Grokipedia;

impl Engine for Grokipedia {
    fn name(&self) -> &'static str {
        "grokipedia"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let offset = (pageno.clamp(1, 3) - 1) * 10;
        format!(
            "https://grokipedia.com/api/full-text-search?query={}&limit=10&offset={}",
            encode(q),
            offset
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("results")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let (Some(slug), Some(title)) = (
                r.get("slug").and_then(|x| x.as_str()),
                r.get("title").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if slug.is_empty() || title.is_empty() {
                continue;
            }
            let snippet = r
                .get("snippet")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            let content: String = snippet.split_whitespace().collect::<Vec<_>>().join(" ");
            out.push(SearchResult::basic(
                "grokipedia",
                format!("https://grokipedia.com/page/{slug}"),
                title.into(),
                content,
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
