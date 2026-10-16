//! NVD CVE search (`services.nvd.nist.gov/rest/json/cves/2.0`) — it.
//! `.../cves/2.0?keywordSearch=..&resultsPerPage=10&startIndex=..`
//! Response: `{vulnerabilities: [{cve: {id, descriptions[{lang,value}],
//! published}}]}`. Descriptions capped at 400 chars (CVE text runs long).
//! Usage policy (checked 2026-10-15): US federal public data; keyless
//! tier (5 req/30s) comfortably fits 1 req/query sequential+SLEEP_S=2.
//! Uses the official REST API, not the reference's nudp endpoint.
//! Fixture: `tests/fixtures/tests-fixture-nvd-test.json` (HTTP 200, clean).

use crate::engine::{Engine, SearchResult};

pub struct Nvd;

impl Engine for Nvd {
    fn name(&self) -> &'static str {
        "nvd"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    /// Full CVE records (configurations/metrics/references) run ~33KB
    /// each; 10 need ~400KB. Default 96KB truncates to silent `[]`.
    fn body_cap_kb(&self) -> Option<usize> {
        Some(400)
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let start = 10 * (pageno.clamp(1, 3) - 1);
        format!(
            "https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch={}&resultsPerPage=10&startIndex={start}",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("vulnerabilities")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for vuln in arr.iter().take(10) {
            let cve = vuln.get("cve");
            let id = cve
                .and_then(|c| c.get("id"))
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            if id.is_empty() {
                continue;
            }
            let desc = cve
                .and_then(|c| c.get("descriptions"))
                .and_then(|d| d.as_array())
                .map(|arr| {
                    arr.iter()
                        .find(|d| d.get("lang").and_then(|x| x.as_str()) == Some("en"))
                        .or_else(|| arr.first())
                        .and_then(|d| d.get("value"))
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                })
                .unwrap_or("");
            let mut content: String = desc.chars().take(400).collect();
            if desc.chars().count() > 400 {
                content.push('…');
            }
            let mut res = SearchResult::basic(
                "nvd",
                format!("https://nvd.nist.gov/vuln/detail/{id}"),
                id.to_string(),
                content,
            );
            if let Some(ts) = cve
                .and_then(|c| c.get("published"))
                .and_then(|x| x.as_str())
                .filter(|s| s.len() >= 10)
            {
                res.published_date = Some(ts[..10].into());
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
