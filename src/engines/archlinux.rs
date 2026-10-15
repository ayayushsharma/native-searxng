//! Arch Linux wiki via the MediaWiki full-text JSON API — it.
//! `https://wiki.archlinux.org/api.php?action=query&list=search&srsearch=..`
//! Response: `{query: {search: [{title, snippet (HTML highlights),
//! timestamp}]}}`. Snippets are de-tagged; titles link to
//! `https://wiki.archlinux.org/title/{title}`.
//! (opensearch was tried first: title-prefix only, empty on ordinary
//! multi-word queries — full-text matches the reference engine.)
//! Usage policy (checked 2026-10-15): robots.txt disallows `/index.php?`
//! but NOT `api.php` — this path is robots-clean; wiki content freely
//! licensed, 1 req/query.
//! Fixture: `tests-fixture-archlinux-test.json` (HTTP 200, clean).

use crate::engine::{Engine, SearchResult, strip_tags};

pub struct ArchLinux;

impl Engine for ArchLinux {
    fn name(&self) -> &'static str {
        "archlinux"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://wiki.archlinux.org/api.php?action=query&list=search&srsearch={}&srlimit=10&format=json",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("query")
            .and_then(|q| q.get("search"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for hit in arr.iter().take(10) {
            let title = hit.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
            if title.is_empty() {
                continue;
            }
            let url = format!(
                "https://wiki.archlinux.org/title/{}",
                title.replace(' ', "_")
            );
            let content = hit
                .get("snippet")
                .and_then(|x| x.as_str())
                .map(|s| {
                    // Drop `[[xx: Title]]` interlanguage links + `#redirect`
                    // markers the API prepends; keep readable text.
                    let mut t = strip_tags(s);
                    while let Some(i) = t.find("[[") {
                        let end = t[i..].find("]]").map(|e| i + e + 2).unwrap_or(t.len());
                        t.replace_range(i..end, " ");
                    }
                    t.split_whitespace().collect::<Vec<_>>().join(" ")
                })
                .unwrap_or_default();
            let mut res = SearchResult::basic("archlinux", url, title.to_string(), content);
            if let Some(ts) = hit
                .get("timestamp")
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
