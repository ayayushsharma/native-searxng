//! Jisho (Japanese-English dictionary) — keyless JSON (new-category #2).
//! `GET jisho.org/api/v1/search/words?keyword=..` → `{data:[{slug,`
//! `japanese:[{word?, reading}], senses:[{parts_of_speech,`
//! `english_definitions, tags, info, restrictions}]}]}`. Mirrors upstream:
//! title = alt forms (`word (reading)` or bare reading), url =
//! `jisho.org/word/{slug}`, content = `; `-joined definitions truncated
//! to 300 chars + `...`. Wikipedia-definition entries are kept (upstream
//! `pass`es the filter — no skip). Truncation is on char boundaries:
//! byte-slicing would split CJK text. Upstream's first-result infobox
//! is skipped (no infobox pipeline — cheatsh precedent: results only).
//! Fixture: `tests-fixture-jisho-test.json` (HTTP 200, clean).

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Jisho;

impl Engine for Jisho {
    fn name(&self) -> &'static str {
        "jisho"
    }

    fn category(&self) -> &'static str {
        "dictionaries"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        // paging=False upstream: pageno ignored.
        format!("https://jisho.org/api/v1/search/words?keyword={}", encode(q))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let arr = v.get("data").and_then(|d| d.as_array()).unwrap_or(&empty);
        let mut out = Vec::new();
        for page in arr.iter().take(10) {
            // Alternative forms for the title.
            let mut alt: Vec<String> = Vec::new();
            if let Some(jap) = page.get("japanese").and_then(|j| j.as_array()) {
                for t in jap {
                    let word = t.get("word").and_then(|x| x.as_str());
                    let reading = t.get("reading").and_then(|x| x.as_str());
                    match (word, reading) {
                        (Some(w), Some(r)) => alt.push(format!("{w} ({r})")),
                        (Some(w), None) => alt.push(w.to_string()),
                        (None, Some(r)) => alt.push(r.to_string()),
                        (None, None) => {}
                    }
                }
            }
            if alt.is_empty() {
                continue;
            }
            let slug = page.get("slug").and_then(|x| x.as_str()).unwrap_or("");
            if slug.is_empty() {
                continue;
            }
            // Definitions: pos + `; `-joined glosses (+ extras), like upstream.
            let mut defs: Vec<String> = Vec::new();
            if let Some(senses) = page.get("senses").and_then(|s| s.as_array()) {
                for s in senses {
                    let pos: Vec<&str> = s
                        .get("parts_of_speech")
                        .and_then(|p| p.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
                        .unwrap_or_default();
                    let gloss: Vec<&str> = s
                        .get("english_definitions")
                        .and_then(|e| e.as_array())
                        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
                        .unwrap_or_default();
                    let mut d = String::new();
                    if !pos.is_empty() {
                        d.push_str(&pos.join(", "));
                        d.push_str(": ");
                    }
                    d.push_str(&gloss.join("; "));
                    if !d.trim().is_empty() {
                        defs.push(d);
                    }
                }
            }
            let content = truncate(&defs.join(" "), 300);
            let url = format!("https://jisho.org/word/{slug}");
            out.push(SearchResult::basic("jisho", url, alt.join(", "), content));
        }
        out
    }
}

/// Upstream `content[:300] + (content[300:] and '...')` — Python slices
/// chars, so do chars here (byte indices would panic on CJK boundaries).
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max).collect();
    t.push_str("...");
    t
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
