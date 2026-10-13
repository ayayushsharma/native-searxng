//! Renderers with SearXNG's exact JSON keys (see agentic/CONTRACT.md).

use crate::engine::SearchResult;
use serde::Serialize;

#[derive(Serialize)]
struct Envelope<'a> {
    query: &'a str,
    results: &'a [SearchResult],
    answers: [&'static str; 0],
    corrections: [&'static str; 0],
    infoboxes: [&'static str; 0],
    suggestions: Vec<String>,
    unresponsive_engines: &'a [(String, String)],
}

pub fn json(query: &str, results: &[SearchResult], unresponsive: &[(String, String)]) -> String {
    let env = Envelope {
        query,
        results,
        answers: [],
        corrections: [],
        infoboxes: [],
        suggestions: vec![],
        unresponsive_engines: unresponsive,
    };
    serde_json::to_string(&env).unwrap_or_else(|_| "{\"error\":\"render failed\"}".into())
}

pub fn error_json(msg: &str) -> String {
    serde_json::json!({"error": msg}).to_string()
}

/// Minimal HTML for human smoke checks. Agents use JSON; the pretty page
/// comes later (deferred per PLAN.md).
pub fn html(query: &str, results: &[SearchResult]) -> String {
    let mut h = String::from(
        "<!doctype html><html><head><meta charset=utf-8><title>searxng</title></head><body>",
    );
    h.push_str("<form action=/search><input name=q value=\"");
    h.push_str(&escape(query));
    h.push_str("\"></form><ol>");
    for r in results {
        h.push_str("<li><a href=\"");
        h.push_str(&escape(&r.url));
        h.push_str("\">");
        h.push_str(&escape(&r.title));
        h.push_str("</a><p>");
        h.push_str(&escape(&r.content));
        h.push_str("</p></li>");
    }
    h.push_str("</ol></body></html>");
    h
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
