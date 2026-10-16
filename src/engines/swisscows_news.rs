//! Swisscows news — plain JSON sibling of the signed web flow.
//! `GET api.swisscows.com/news/search?query=&itemsCount=&region=de-DE`
//! → `{"items": [{uri, title, excerpt, created}]}`. Titles/excerpts carry
//! `<em>` highlight markup (stripped); `created` is ISO datetime.

use crate::engine::{strip_tags, Engine, SearchResult};
use serde_json::Value;

pub struct SwisscowsNews;

const BASE: &str = "https://api.swisscows.com/news/search";

impl Engine for SwisscowsNews {
    fn name(&self) -> &'static str {
        "swisscows_news"
    }

    fn category(&self) -> &'static str {
        "news"
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        BASE.to_string()
    }

    fn request(
        &self,
        query: &str,
        pageno: u32,
        prev: &[crate::engine::InResponse],
    ) -> Option<crate::engine::OutRequest> {
        if !prev.is_empty() {
            return None;
        }
        let offset = (pageno.saturating_sub(1)) * 10;
        Some(crate::engine::OutRequest {
            url: format!(
                "{BASE}?query={}&itemsCount=10&region=de-DE&offset={offset}",
                encode(query),
            ),
            method: crate::engine::Method::Get,
            form: vec![],
            json: None,
            headers: vec![],
        })
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let items = v.get("items").and_then(|x| x.as_array()).unwrap_or(&empty);
        items
            .iter()
            .filter_map(|r| {
                let url = r.get("uri").and_then(|x| x.as_str())?.trim();
                let title = r.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
                if url.is_empty() || title.is_empty() {
                    return None;
                }
                let desc = r.get("excerpt").and_then(|x| x.as_str()).unwrap_or("");
                let mut res = SearchResult::basic(
                    "swisscows_news",
                    url.to_string(),
                    strip_tags(title),
                    strip_tags(desc),
                );
                if let Some(d) = r.get("created").and_then(|x| x.as_str()) {
                    if d.len() >= 10 {
                        res.published_date = Some(d[..10].to_string());
                    }
                }
                Some(res)
            })
            .take(10)
            .collect()
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
