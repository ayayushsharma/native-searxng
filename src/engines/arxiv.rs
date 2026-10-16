//! arXiv (`export.arxiv.org`) — keyless Atom API, science.
//! `https://export.arxiv.org/api/query?search_query=all:..&max_results=10`
//! Response: Atom `<feed>` with `<entry>{<title>, <id>, <summary>}`.
//! Parsed with a minimal tag scanner (no XML crate): arXiv output is
//! machine-generated and rigid; malformed input yields zero results.
//! Fixture: `tests/fixtures/tests-fixture-arxiv-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult, unescape};

pub struct Arxiv;

impl Engine for Arxiv {
    fn name(&self) -> &'static str {
        "arxiv"
    }

    fn category(&self) -> &'static str {
        "science"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let start = (pageno.clamp(1, 3) - 1) * 10;
        format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&start={start}&max_results=10",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let mut out = Vec::new();
        // Skip the feed header: entries start at the first <entry>.
        for chunk in body.split("<entry").skip(1) {
            if out.len() >= 10 {
                break;
            }
            let entry = &chunk[..chunk.find("</entry>").unwrap_or(chunk.len())];
            let (Some(title), Some(url)) = (tag(entry, "title"), tag(entry, "id")) else {
                continue;
            };
            let title = norm(&unescape(&title));
            let url = url.trim().to_string();
            if title.is_empty() || url.is_empty() {
                continue;
            }
            let summary = tag(entry, "summary").map(|s| norm(&unescape(&s))).unwrap_or_default();
            out.push(SearchResult::basic("arxiv", url, title, summary));
        }
        out
    }
}

/// Inner text of the first `<name>...</name>` in `entry`. None if absent.
fn tag(entry: &str, name: &str) -> Option<String> {
    let open = entry.find(&format!("<{name}"))?;
    let start = entry[open..].find('>')? + open + 1;
    let end = entry[start..].find(&format!("</{name}>"))? + start;
    Some(entry[start..end].to_string())
}

/// Collapse Atom's whitespace/newlines to single spaces.
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
