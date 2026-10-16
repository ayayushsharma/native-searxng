//! Sphinx `searchindex.js` client-side search (DEV-SEARCH pattern #2).
//!
//! Sphinx doc sites (Flask, pip, Django...) ship a static search index:
//! `Search.setIndex({docnames:[...], titles:[...], terms:{word:docidx|[docidx]},
//! titleterms:{...}})`. The engine fetches it LIVE per query (raised cap,
//! no stored state — statelessness invariant), scores query tokens
//! against `terms` (+1) and `titleterms` (+2), falls back to title
//! substring when nothing matches (symbol queries like `opendir`),
//! assembles links as `{base}{docname}{suffix}`.
//!
//! Format skew across Sphinx versions is contained: only `docnames`,
//! `titles`, `terms`, `titleterms` are read; posting values accept int
//! or int-array; everything else is ignored. Sites whose index exceeds
//! the engine's `body_cap_kb` drop out of the class (docs.python.org's
//! 4.1MB index is parked with evidence — see SKIP-LOG).

use crate::engine::SearchResult;
use serde_json::Value;
use std::collections::HashMap;

/// One doc site in the class: index URL, link base, URL suffix.
pub struct Site {
    pub index_url: &'static str,
    pub base: &'static str,
    /// RTD-style dirhtml sites use `"/"`; plain-Sphinx sites `".html"`.
    pub suffix: &'static str,
}

pub fn search(body: &str, site: &Site, query: &str, engine: &'static str) -> Vec<SearchResult> {
    // Tolerant envelope strip: first `{` … last `}` (prefix/suffix vary).
    let (Some(s), Some(e)) = (body.find('{'), body.rfind('}')) else {
        return vec![];
    };
    let v: Value = match serde_json::from_str(&body[s..=e]) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    let empty_v = vec![];
    let empty_m = serde_json::Map::new();
    let docnames: Vec<&str> = v
        .get("docnames")
        .and_then(|x| x.as_array())
        .unwrap_or(&empty_v)
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    let titles: Vec<&str> = v
        .get("titles")
        .and_then(|x| x.as_array())
        .unwrap_or(&empty_v)
        .iter()
        .filter_map(|x| x.as_str())
        .collect();
    if docnames.is_empty() {
        return vec![];
    }
    let terms = v
        .get("terms")
        .and_then(|x| x.as_object())
        .unwrap_or(&empty_m);
    let titleterms = v
        .get("titleterms")
        .and_then(|x| x.as_object())
        .unwrap_or(&empty_m);

    let tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| t.len() >= 2)
        .collect();
    if tokens.is_empty() {
        return vec![];
    }
    let mut scores: HashMap<usize, i32> = HashMap::new();
    for tok in &tokens {
        for (table, w) in [(terms, 1), (titleterms, 2)] {
            if let Some(post) = table.get(tok.as_str()) {
                for idx in posting(post) {
                    *scores.entry(idx).or_insert(0) += w;
                }
            }
        }
    }
    if scores.is_empty() {
        // Symbol fallback: substring over titles (no stemming in index).
        let ql = query.to_lowercase();
        for (i, t) in titles.iter().enumerate() {
            if t.to_lowercase().contains(&ql) {
                scores.insert(i, 1);
            }
        }
    }
    let mut ranked: Vec<(usize, i32)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ranked
        .into_iter()
        .take(10)
        .filter_map(|(i, _)| {
            let doc = docnames.get(i)?.trim();
            if doc.is_empty() {
                return None;
            }
            let title = titles
                .get(i)
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .unwrap_or(doc);
            Some(SearchResult::basic(
                engine,
                format!("{}{}{}", site.base, doc, site.suffix),
                title.to_string(),
                format!("{title} — {}{doc}", site.base),
            ))
        })
        .collect()
}

/// Posting value: bare int (single doc) or int array. Anything else: skip.
fn posting(v: &Value) -> Vec<usize> {
    match v {
        Value::Number(n) => n.as_u64().map(|i| vec![i as usize]).unwrap_or_default(),
        Value::Array(a) => a
            .iter()
            .filter_map(|x| x.as_u64().map(|i| i as usize))
            .collect(),
        _ => vec![],
    }
}
