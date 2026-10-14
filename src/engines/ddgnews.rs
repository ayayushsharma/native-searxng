//! DuckDuckGo News (`html.duckduckgo.com`) — challenged at this IP, news.
//! Same no-JS HTML endpoint as the `ddg` engine with the news vertical
//! (`iar=news`); same `anomaly.js` bot-wall, reported via `challenge()`
//! as 0 results + an `unresponsive_engines` entry (iron rule 5).
//! Ported for coverage parity with the reference: it auto-recovers the
//! day this egress IP's reputation clears, with zero code changes.
//! No live fixture possible while challenged (see ddg anomaly fixture).

use scraper::{Html, Selector};

use crate::engine::{Engine, SearchResult};

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
            out.push(SearchResult::basic("ddgnews", url, title, content));
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
