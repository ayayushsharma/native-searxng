//! DuckDuckGo HTML endpoint (lenient, no-JS form service).
//! Parser is fixture-driven (see tests/): substring spans only, skip on mismatch.

use crate::engine::{ddg_parse, Engine, SearchResult};

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
        // egress IPs (see tests/fixtures/tests-fixture-ddg-anomaly.html). Never surface it
        // as results — report unresponsive instead (iron rule 5).
        if body.contains("anomaly.js") || body.contains("challenge-form") {
            return Some("bot challenge (anomaly.js)".into());
        }
        None
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        // Shared hand scanner (no scraper/encoding_rs): DDG is UTF-8.
        ddg_parse(body, "ddg")
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
