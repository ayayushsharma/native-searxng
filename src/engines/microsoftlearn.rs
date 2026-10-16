//! Microsoft Learn docs search (`learn.microsoft.com/api/search`) — it.
//! Keyless JSON API (mirrors upstream `microsoft_learn.py`):
//! `?search=..&locale=en-us&scoringprofile=semantic-answers&facet=..`
//! `&$top=10&$skip=..&expandScope=true&partnerId=LearnSite`
//! Response: `{results: [{title, url (absolute), description,
//! lastUpdatedDate}]}`. Covers Azure + Microsoft 365 + .NET docs —
//! the official-docs slice community engines never surface.
//! Usage policy: public site search API, no key, no auth, 1 req/query.
//! Fixture: `tests/fixtures/tests-fixture-microsoftlearn-test.json` (HTTP 200, clean).

use crate::engine::{Engine, SearchResult};

pub struct MicrosoftLearn;

impl Engine for MicrosoftLearn {
    fn name(&self) -> &'static str {
        "microsoftlearn"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let skip = 10 * (pageno.clamp(1, 3) - 1);
        format!(
            "https://learn.microsoft.com/api/search?search={}&locale=en-us&scoringprofile=semantic-answers&facet=category&facet=products&facet=tags&$top=10&$skip={skip}&expandScope=true&includeQuestion=false&applyOperator=false&partnerId=LearnSite",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("results")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for item in arr.iter().take(10) {
            let title = item
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            let url = item
                .get("url")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            if title.is_empty() || url.is_empty() {
                continue;
            }
            let content = item
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let mut res = SearchResult::basic(
                "microsoftlearn",
                url.to_string(),
                title.to_string(),
                content,
            );
            if let Some(ts) = item
                .get("lastUpdatedDate")
                .and_then(|x| x.as_str())
                .filter(|s| s.len() >= 10)
            {
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
