//! Startpagina (`search.kompas.services`) — keyless JSON API, general web.
//! `https://search.kompas.services/api/v2/search/web/?q=..&page_size=10&page=..`
//! Response: `{"results": [{original_url, title, description, ...}]}`.
//! Fixture: `tests/fixtures/tests-fixture-startpagina-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Startpagina;

impl Engine for Startpagina {
    fn name(&self) -> &'static str {
        "startpagina"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://search.kompas.services/api/v2/search/web/?q={}&page_size=10&page={}",
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
            .get("results")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let (Some(url), Some(title)) = (
                r.get("original_url").and_then(|x| x.as_str()),
                r.get("title").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if url.is_empty() || title.is_empty() {
                continue;
            }
            let desc = r
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            let content: String = desc.split_whitespace().collect::<Vec<_>>().join(" ");
            out.push(SearchResult::basic(
                "startpagina",
                url.into(),
                title.split_whitespace().collect::<Vec<_>>().join(" "),
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
