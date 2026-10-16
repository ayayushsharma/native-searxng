//! GitHub repo search (`api.github.com`) — keyless JSON API, it.
//! `https://api.github.com/search/repositories?sort=stars&order=desc&q=..`
//! Response: `{"items": [{html_url, full_name, language, description,
//! owner{login, avatar_url}, updated_at, ...}]}`.
//! Fixture: `tests/fixtures/tests-fixture-github-test.json` (HTTP 200, verified clean).
//! Note: unauthenticated rate limit (60 req/h) — failures surface as
//! `unresponsive_engines`, never silent empties.

use crate::engine::{Engine, SearchResult};

pub struct Github;

impl Engine for Github {
    fn name(&self) -> &'static str {
        "github"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://api.github.com/search/repositories?sort=stars&order=desc&q={}&per_page=10&page={}",
            encode(q),
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
                r.get("html_url").and_then(|x| x.as_str()),
                r.get("full_name").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if url.is_empty() || title.is_empty() {
                continue;
            }
            let mut bits = Vec::new();
            for k in ["language", "description"] {
                if let Some(s) = r.get(k).and_then(|x| x.as_str()) {
                    if !s.is_empty() {
                        bits.push(s);
                    }
                }
            }
            let mut res = SearchResult::basic(
                "github",
                url.into(),
                title.into(),
                bits.join(" / "),
            );
            res.template = "packages.html".into();
            if let Some(avatar) = r
                .get("owner")
                .and_then(|o| o.get("avatar_url"))
                .and_then(|x| x.as_str())
            {
                res.thumbnail = avatar.into();
            }
            if let Some(ts) = r
                .get("updated_at")
                .or_else(|| r.get("created_at"))
                .and_then(|x| x.as_str())
            {
                res.published_date = Some(ts.into());
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
