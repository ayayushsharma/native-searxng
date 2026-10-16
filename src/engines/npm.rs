//! npm (`api.npms.io`) — keyless JSON API, packages.
//! `https://api.npms.io/v2/search?q=..&size=10`
//! Response: `{"results": [{package: {name, version, description,
//! links{npm, homepage, repository}, author{name}, date, keywords}}]}`.
//! Fixture: `tests/fixtures/tests-fixture-npm-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Npm;

impl Engine for Npm {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let _ = pageno;
        format!("https://api.npms.io/v2/search?q={}&size=10", encode(q))
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
        for entry in arr.iter().take(10) {
            let Some(pkg) = entry.get("package") else {
                continue;
            };
            let (Some(name), Some(npm_url)) = (
                pkg.get("name").and_then(|x| x.as_str()),
                pkg.get("links")
                    .and_then(|l| l.get("npm"))
                    .and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if name.is_empty() || npm_url.is_empty() {
                continue;
            }
            let desc = pkg
                .get("description")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            let mut res = SearchResult::basic(
                "npm",
                npm_url.into(),
                name.into(),
                desc.split_whitespace().collect::<Vec<_>>().join(" "),
            );
            res.template = "packages.html".into();
            if let Some(date) = pkg
                .get("date")
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
