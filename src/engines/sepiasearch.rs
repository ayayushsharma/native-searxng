//! SepiaSearch (PeerTube video search) — keyless JSON (new-category #1).
//! `GET sepiasearch.org/api/v1/search/videos?search=..&start=..&count=10`
//! `&sort=-match&nsfw=false` → `{data:[{url, name, description, account,`
//! `channel, tags, duration, views, publishedAt, embedUrl,`
//! `thumbnailUrl|previewUrl}], total}`. Shape mirrors upstream
//! `peertube.video_response` (template videos.html, iframe_src=embedUrl,
//! thumbnail). `nsfw=false` pinned (== upstream default at safesearch 1;
//! `0→both` unported — safesearch plumbing doesn't reach build_url).
//! Language/time-range params skipped (MVP, like the rest of the fleet).
//! Fixture: `tests/fixtures/tests-fixture-sepiasearch-test.json` (HTTP 200, clean).

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct SepiaSearch;

impl Engine for SepiaSearch {
    fn name(&self) -> &'static str {
        "sepiasearch"
    }

    fn category(&self) -> &'static str {
        "videos"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://sepiasearch.org/api/v1/search/videos?search={}&start={}&count=10&sort=-match&nsfw=false",
            encode(q),
            pageno.saturating_sub(1) * 10,
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let arr = v.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let url = r.get("url").and_then(|x| x.as_str()).unwrap_or("");
            let title = r.get("name").and_then(|x| x.as_str()).unwrap_or("").trim();
            if url.is_empty() || title.is_empty() {
                continue;
            }
            // Upstream metadata line: channel display, name@host, tags.
            let chan = r.get("channel").and_then(|c| c.as_object());
            let meta: Vec<String> = [
                chan.and_then(|c| c.get("displayName")).and_then(|x| x.as_str()),
                r.get("account").and_then(|a| a.get("displayName")).and_then(|x| x.as_str()),
            ]
            .into_iter()
            .flatten()
            .map(|s| s.to_string())
            .collect();
            let tags: Vec<String> = r
                .get("tags")
                .and_then(|t| t.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()).collect())
                .unwrap_or_default();
            let desc = r.get("description").and_then(|x| x.as_str()).unwrap_or("");
            let mut content = String::new();
            if !meta.is_empty() {
                content.push_str(&meta.join(" | "));
                content.push_str(" — ");
            }
            content.push_str(&desc.split_whitespace().collect::<Vec<_>>().join(" "));
            if !tags.is_empty() {
                content.push_str(" [");
                content.push_str(&tags.join(", "));
                content.push(']');
            }
            let mut res = SearchResult::basic("sepiasearch", url.to_string(), title.to_string(), content);
            res.template = "videos.html".into();
            res.iframe_src = r.get("embedUrl").and_then(|x| x.as_str()).unwrap_or("").to_string();
            res.thumbnail = r
                .get("thumbnailUrl")
                .and_then(|x| x.as_str())
                .or_else(|| r.get("previewUrl").and_then(|x| x.as_str()))
                .unwrap_or("")
                .to_string();
            if let Some(ts) = r.get("publishedAt").and_then(|x| x.as_str()).filter(|s| s.len() >= 10) {
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
