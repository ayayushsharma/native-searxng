//! StackExchange sites (`api.stackexchange.com`) — keyless JSON API, it.
//! One parser, three engine names via the `site` parameter: stackoverflow,
//! superuser, askubuntu (all verified clean, HTTP 200).
//! `.../2.3/search/advanced?q=..&site=..&pagesize=10&page=..`
//! Response: `{"items": [{link, title, tags[], owner{}, is_answered, score}]}`.
//! Fixture: `tests/fixtures/tests-fixture-stackexchange-test.json` (site=stackoverflow).

use crate::engine::{unescape, Engine, SearchResult};

pub struct StackExchange {
    site: &'static str,
}

impl StackExchange {
    pub fn new(site: &'static str) -> Self {
        Self { site }
    }
}

impl Engine for StackExchange {
    fn name(&self) -> &'static str {
        match self.site {
            "superuser" => "superuser",
            "askubuntu" => "askubuntu",
            _ => "stackoverflow",
        }
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://api.stackexchange.com/2.3/search/advanced?q={}&site={}&pagesize=10&page={}&order=desc&sort=activity",
            encode(q),
            self.site,
            pageno.clamp(1, 2)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("items")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let (Some(url), Some(title)) = (
                r.get("link").and_then(|x| x.as_str()),
                r.get("title").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if url.is_empty() || title.is_empty() {
                continue;
            }
            // Mirror SearXNG content: "[tag1, tag2] owner // answered // score".
            let mut content = String::from("[");
            if let Some(tags) = r.get("tags").and_then(|x| x.as_array()) {
                let ts: Vec<&str> = tags.iter().filter_map(|t| t.as_str()).collect();
                content.push_str(&ts.join(", "));
            }
            content.push(']');
            if let Some(owner) = r
                .get("owner")
                .and_then(|o| o.get("display_name"))
                .and_then(|x| x.as_str())
            {
                content.push(' ');
                content.push_str(owner);
            }
            if r.get("is_answered")
                .and_then(|x| x.as_bool())
                .unwrap_or(false)
            {
                content.push_str(" // is answered");
            }
            if let Some(score) = r.get("score").and_then(|x| x.as_i64()) {
                content.push_str(&format!(" // score: {score}"));
            }
            out.push(SearchResult::basic(
                self.name(),
                url.into(),
                unescape(title),
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
