//! Open Library `search.json` — keyless JSON API, general/books.
//! `https://openlibrary.org/search.json?q=..&page=..&limit=10`
//! Fixture: `tests/fixtures/tests-fixture-openlibrary-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Openlibrary;

impl Engine for Openlibrary {
    fn name(&self) -> &'static str {
        "openlibrary"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://openlibrary.org/search.json?q={}&page={}&limit=10&fields=key,title,author_name,first_sentence",
            encode(q),
            pageno.clamp(1, 3)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let docs = v
            .get("docs")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for d in docs.iter().take(10) {
            let (Some(key), Some(title)) = (
                d.get("key").and_then(|x| x.as_str()),
                d.get("title").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if key.is_empty() || title.is_empty() {
                continue;
            }
            let authors = d
                .get("author_name")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            // first_sentence may be a list (fields=*) or a plain string.
            let sentence = match d.get("first_sentence") {
                Some(serde_json::Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str())
                    .collect::<Vec<_>>()
                    .join(" / "),
                Some(serde_json::Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            let content = match (sentence.is_empty(), authors.is_empty()) {
                (true, true) => String::new(),
                (true, false) => authors,
                (false, true) => sentence,
                (false, false) => format!("{sentence} — {authors}"),
            };
            out.push(SearchResult::basic(
                "openlibrary",
                format!("https://openlibrary.org{key}"),
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
