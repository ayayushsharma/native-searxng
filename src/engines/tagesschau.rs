//! tagesschau (German news) — keyless JSON (survey #5, news depth).
//! `GET www.tagesschau.de/api2u/search?searchText=..&resultPage=..` →
//! `{searchResults:[{title, content:[{value}], date, detailsweb,
//! shareURL, sophoraId...}], totalItemCount}`. Fat JSON (477KB) needs
//! `body_cap_kb` 512 (openalex pattern; user-softened cap 2026-10-16).
//! URL prefers `detailsweb` (tagesschau.de canonical), falls back to
//! `shareURL` (regional affiliates). Content joins `content[].value`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Tagesschau;

impl Engine for Tagesschau {
    fn name(&self) -> &'static str {
        "tagesschau"
    }

    fn category(&self) -> &'static str {
        "news"
    }

    fn body_cap_kb(&self) -> Option<usize> {
        Some(512)
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://www.tagesschau.de/api2u/search?searchText={}&resultPage={}",
            encode(q),
            pageno.saturating_sub(1)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v
            .get("searchResults")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let title = g.get("title")?.as_str()?.trim();
                if title.is_empty() {
                    return None;
                }
                let web = g.get("detailsweb").and_then(|x| x.as_str()).unwrap_or("");
                let share = g.get("shareURL").and_then(|x| x.as_str()).unwrap_or("");
                let url = if web.starts_with("http") {
                    web
                } else if share.starts_with("http") {
                    share
                } else {
                    return None;
                };
                let content = g
                    .get("content")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|c| c.get("value").and_then(|x| x.as_str()))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let mut chars: String = content.chars().take(400).collect();
                if content.chars().count() > 400 {
                    chars.push('…');
                }
                let mut res = SearchResult::basic(
                    "tagesschau",
                    url.to_string(),
                    title.to_string(),
                    chars.split_whitespace().collect::<Vec<_>>().join(" "),
                );
                if let Some(date) = g.get("date").and_then(|x| x.as_str()) {
                    if date.len() >= 10 {
                        res.published_date = Some(date[..10].to_string());
                    }
                }
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
