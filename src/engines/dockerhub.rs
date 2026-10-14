//! Docker Hub (`hub.docker.com`) — keyless JSON API, it.
//! `https://hub.docker.com/api/search/v3/catalog/search?q=..&page_size=10`
//! Response: `{"results": [{slug, name, short_description, source,
//! logo_url{large,small}, publisher{name}, updated_at, ...}]}`.
//! Official images (`source` store/official) live under `/_/`, rest under `/r/`.
//! Fixture: `tests-fixture-dockerhub-test.json` (HTTP 200, verified clean).

use crate::engine::{Engine, SearchResult};

pub struct DockerHub;

impl Engine for DockerHub {
    fn name(&self) -> &'static str {
        "dockerhub"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        let from = (pageno.clamp(1, 3) - 1) * 10;
        format!(
            "https://hub.docker.com/api/search/v3/catalog/search?q={}&page_size=10&from={from}",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: serde_json::Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let arr = v
            .get("results")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        let mut out = Vec::new();
        for r in arr.iter().take(10) {
            let (Some(slug), Some(title)) = (
                r.get("slug").and_then(|x| x.as_str()),
                r.get("name").and_then(|x| x.as_str()),
            ) else {
                continue;
            };
            if slug.is_empty() || title.is_empty() {
                continue;
            }
            let official = matches!(
                r.get("source").and_then(|x| x.as_str()),
                Some("store") | Some("official")
            );
            let url = if official {
                format!("https://hub.docker.com/_/{slug}")
            } else {
                format!("https://hub.docker.com/r/{slug}")
            };
            let desc = r
                .get("short_description")
                .and_then(|x| x.as_str())
                .unwrap_or_default();
            let mut res = SearchResult::basic(
                "dockerhub",
                url,
                title.into(),
                desc.split_whitespace().collect::<Vec<_>>().join(" "),
            );
            res.template = "packages.html".into();
            if let Some(logo) = r.get("logo_url") {
                if let Some(large) = logo
                    .get("large")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        logo.get("small").and_then(|x| x.as_str()).filter(|s| !s.is_empty())
                    })
                {
                    res.thumbnail = large.into();
                }
            }
            if let Some(ts) = r
                .get("updated_at")
                .or_else(|| r.get("created_at"))
                .and_then(|x| x.as_str())
            {
                res.published_date = Some(ts.into());
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
