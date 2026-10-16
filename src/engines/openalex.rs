//! OpenAlex scholarly works — clean keyless JSON (queue #8, replaces dead
//! OpenAIRE). `GET /works?search=..&per-page=10&page=..` → `{"results":
//! [{title, doi, publication_year, authorships, abstract_inverted_index}]}`.
//! Abstracts come as word→positions inverted indexes; `abstract()` rebuilds
//! the first ~60 words. URL prefers the DOI, falls back to the OpenAlex id.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct OpenAlex;

impl Engine for OpenAlex {
    fn name(&self) -> &'static str {
        "openalex"
    }

    fn category(&self) -> &'static str {
        "science"
    }

    /// Works carry inverted abstracts (~25KB each); 10 fit in ~300KB.
    fn body_cap_kb(&self) -> Option<usize> {
        Some(300)
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "https://api.openalex.org/works?search={}&per-page=10&page={pageno}",
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
            .get("results")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|r| {
                let url = r
                    .get("doi")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| r.get("id").and_then(|x| x.as_str()))
                    .unwrap_or("")
                    .trim();
                if url.is_empty() {
                    return None;
                }
                let title = r.get("title").and_then(|x| x.as_str()).unwrap_or("");
                if title.trim().is_empty() || title == "null" {
                    return None;
                }
                let year = r
                    .get("publication_year")
                    .and_then(|x| x.as_u64())
                    .map(|y| y.to_string())
                    .unwrap_or_default();
                let content = match abstract_text(r) {
                    Some(a) => a,
                    None => year.clone(),
                };
                let content = if year.is_empty() || content.contains(&year) {
                    content
                } else {
                    format!("{year} — {content}")
                };
                Some(SearchResult::basic(
                    "openalex",
                    url.to_string(),
                    title.trim().to_string(),
                    content,
                ))
            })
            .take(10)
            .collect()
    }
}

/// Rebuild text from `abstract_inverted_index: {word: [positions]}`,
/// first 60 words (~abstract lead). None when absent (paywalled/none).
fn abstract_text(work: &Value) -> Option<String> {
    let idx = work.get("abstract_inverted_index")?.as_object()?;
    if idx.is_empty() {
        return None;
    }
    let mut max = 0usize;
    for pos in idx.values().filter_map(|v| v.as_array()) {
        for p in pos.iter().filter_map(|x| x.as_u64()) {
            max = max.max(p as usize);
        }
    }
    if max == 0 && idx.is_empty() {
        return None;
    }
    let mut words: Vec<&str> = vec![""; max + 1];
    for (w, pos) in idx {
        let Some(arr) = pos.as_array() else { continue };
        for p in arr.iter().filter_map(|x| x.as_u64()) {
            if (p as usize) <= max {
                words[p as usize] = w.as_str();
            }
        }
    }
    let text = words
        .into_iter()
        .take(60)
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return None;
    }
    Some(text)
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
