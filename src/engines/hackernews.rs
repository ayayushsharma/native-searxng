//! Hacker News (`hn.algolia.com/api/v1`) — keyless JSON API, it.
//! `https://hn.algolia.com/api/v1/search?query=..&page=..&hitsPerPage=10`
//! Response: `{hits: [{title, url, author, points, num_comments,
//! created_at, objectID}]}`. Ask-HN/self posts have no `url` → link to
//! the HN item thread instead.
//! Usage policy (checked 2026-10-15): public documented community
//! endpoint, no key, no auth; upstream ships an engine against it.
//! Fixture: `tests-fixture-hackernews-test.json` (HTTP 200, clean).

use crate::engine::{Engine, SearchResult};

pub struct HackerNews;

impl Engine for HackerNews {
    fn name(&self) -> &'static str {
        "hackernews"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let page = pageno.clamp(1, 3) - 1;
        format!("https://hn.algolia.com/api/v1/search?query={}&page={page}&hitsPerPage=10", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("hits")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for hit in arr.iter().take(10) {
            let title = hit
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            if title.is_empty() {
                continue;
            }
            let url = hit
                .get("url")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| {
                    hit.get("objectID")
                        .and_then(|x| x.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|id| format!("https://news.ycombinator.com/item?id={id}"))
                })
                .unwrap_or_default();
            if url.is_empty() {
                continue;
            }
            let mut bits = Vec::new();
            if let Some(author) = hit
                .get("author")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                bits.push(format!("by {author}"));
            }
            if let Some(p) = hit.get("points").and_then(|x| x.as_u64()) {
                bits.push(format!("{p} points"));
            }
            if let Some(c) = hit.get("num_comments").and_then(|x| x.as_u64()) {
                bits.push(format!("{c} comments"));
            }
            let mut res = SearchResult::basic("hackernews", url, title.to_string(), bits.join(" · "));
            if let Some(ts) = hit.get("created_at").and_then(|x| x.as_str()).filter(|s| s.len() >= 10) {
                res.published_date = Some(ts[..10].into());
            }
            out.push(res);
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
