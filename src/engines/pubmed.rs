//! PubMed (`eutils.ncbi.nlm.nih.gov`) — two-step JSON API, science.
//! Step 1 (build_url): `esearch.fcgi?db=pubmed&term=..&retmode=json`
//!   → `{"esearchresult": {"idlist": [...]}}`.
//! Step 2 (follow_up): `esummary.fcgi?db=pubmed&id=..&retmode=json`
//!   → `{"result": {"uids": [...], "<uid>": {title, source, pubdate,
//! authors[{name}], ...}}}`.
//! First two-step engine: the trait's `follow_up` hook performs at most
//! one extra fetch; either leg failing surfaces as `unresponsive_engines`.
//! Fixture: `tests-fixture-pubmed-test.json` (leg-2 esummary body,
//! HTTP 200, verified clean; leg-1 esearch shape asserted live).

use crate::engine::{Engine, SearchResult};

pub struct Pubmed;

impl Engine for Pubmed {
    fn name(&self) -> &'static str {
        "pubmed"
    }

    fn category(&self) -> &'static str {
        "science"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let _ = pageno;
        format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}&retmax=10&retmode=json",
            encode(q)
        )
    }

    fn rounds(&self) -> usize {
        2
    }

    fn request(&self, _query: &str, _pageno: u32, prev: &[crate::engine::InResponse]) -> Option<crate::engine::OutRequest> {
        if prev.is_empty() {
            return Some(crate::engine::OutRequest::get(self.build_url(_query, _pageno)));
        }
        if prev.len() > 1 {
            return None;
        }
        let v: serde_json::Value = serde_json::from_str(&prev[0].body).ok()?;
        let ids = v
            .get("esearchresult")?
            .get("idlist")?
            .as_array()?;
        if ids.is_empty() {
            return None;
        }
        let list: Vec<&str> = ids.iter().filter_map(|x| x.as_str()).take(10).collect();
        if list.is_empty() {
            return None;
        }
        Some(crate::engine::OutRequest::get(format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
            list.join(",")
        )))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let result = match v.get("result") {
            Some(r) => r,
            None => return vec![],
        };
        let uids = result
            .get("uids")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for uid in uids.iter().take(10) {
            let Some(id) = uid.as_str() else { continue };
            let Some(r) = result.get(id) else { continue };
            let title = r
                .get("title")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            if title.is_empty() {
                continue;
            }
            let mut bits = Vec::new();
            if let Some(source) = r.get("source").and_then(|x| x.as_str()) {
                if !source.is_empty() {
                    bits.push(source.to_string());
                }
            }
            if let Some(authors) = r.get("authors").and_then(|x| x.as_array()) {
                let names: Vec<&str> = authors
                    .iter()
                    .filter_map(|a| a.get("name").and_then(|x| x.as_str()))
                    .take(4)
                    .collect();
                if !names.is_empty() {
                    bits.push(names.join(", "));
                }
            }
            let mut res = SearchResult::basic(
                "pubmed",
                format!("https://pubmed.ncbi.nlm.nih.gov/{id}/"),
                norm(title),
                norm(&bits.join(" — ")),
            );
            if let Some(date) = r
                .get("pubdate")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                res.published_date = Some(date.into());
            }
            out.push(res);
        }
        out
    }
}

fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
