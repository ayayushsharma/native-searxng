//! Packagist (PHP) search — keyless JSON (DEV-SEARCH queue #1).
//! `GET /search.json?q=..` → `{results:[{name, description, url,
//! repository, downloads, favers}], total}`. Single GET, template
//! `packages.html`, downloads folded into content tail.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Packagist;

impl Engine for Packagist {
    fn name(&self) -> &'static str {
        "packagist"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://packagist.org/search.json?q={}", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.get("results").and_then(|x| x.as_array()).unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let name = g.get("name")?.as_str()?.trim();
                let url = g.get("url")?.as_str()?.trim();
                if name.is_empty() || url.is_empty() {
                    return None;
                }
                let desc = g.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let dl = g.get("downloads").and_then(|x| x.as_u64()).unwrap_or(0);
                let content = if dl > 0 {
                    format!("{desc} — {dl} downloads")
                } else {
                    desc.to_string()
                };
                let mut res = SearchResult::basic(
                    "packagist",
                    url.to_string(),
                    name.to_string(),
                    content.split_whitespace().collect::<Vec<_>>().join(" "),
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
