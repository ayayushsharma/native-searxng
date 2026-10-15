//! Dogpile web search — 2-leg POST chain (queue #5).
//! 1. `POST /api/token/refresh` (cookie `dp_api_token=1`, Origin headers)
//!    → `{"token": "<~240s JWT>"}` (oracle caches it 240s; we mint fresh
//!    per query — correctness first, caching deferred like boardreader).
//! 2. `POST /api/search` JSON `{q, qadf, page}` with `x-dogpile-token` +
//!    cookie → `{"results": [{clickUrl,title,description}]}`.
//! Titles carry `<b>` (oracle html_to_text) — `strip` twin used.

use crate::engine::{Engine, InResponse, Method, OutRequest, SearchResult};

pub struct Dogpile;

const BASE: &str = "https://www.dogpile.com";

impl Engine for Dogpile {
    fn name(&self) -> &'static str {
        "dogpile"
    }

    fn rounds(&self) -> usize {
        2
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        format!("{BASE}/api/token/refresh")
    }

    fn request(&self, query: &str, pageno: u32, prev: &[InResponse]) -> Option<OutRequest> {
        match prev.len() {
            0 => Some(OutRequest {
                url: format!("{BASE}/api/token/refresh"),
                method: Method::Post,
                form: vec![],
                json: None,
                headers: vec![
                    ("Origin".into(), BASE.to_string()),
                    ("Sec-Fetch-Mode".into(), "cors".into()),
                    ("Cookie".into(), "dp_api_token=1".into()),
                ],
            }),
            1 => {
                let token = token(&prev[0].body)?;
                let q = serde_json::Value::String(query.to_string());
                Some(OutRequest {
                    url: format!("{BASE}/api/search"),
                    method: Method::Post,
                    form: vec![],
                    json: Some(format!(
                        "{{\"q\":{q},\"qadf\":\"moderate\",\"page\":{pageno}}}"
                    )),
                    headers: vec![
                        ("Origin".into(), BASE.to_string()),
                        ("Sec-Fetch-Mode".into(), "cors".into()),
                        ("Cookie".into(), "dp_api_token=1".into()),
                        ("x-dogpile-token".into(), token),
                    ],
                })
            }
            _ => None,
        }
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.get("results").and_then(|x| x.as_array()).unwrap_or(&empty);
        rs.iter()
            .filter_map(|r| {
                let url = r.get("clickUrl")?.as_str()?.trim();
                if url.is_empty() {
                    return None;
                }
                Some(SearchResult::basic(
                    "dogpile",
                    url.to_string(),
                    strip(r.get("title").and_then(|x| x.as_str()).unwrap_or("")),
                    strip(r.get("description").and_then(|x| x.as_str()).unwrap_or("")),
                ))
            })
            .take(10)
            .collect()
    }
}

/// `{"token": "<jwt>"}` from leg 1.
fn token(refresh_body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(refresh_body).ok()?;
    let t = v.get("token")?.as_str()?.trim();
    if t.is_empty() {
        return None;
    }
    Some(t.to_string())
}

fn strip(s: &str) -> String {
    let mut out = crate::engine::unescape(s);
    loop {
        let Some(a) = out.find('<') else { break };
        let Some(b) = out[a..].find('>') else { break };
        out.replace_range(a..a + b + 1, " ");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
