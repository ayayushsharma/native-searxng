//! Boardreader (forum search) — session-flow engine (queue #1).
//! Step 1 (build_url): homepage `https://boardreader.com`
//!   → contains `localStorage.setItem('currentSessionId', '<id>')`.
//! Step 2 (request, prev.len()==1): `/return.php?query=..&page=..&language=All&session_id=<id>`
//!   → `{"SearchResults": [{Subject, Text, Url, Published, Author}]}`.
//! `Subject`/`Text` carry `[Keyword]…[/Keyword]` markers — stripped.
//! Language pinned to `All` (oracle defaults per-locale via traits; MVP
//! serves the union). Either leg failing → unresponsive, never silent.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Boardreader;

const BASE: &str = "https://boardreader.com";

impl Engine for Boardreader {
    fn name(&self) -> &'static str {
        "boardreader"
    }

    fn rounds(&self) -> usize {
        2
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        BASE.into()
    }

    fn request(
        &self,
        query: &str,
        pageno: u32,
        prev: &[crate::engine::InResponse],
    ) -> Option<crate::engine::OutRequest> {
        if prev.is_empty() {
            return Some(crate::engine::OutRequest::get(
                self.build_url(query, pageno),
            ));
        }
        if prev.len() > 1 {
            return None;
        }
        let sid = session_id(&prev[0].body)?;
        Some(crate::engine::OutRequest::get(format!(
            "{BASE}/return.php?query={}&page={pageno}&language=All&session_id={sid}",
            encode(query),
        )))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let list = v
            .get("SearchResults")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        list.iter()
            .filter_map(|r| {
                let url = r.get("Url")?.as_str()?.trim();
                if url.is_empty() {
                    return None;
                }
                let author = r.get("Author").and_then(|x| x.as_str()).unwrap_or("");
                let mut row = SearchResult::basic(
                    "boardreader",
                    url.to_string(),
                    strip_kw(r.get("Subject").and_then(|x| x.as_str()).unwrap_or("")),
                    strip_kw(r.get("Text").and_then(|x| x.as_str()).unwrap_or("")),
                );
                row.published_date = r
                    .get("Published")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string());
                if !author.is_empty() {
                    row.content = format!("{} — Posted by {author}", row.content);
                }
                Some(row)
            })
            .take(10)
            .collect()
    }
}

/// `'currentSessionId', '<id>'` from the homepage (oracle's `extr` twin).
fn session_id(home: &str) -> Option<String> {
    let mark = "'currentSessionId', '";
    let i = home.find(mark)? + mark.len();
    let tail = &home[i..];
    let j = tail.find('\'')?;
    let sid = &tail[..j];
    if sid.len() < 8 {
        return None;
    }
    Some(sid.to_string())
}

fn strip_kw(s: &str) -> String {
    s.replace("[Keyword]", "")
        .replace("[/Keyword]", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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
