//! Crossref (`api.crossref.org/works`) — keyless JSON API, science.
//! `https://api.crossref.org/works?query=..&rows=10&offset=..`
//! Response: `{message: {items: [{title[], DOI, URL, author[],
//! container-title[], published{"date-parts"}}]}}`.
//! Usage policy (checked 2026-10-15): no robots.txt on the API host;
//! etiquette asks an optional `mailto=` for the polite pool — none
//! invented (no credentials on box), so descriptive UA only, 1 req/query.
//! Fixture: `tests/fixtures/tests-fixture-crossref-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct Crossref;

impl Engine for Crossref {
    fn name(&self) -> &'static str {
        "crossref"
    }

    fn category(&self) -> &'static str {
        "science"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let offset = 10 * (pageno.clamp(1, 3) - 1);
        format!(
            "https://api.crossref.org/works?query={}&rows=10&offset={offset}",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("message")
            .and_then(|m| m.get("items"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for item in arr.iter().take(10) {
            let title = item
                .get("title")
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            if title.is_empty() {
                continue;
            }
            let url = item
                .get("URL")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| {
                    item.get("DOI")
                        .and_then(|x| x.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|d| format!("https://doi.org/{d}"))
                })
                .unwrap_or_default();
            if url.is_empty() {
                continue;
            }
            let mut bits = Vec::new();
            let authors: Vec<String> = item
                .get("author")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|a| {
                            a.get("name")
                                .and_then(|x| x.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| {
                                    let g = a.get("given").and_then(|x| x.as_str()).unwrap_or("");
                                    let f = a.get("family").and_then(|x| x.as_str()).unwrap_or("");
                                    let full = format!("{g} {f}").trim().to_string();
                                    if full.is_empty() {
                                        None
                                    } else {
                                        Some(full)
                                    }
                                })
                        })
                        .take(3)
                        .collect()
                })
                .unwrap_or_default();
            if !authors.is_empty() {
                bits.push(authors.join(", "));
            }
            if let Some(container) = item
                .get("container-title")
                .and_then(|t| t.as_array())
                .and_then(|a| a.first())
                .and_then(|x| x.as_str())
                .filter(|s| !s.trim().is_empty())
            {
                bits.push(container.trim().to_string());
            }
            let mut res = SearchResult::basic("crossref", url, title.to_string(), bits.join(" — "));
            // Date lives under published / published-print /
            // published-online / issued (whichever the record carries).
            let date_obj = ["published", "published-print", "published-online", "issued"]
                .iter()
                .filter_map(|k| item.get(*k))
                .next();
            if let Some(year) = date_obj
                .and_then(|p| p.get("date-parts"))
                .and_then(|d| d.as_array())
                .and_then(|a| a.first())
                .and_then(|y| y.as_array())
                .and_then(|a| a.first())
                .and_then(|y| y.as_u64())
            {
                res.published_date = Some(format!("{year:04}"));
            }
            out.push(res);
        }
        out
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
