//! pub.dev (Dart/Flutter) package search — keyless JSON (DEV-SEARCH #4).
//! `GET /api/search?q=..` → `{packages:[{package: name}], next}` —
//! names only by design, so results are honest name+URL links to
//! `https://pub.dev/packages/{name}` with empty content. Template
//! `packages.html`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Pubdev;

impl Engine for Pubdev {
    fn name(&self) -> &'static str {
        "pubdev"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://pub.dev/api/search?q={}", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v
            .get("packages")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let name = g.get("package")?.as_str()?.trim();
                if name.is_empty() {
                    return None;
                }
                let mut res = SearchResult::basic(
                    "pubdev",
                    format!("https://pub.dev/packages/{name}"),
                    name.to_string(),
                    String::new(),
                );
                res.template = "packages.html".into();
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
