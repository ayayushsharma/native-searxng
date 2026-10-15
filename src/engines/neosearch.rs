//! Neosearch — 2-leg xsrf session + POST (queue #14, LAST).
//! 1. `GET /` → `<meta name="xsrf-token" content="...">` (64 hex).
//! 2. `POST /search` JSON `{q, generate: auto}` with `X-XSRF-TOKEN` →
//!    multi-line body whose LAST line is `{"lenses": [{categories:
//!    [{links: [{url, title, snippet/description}]}]}], suggestions: []}`.
//! Suggestions are dropped (no engine-suggestion slot in the contract;
//! query-level `suggestions` come from spellcheck, not engines).

use crate::engine::{Engine, InResponse, Method, OutRequest, SearchResult};
use serde_json::Value;

pub struct Neosearch;

const BASE: &str = "https://neosearch.org";

impl Engine for Neosearch {
    fn name(&self) -> &'static str {
        "neosearch"
    }

    fn rounds(&self) -> usize {
        2
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        BASE.to_string()
    }

    fn request(&self, query: &str, _pageno: u32, prev: &[InResponse]) -> Option<OutRequest> {
        match prev.len() {
            0 => Some(OutRequest {
                url: BASE.to_string(),
                method: Method::Get,
                form: vec![],
                json: None,
                headers: vec![],
            }),
            1 => {
                let token = xsrf(&prev[0].body)?;
                let q = Value::String(query.to_string());
                Some(OutRequest {
                    url: format!("{BASE}/search"),
                    method: Method::Post,
                    form: vec![],
                    json: Some(format!("{{\"q\":{q},\"generate\":\"auto\"}}")),
                    headers: vec![("X-XSRF-TOKEN".into(), token)],
                })
            }
            _ => None,
        }
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let last = body.split('\n').filter(|l| !l.trim().is_empty()).last().unwrap_or("");
        let v: Value = match serde_json::from_str(last) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let no_lens = vec![];
        let lenses = v.get("lenses").and_then(|x| x.as_array()).unwrap_or(&no_lens);
        let mut out = vec![];
        for lens in lenses {
            let no_cats = vec![];
            let cats = lens.get("categories").and_then(|x| x.as_array()).unwrap_or(&no_cats);
            for cat in cats {
                let no_links = vec![];
                let links = cat.get("links").and_then(|x| x.as_array()).unwrap_or(&no_links);
                for link in links {
                    let url = link.get("url").and_then(|x| x.as_str()).unwrap_or("").trim();
                    let title = link.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
                    if url.is_empty() || title.is_empty() {
                        continue;
                    }
                    let snip = link.get("snippet").and_then(|x| x.as_str()).unwrap_or("");
                    let desc = link.get("description").and_then(|x| x.as_str()).unwrap_or("");
                    let content = if snip.trim().is_empty() { desc } else { snip };
                    out.push(SearchResult::basic(
                        "neosearch",
                        url.to_string(),
                        title.to_string(),
                        content.split_whitespace().collect::<Vec<_>>().join(" "),
                    ));
                    if out.len() >= 10 {
                        return out;
                    }
                }
            }
        }
        out
    }
}

/// `<meta name="xsrf-token" content="...">` from the home page (either
/// quote style; first hit wins; empty → None so the chain stops honestly).
fn xsrf(home_body: &str) -> Option<String> {
    for name in ["name=\"xsrf-token\"", "name='xsrf-token'"] {
        let Some(i) = home_body.find(name) else { continue };
        let rest = &home_body[i + name.len()..];
        let Some(c) = rest.find("content=") else { continue };
        let val = rest[c + 8..].trim_start_matches(['"', '\'']);
        let tok: String = val
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if !tok.is_empty() {
            return Some(tok);
        }
    }
    None
}
