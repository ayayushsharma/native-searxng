//! GMX search — GET-only session flow (queue #3).
//! Step 1 (build_url): `/web/result?q=..&page=..` (browser-like headers help;
//! oracle warns look-offs get rate-limited fast)
//!   → HTML containing `&h=<hash>&t=` (captcha = `#spam-messages` element).
//! Step 2 (follow_up): `/desk?lang=en&q=..&page=..&h=<hash>&t=<now/10>`
//!   → `{"results": {"hits": [{u, t, s}], "rs": [{t}]}}`.
//! Hash sits ~40KB into the page — inside the 96KB body cap, safe.
//! Captcha element → challenge(); either leg failing → unresponsive.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Gmx;

const BASE: &str = "https://search.gmx.com";

impl Engine for Gmx {
    fn name(&self) -> &'static str {
        "gmx"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!("{BASE}/web/result?q={}&page={pageno}", encode(q))
    }

    fn two_step(&self) -> bool {
        true
    }

    fn follow_up(&self, query: &str, pageno: u32, first_body: &str) -> Option<String> {
        let hash = page_hash(first_body)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() / 10)
            .unwrap_or(0);
        Some(format!(
            "{BASE}/desk?lang=en&q={}&page={pageno}&h={hash}&t={now}",
            encode(query),
        ))
    }

    fn challenge(&self, body: &str) -> Option<String> {
        // Oracle: `//*[@id='spam-messages']` element = captcha. String-match
        // both quote styles; JS string mentions without the element are fine.
        if body.contains("id='spam-messages'") || body.contains("id=\"spam-messages\"") {
            return Some("captcha (spam-messages)".into());
        }
        None
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let hits = v
            .get("results")
            .and_then(|x| x.get("hits"))
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        hits
            .iter()
            .filter_map(|r| {
                let url = r.get("u")?.as_str()?.trim();
                if url.is_empty() {
                    return None;
                }
                Some(SearchResult::basic(
                    "gmx",
                    url.to_string(),
                    strip(r.get("t").and_then(|x| x.as_str()).unwrap_or("")),
                    strip(r.get("s").and_then(|x| x.as_str()).unwrap_or("")),
                ))
            })
            .take(10)
            .collect()
    }
}

/// `&h=<hash>&t=` from the step-1 page (oracle's `extr` twin).
fn page_hash(step1: &str) -> Option<String> {
    let mark = "&h=";
    let i = step1.find(mark)? + mark.len();
    let tail = &step1[i..];
    let j = tail.find("&t=")?;
    let hash = &tail[..j];
    if hash.is_empty() || hash.len() > 64 || !hash.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(hash.to_string())
}

fn strip(s: &str) -> String {
    // Titles/snippets may carry HTML entities/tags (oracle html_to_text).
    let mut out = crate::engine::unescape(s);
    loop {
        let Some(a) = out.find('<') else { break };
        let Some(b) = out[a..].find('>') else { break };
        out.replace_range(a..a + b + 1, " ");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
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
