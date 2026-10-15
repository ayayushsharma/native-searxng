//! RubyGems search — clean keyless JSON (queue #9, packages depth).
//! `GET /api/v1/search.json?query=..` → up to 30 `[{name, version, info,
//! project_uri, downloads}]`. Single GET, template `packages.html` like
//! npm/crates, version folded into the title (`name version`).

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Rubygems;

impl Engine for Rubygems {
    fn name(&self) -> &'static str {
        "rubygems"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://rubygems.org/api/v1/search.json?query={}", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.as_array().unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let name = g.get("name")?.as_str()?.trim();
                let url = g.get("project_uri")?.as_str()?.trim();
                if name.is_empty() || url.is_empty() {
                    return None;
                }
                let version = g.get("version").and_then(|x| x.as_str()).unwrap_or("");
                let title = if version.is_empty() {
                    name.to_string()
                } else {
                    format!("{name} {version}")
                };
                let info = g.get("info").and_then(|x| x.as_str()).unwrap_or("");
                let mut res = SearchResult::basic(
                    "rubygems",
                    url.to_string(),
                    title,
                    info.split_whitespace().collect::<Vec<_>>().join(" "),
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
