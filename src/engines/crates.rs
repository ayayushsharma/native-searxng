//! crates.io (`crates.io/api`) — keyless JSON API, packages.
//! `https://crates.io/api/v1/crates?q=..&per_page=10`
//! Response: `{"crates": [{name, description, updated_at, downloads,
//! homepage, repository, ...}]}`.
//! Fixture: `tests/fixtures/tests-fixture-crates-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Crates;

impl Engine for Crates {
    fn name(&self) -> &'static str {
        "crates"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let page = pageno.clamp(1, 3);
        format!("https://crates.io/api/v1/crates?q={}&per_page=10&page={page}", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("crates")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for package in arr.iter().take(10) {
            let Some(name) = package.get("name").and_then(|x| x.as_str()) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let mut bits = Vec::new();
            if let Some(desc) = package
                .get("description")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                bits.push(desc.to_string());
            }
            if let Some(dl) = package.get("downloads").and_then(|x| x.as_u64()) {
                bits.push(format!("{dl} downloads"));
            }
            let mut res = SearchResult::basic(
                "crates",
                format!("https://crates.io/crates/{name}"),
                name.into(),
                bits.join(" — "),
            );
            res.template = "packages.html".into();
            if let Some(date) = package
                .get("updated_at")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                res.published_date = Some(date.into());
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
