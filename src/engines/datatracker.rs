//! IETF Datatracker (RFC/standards) search — keyless Tastypie JSON
//! (DEV-SEARCH queue #5). `GET /api/v1/doc/document/?name__icontains=..
//! &limit=10` → `{objects:[{name, title, abstract, ...}], meta}` (DRF
//! pagination envelope, not a bare array). URL is the canonical doc
//! page `https://datatracker.ietf.org/doc/{name}/`; content prefers the
//! abstract, falls back to the title. Category `it` (standards are dev
//! reference, like NVD).

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Datatracker;

impl Engine for Datatracker {
    fn name(&self) -> &'static str {
        "datatracker"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://datatracker.ietf.org/api/v1/doc/document/?name__icontains={}&limit=10",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v
            .get("objects")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let name = g.get("name")?.as_str()?.trim();
                if name.is_empty() {
                    return None;
                }
                let title = g.get("title").and_then(|x| x.as_str()).unwrap_or("");
                let abs = g.get("abstract").and_then(|x| x.as_str()).unwrap_or("");
                let content = if abs.trim().is_empty() { title } else { abs };
                let mut chars: String = content.chars().take(400).collect();
                if content.chars().count() > 400 {
                    chars.push('…');
                }
                Some(SearchResult::basic(
                    "datatracker",
                    format!("https://datatracker.ietf.org/doc/{name}/"),
                    if title.trim().is_empty() {
                        name.to_string()
                    } else {
                        title.trim().to_string()
                    },
                    chars.split_whitespace().collect::<Vec<_>>().join(" "),
                ))
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
