//! Keenable AI search — single POST, keyless by default (queue #13).
//! `POST /v1/search/public` JSON `{query, mode: pro}` with an
//! `X-Keenable-Title` UA header → `{"results": [{url, title,
//! description/snippet, published_at}]}`. When `SEARXNG_API_KEY_KEENABLE`
//! is set, the keyed `/v1/search` + `X-API-KEY` endpoint is used instead
//! (same env convention as jina/chatnoir; unset = dormant keyless).

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Keenable;

const BASE: &str = "https://api.keenable.ai";

impl Engine for Keenable {
    fn name(&self) -> &'static str {
        "keenable"
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        format!("{BASE}/v1/search/public")
    }

    fn request(
        &self,
        query: &str,
        _pageno: u32,
        prev: &[crate::engine::InResponse],
    ) -> Option<crate::engine::OutRequest> {
        if !prev.is_empty() {
            return None;
        }
        let key = crate::config::api_key("keenable");
        let url = if key.is_some() {
            format!("{BASE}/v1/search")
        } else {
            format!("{BASE}/v1/search/public")
        };
        let q = Value::String(query.to_string());
        let mut headers = vec![(
            "X-Keenable-Title".to_string(),
            "SearXNG native-searxng".to_string(),
        )];
        if let Some(k) = key {
            headers.push(("X-API-KEY".to_string(), k));
        }
        Some(crate::engine::OutRequest {
            url,
            method: crate::engine::Method::Post,
            form: vec![],
            json: Some(format!("{{\"query\":{q},\"mode\":\"pro\"}}")),
            headers,
        })
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.get("results").and_then(|x| x.as_array()).unwrap_or(&empty);
        rs.iter()
            .filter_map(|r| {
                let url = r.get("url")?.as_str()?.trim();
                let title = r.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
                if url.is_empty() || title.is_empty() {
                    return None;
                }
                let desc = r.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let snip = r.get("snippet").and_then(|x| x.as_str()).unwrap_or("");
                let content = if desc.trim().is_empty() { snip } else { desc };
                let mut res = SearchResult::basic(
                    "keenable",
                    url.to_string(),
                    title.to_string(),
                    content.split_whitespace().collect::<Vec<_>>().join(" "),
                );
                if let Some(pub_at) = r
                    .get("published_at")
                    .and_then(|x| x.as_str())
                    .filter(|s| !s.trim().is_empty())
                {
                    res.published_date = Some(pub_at.trim().to_string());
                }
                Some(res)
            })
            .take(10)
            .collect()
    }
}
