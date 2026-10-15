//! NuGet (.NET) package search — keyless JSON (DEV-SEARCH queue #2).
//! `GET azuresearch-usnc.nuget.org/query?q=..&take=10` →
//! `{data:[{id, version, description, projectUrl, ...}], totalHits}`.
//! URL prefers `projectUrl` when http(s), else the registry page
//! `https://www.nuget.org/packages/{id}/{version}`. Description capped
//! at 400 chars (nvd pattern). Template `packages.html`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Nuget;

impl Engine for Nuget {
    fn name(&self) -> &'static str {
        "nuget"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!("https://azuresearch-usnc.nuget.org/query?q={}&take=10", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.get("data").and_then(|x| x.as_array()).unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let id = g.get("id")?.as_str()?.trim();
                if id.is_empty() {
                    return None;
                }
                let ver = g.get("version").and_then(|x| x.as_str()).unwrap_or("");
                let proj = g.get("projectUrl").and_then(|x| x.as_str()).unwrap_or("");
                let url = if proj.starts_with("http") {
                    proj.to_string()
                } else if ver.is_empty() {
                    format!("https://www.nuget.org/packages/{id}")
                } else {
                    format!("https://www.nuget.org/packages/{id}/{ver}")
                };
                let title = if ver.is_empty() {
                    id.to_string()
                } else {
                    format!("{id} {ver}")
                };
                let desc = g.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let mut chars: String = desc.chars().take(400).collect();
                if desc.chars().count() > 400 {
                    chars.push('…');
                }
                let mut res = SearchResult::basic(
                    "nuget",
                    url,
                    title,
                    chars.split_whitespace().collect::<Vec<_>>().join(" "),
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
