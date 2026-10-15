//! cheat.sh (CLI cheat sheets) — plain-text answers engine (DEV-SEARCH #9).
//!
//! cheat.sh's documented text API keys off the client: curl UAs get
//! plain text, browser UAs get a 547KB HTML page. We send the curl UA
//! explicitly (this IS their API — curl is the documented client, not
//! spoofing) plus `?QT` (quiet + strip ANSI). Topic = first query token.
//! `HTTP 200 "Unknown topic."` bodies parse to `[]` (honest empty).
//! Content capped at 3000 chars; a normal result (not answers[]) so no
//! render plumbing changes.

use crate::engine::{Engine, OutRequest, SearchResult};

pub struct Cheatsh;

impl Engine for Cheatsh {
    fn name(&self) -> &'static str {
        "cheatsh"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        let topic = q.split_whitespace().next().unwrap_or("");
        format!("http://cheat.sh/{topic}?QT")
    }

    fn request(&self, query: &str, pageno: u32, prev: &[crate::engine::InResponse]) -> Option<OutRequest> {
        if !prev.is_empty() {
            return None;
        }
        let mut req = OutRequest::get(self.build_url(query, pageno));
        // cheat.sh serves text to curl, HTML to browsers. curl UA + ?QT
        // is the documented text interface.
        req.headers.push(("User-Agent".into(), "curl/8.5".into()));
        Some(req)
    }

    fn parse(&self, _body: &str) -> Vec<SearchResult> {
        vec![]
    }

    fn parse_scored(&self, body: &str, query: &str) -> Vec<SearchResult> {
        let text = strip_ansi(body);
        let t = text.trim();
        // Scaffolding, not a sheet: unknown-topic notice, JSON/HTML
        // shells, or stubs too short to be a cheat sheet.
        if t.is_empty() || t.len() < 50 || t.starts_with("Unknown topic.") || t.starts_with('<') || t.starts_with('{') {
            return vec![];
        }
        let topic = query.split_whitespace().next().unwrap_or("").to_lowercase();
        let content: String = t.chars().take(3000).collect();
        vec![SearchResult::basic(
            "cheatsh",
            format!("https://cheat.sh/{topic}"),
            format!("cheat.sh:{topic}"),
            content,
        )]
    }
}

/// Drop `\x1b[...m` SGR sequences (cheat.sh colors even with ?Q at times).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;
    #[test]
    fn ansi_stripped() {
        assert_eq!(strip_ansi("\x1b[38;5;248m# hi\x1b[0m\n"), "# hi\n");
    }
}
