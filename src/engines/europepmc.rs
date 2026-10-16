//! Europe PMC (`ebi.ac.uk`) — keyless JSON API, science.
//! `https://www.ebi.ac.uk/europepmc/webservices/rest/search?query=..&format=json`
//! Response: `{"resultList": {"result": [{source, id, title, abstractText,
//! firstPublicationDate, ...}]}}`. Page-1 needs no cursorMark.
//! Fixture: `tests/fixtures/tests-fixture-europepmc-test.json` (HTTP 200, verified clean).

use crate::engine::{unescape, Engine, SearchResult};

pub struct EuropePmc;

impl Engine for EuropePmc {
    fn name(&self) -> &'static str {
        "europepmc"
    }

    fn category(&self) -> &'static str {
        "science"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let _ = pageno;
        format!(
            "https://www.ebi.ac.uk/europepmc/webservices/rest/search?query={}&format=json&resultType=core&pageSize=10",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("resultList")
            .and_then(|x| x.get("result"))
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let (Some(source), Some(id), Some(title)) = (
                r.get("source").and_then(|x| x.as_str()),
                r.get("id").and_then(|x| x.as_str()),
                r.get("title").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if source.is_empty() || id.is_empty() || title.is_empty() {
                continue;
            }
            let abstract_text = r
                .get("abstractText")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            let mut res = SearchResult::basic(
                "europepmc",
                format!("https://europepmc.org/article/{source}/{id}"),
                norm(&unescape(title)),
                norm(&unescape(abstract_text)),
            );
            if let Some(date) = r
                .get("firstPublicationDate")
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
