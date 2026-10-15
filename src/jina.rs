//! Jina Reader (`r.jina.ai`) — page extraction backend for `/fetch`.
//! GET `https://r.jina.ai/<target-url>` with `Accept: application/json`
//! returns `{"code":200, "data": {title, description, content, links}}`.
//! Free tier works keyless (rate-limited); `SEARXNG_API_KEY_JINA` as
//! `Authorization: Bearer <key>` raises limits. Key material never flows
//! into errors or logs — only into the outbound header.
//! Replaces the `web_fetch` half of pi-ollama-web-search (see report).

use crate::{config, fetch::Client};

pub struct FetchResult {
    pub title: String,
    pub content: String,
    pub links: Vec<String>,
}

/// Extract `target` via Reader. `Ok` on Reader code 200, `Err(reason)`
/// otherwise (caller maps to `{"error":…}` / unresponsive, never a key).
pub fn fetch(client: &Client, target: &str) -> Result<FetchResult, String> {
    if !target.starts_with("http://") && !target.starts_with("https://") {
        return Err("fetch needs an http(s) URL".into());
    }
    let url = format!("https://r.jina.ai/{target}");
    let mut headers = vec![("Accept", "application/json")];
    let key: Option<String> = config::api_key("jina");
    let bearer;
    if let Some(k) = key.as_ref() {
        bearer = format!("Bearer {k}");
        headers.push(("Authorization", bearer.as_str()));
    }
    let body = client.get_with(&url, &headers).map_err(|e| short_err(&e))?;
    parse(&body).ok_or_else(|| "reader returned no usable content".into())
}

/// Pure parse (offline-testable): JSON envelope first, plain-text fallback.
pub fn parse(body: &str) -> Option<FetchResult> {
    let t = body.trim();
    if t.starts_with('{') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
            let code = v.get("code").and_then(|x| x.as_u64()).unwrap_or(0);
            if code == 200 {
                let d = v.get("data")?;
                let content = d
                    .get("content")
                    .and_then(|x| x.as_str())
                    .unwrap_or_default();
                if content.trim().is_empty() {
                    return None;
                }
                let links: Vec<String> = d
                    .get("links")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .take(50)
                            .collect()
                    })
                    .unwrap_or_default();
                return Some(FetchResult {
                    title: d
                        .get("title")
                        .and_then(|x| x.as_str())
                        .unwrap_or_default()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    content: content.to_string(),
                    links,
                });
            }
            return None;
        }
    }
    // Plain-text/markdown fallback (Reader default without Accept).
    let text = t.trim().to_string();
    if text.len() < 20 {
        return None;
    }
    let title = text.lines().next().unwrap_or_default().trim_start_matches("# ").to_string();
    Some(FetchResult {
        title,
        content: text,
        links: vec![],
    })
}

/// Scrub upstream errors: status text only, never bodies (may echo the
/// target URL) or headers (key material).
fn short_err(e: &str) -> String {
    let first = e.lines().next().unwrap_or("fetch failed");
    first.chars().take(120).collect()
}
