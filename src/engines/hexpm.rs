//! hex.pm (Elixir) package search — keyless JSON (DEV-SEARCH queue #4).
//! `GET /api/packages?search=..&per_page=10` → `[{name, meta:{description,
//! links}, url(api URL), downloads, ...}]`. Page URL assembled as
//! `https://hex.pm/packages/{name}` (the `url` field points at the API).
//! Template `packages.html`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Hex;

impl Engine for Hex {
    fn name(&self) -> &'static str {
        "hexpm"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://hex.pm/api/packages?search={}&per_page=10",
            encode(q)
        )
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
                if name.is_empty() {
                    return None;
                }
                let desc = g
                    .get("meta")
                    .and_then(|m| m.get("description"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                let mut res = SearchResult::basic(
                    "hexpm",
                    format!("https://hex.pm/packages/{name}"),
                    name.to_string(),
                    desc.split_whitespace().collect::<Vec<_>>().join(" "),
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
