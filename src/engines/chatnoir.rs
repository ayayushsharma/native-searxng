//! ChatNoir web search — 3-leg chain (queue #4, first POST engine).
//! 1. `GET /` → `Set-Cookie: csrftoken=…` (homepage is a 2KB shell).
//! 2. `POST /?init` with `X-Csrf-Token` + cookies → `{"token":{"token"}}`.
//! 3. `POST /api/v1/_search` JSON `{query,index,from,size}` with
//!    `Authorization: Bearer <token>` + `X-Csrf-Token` → `{"results":
//!    [{target_uri,title,snippet}]}` (10 per `size`).
//! The chain is stateless: leg 3 re-reads the csrf from leg 1's cookies
//! (via the shared `prev` slice) and the token from leg 2's body.
//! Oracle parity: `impersonate="none"` is its default TLS posture — ours
//! is plain ureq either way; `index=["cw22"]`, `size=10`, `_extended_meta`
//! are matched exactly. A user-supplied `SEARXNG_API_KEY_CHATNOIR` skips
//! legs 1-2 (oracle's `api_key` branch) — unset means scrape (default).

use crate::engine::{Engine, InResponse, Method, OutRequest, SearchResult, cookie};
use serde_json::Value;

pub struct ChatNoir;

const BASE: &str = "https://www.chatnoir.eu";

impl Engine for ChatNoir {
    fn name(&self) -> &'static str {
        "chatnoir"
    }

    fn rounds(&self) -> usize {
        3
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        format!("{BASE}/")
    }

    fn request(&self, query: &str, pageno: u32, prev: &[InResponse]) -> Option<OutRequest> {
        match prev.len() {
            0 => Some(OutRequest::get(format!("{BASE}/"))),
            1 => {
                // Leg 2: mint a Bearer token via the csrf from leg 1.
                let csrf = cookie(&prev[0].cookies, "csrftoken")?;
                Some(OutRequest {
                    url: format!("{BASE}/?init"),
                    method: Method::Post,
                    form: vec![],
                    json: None,
                    headers: vec![
                        ("Referer".into(), format!("{BASE}/")),
                        ("X-Requested-With".into(), "XMLHttpRequest".into()),
                        ("X-Csrf-Token".into(), csrf),
                    ],
                })
            }
            2 => {
                let csrf = cookie(&prev[0].cookies, "csrftoken").unwrap_or_default();
                // Env key wins when set (oracle's `api_key` branch);
                // otherwise the token minted in leg 2. Chain stays 3 legs
                // either way, so `rounds` never lies.
                let token = env_key().or_else(|| token(&prev[1].body))?;
                Some(search_req(query, pageno, &token, &csrf))
            }
            _ => None,
        }
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.get("results").and_then(|x| x.as_array()).unwrap_or(&empty);
        rs.iter()
            .filter_map(|r| {
                let url = r.get("target_uri")?.as_str()?.trim();
                if url.is_empty() {
                    return None;
                }
                Some(SearchResult::basic(
                    "chatnoir",
                    url.to_string(),
                    strip(r.get("title").and_then(|x| x.as_str()).unwrap_or("")),
                    strip(r.get("snippet").and_then(|x| x.as_str()).unwrap_or("")),
                ))
            })
            .take(10)
            .collect()
    }
}

/// Leg-3 search POST. `from` pages in tens like the oracle
/// (`(pageno-1) * 10`).
fn search_req(query: &str, pageno: u32, token: &str, csrf: &str) -> OutRequest {
    let from = pageno.saturating_sub(1) * 10;
    let body = format!(
        "{{\"query\":{},\"index\":[\"cw22\"],\"from\":{from},\"size\":10,\"_extended_meta\":true}}",
        serde_json::Value::String(query.to_string()),
    );
    OutRequest {
        url: format!("{BASE}/api/v1/_search"),
        method: Method::Post,
        form: vec![],
        json: Some(body),
        headers: vec![
            ("Authorization".into(), format!("Bearer {token}")),
            ("X-Csrf-Token".into(), csrf.to_string()),
        ],
    }
}

/// `SEARXNG_API_KEY_CHATNOIR` when set (never logged, never in errors).
fn env_key() -> Option<String> {
    std::env::var("SEARXNG_API_KEY_CHATNOIR")
        .ok()
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
}

/// `{"token": {"token": "<bearer>"}}` from leg 2.
fn token(init_body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(init_body).ok()?;
    let t = v.get("token")?.get("token")?.as_str()?.trim();
    if t.is_empty() {
        return None;
    }
    Some(t.to_string())
}

/// Titles/snippets carry `<em>` + entities (oracle html_to_text).
fn strip(s: &str) -> String {
    let mut out = crate::engine::unescape(s);
    loop {
        let Some(a) = out.find('<') else { break };
        let Some(b) = out[a..].find('>') else { break };
        out.replace_range(a..a + b + 1, " ");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
