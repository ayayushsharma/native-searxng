//! Wikinews (`en.wikinews.org`) — MediaWiki API, keyless JSON, news.
//! `https://en.wikinews.org/w/api.php?action=query&list=search&srsearch=..`
//! Response: `{"query": {"search": [{title, snippet, ...}]}}` (same shape
//! as the Gentoo wiki engine; snippets carry searchmatch markup — stripped).
//! Fixture: `tests/fixtures/tests-fixture-wikinews-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Wikinews;

impl Engine for Wikinews {
    fn name(&self) -> &'static str {
        "wikinews"
    }

    fn category(&self) -> &'static str {
        "news"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let _ = pageno;
        format!(
            "https://en.wikinews.org/w/api.php?action=query&list=search&srsearch={}&srlimit=10&format=json&utf8=1",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("query")
            .and_then(|x| x.get("search"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let Some(title) = r.get("title").and_then(|x| x.as_str()) else {
                continue;
            };
            if title.is_empty() {
                continue;
            }
            let snippet = r
                .get("snippet")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            out.push(SearchResult::basic(
                "wikinews",
                format!("https://en.wikinews.org/wiki/{}", title.replace(' ', "_")),
                title.into(),
                strip_tags(snippet),
            ));
        }
        out
    }
}

/// Drop `<...>` markup from MediaWiki snippets. Never panics.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
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
