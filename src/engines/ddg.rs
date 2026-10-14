//! DuckDuckGo HTML endpoint (lenient, no-JS form service).
//! Parser is fixture-driven (see tests/): substring spans only, skip on mismatch.

use crate::engine::{Engine, SearchResult};
use scraper::{Html, Selector};

pub struct Ddg;

impl Engine for Ddg {
    fn name(&self) -> &'static str {
        "ddg"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://html.duckduckgo.com/html/?q={}", encode(q))
    }

    fn challenge(&self, body: &str) -> Option<String> {
        // DDG serves `anomaly.js?cc=botnet` + a challenge-form to flagged
        // egress IPs (see tests-fixture-ddg-anomaly.html). Never surface it
        // as results — report unresponsive instead (iron rule 5).
        if body.contains("anomaly.js") || body.contains("challenge-form") {
            return Some("bot challenge (anomaly.js)".into());
        }
        None
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        // encoding_rs first: DDG is UTF-8, but normalize anyway (pattern for
        // GBK/1251 engines later).
        let (text, _, _) = encoding_rs::UTF_8.decode(body.as_bytes());
        let doc = Html::parse_document(&text);
        let Ok(block) = Selector::parse(".result") else {
            return vec![];
        };
        let Ok(link) = Selector::parse(".result__a") else {
            return vec![];
        };
        let Ok(snippet) = Selector::parse(".result__snippet") else {
            return vec![];
        };
        let mut out = Vec::new();
        for el in doc.select(&block).take(10) {
            let Some(a) = el.select(&link).next() else {
                continue;
            };
            let url = a.value().attr("href").unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let title: String = a.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
            let content: String = el
                .select(&snippet)
                .next()
                .map(|s| s.text().collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            out.push(SearchResult::basic("ddg", url, title, content));
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
