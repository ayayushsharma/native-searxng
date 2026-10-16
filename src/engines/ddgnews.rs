//! DuckDuckGo News (`html.duckduckgo.com`) — challenged at this IP, news.
//! Same no-JS HTML endpoint as the `ddg` engine with the news vertical
//! (`iar=news`); same `anomaly.js` bot-wall, reported via `challenge()`
//! as 0 results + an `unresponsive_engines` entry (iron rule 5).
//! Ported for coverage parity with the reference: it auto-recovers the
//! day this egress IP's reputation clears, with zero code changes.
//! No live fixture possible while challenged (see ddg anomaly fixture).

use crate::engine::{ddg_parse, Engine, SearchResult};

pub struct DdgNews;

impl Engine for DdgNews {
    fn name(&self) -> &'static str {
        "ddgnews"
    }

    fn category(&self) -> &'static str {
        "news"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://html.duckduckgo.com/html/?q={}&iar=news", encode(q))
    }

    fn challenge(&self, body: &str) -> Option<String> {
        if body.contains("anomaly.js") || body.contains("challenge-form") {
            return Some("bot challenge (anomaly.js)".into());
        }
        None
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        // Same card markup as the web vertical (verified against the
        // reference `duckduckgo_extra` news selectors when unblocked).
        // Shared hand scanner: no scraper/encoding_rs.
        ddg_parse(body, "ddgnews")
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
