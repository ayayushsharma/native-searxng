//! Swisscows web search — signed calculator flow (queue #12, decision: PORT).
//! `X-Request-Nonce` (32 random chars) + ROT13-with-case-flip +
//! `X-Request-Signature = b64url(sha256(signed_path + shifted_nonce))`
//! where the signed path sorts keys alphabetically with PLAINTEXT values.
//! Response is a JWT: the middle segment base64url-decodes to
//! `{"items": [{type WebPage, url, name, description, thumbnail.url}]}`.
//! Web only (images/videos need unported media templates).
//! Nonce uses time+pid entropy: it proves freshness, not secrecy.

use crate::crypto::{b64url_decode, b64url_encode, sha256};
use crate::engine::{strip_tags, Engine, SearchResult};
use serde_json::Value;

pub struct Swisscows;

const BASE: &str = "https://api.swisscows.com";
const PATH: &str = "/v5/web/search";
const NONCE_ALPHA: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~";

impl Engine for Swisscows {
    fn name(&self) -> &'static str {
        "swisscows"
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        format!("{BASE}{PATH}")
    }

    fn request(
        &self,
        query: &str,
        pageno: u32,
        prev: &[crate::engine::InResponse],
    ) -> Option<crate::engine::OutRequest> {
        if !prev.is_empty() {
            return None;
        }
        // Keys sorted alphabetically, values PLAINTEXT in the signed path
        // (URL-encoding happens only in the actual request URL).
        let offset = (pageno.saturating_sub(1)) * 10;
        let signed = format!(
            "{PATH}?freshness=All&itemsCount=10&locale=en-US&offset={offset}&query={query}&spellcheck=True"
        );
        let nonce = nonce();
        let sig = b64url_encode(&sha256(format!("{signed}{}", caesar(&nonce)).as_bytes()));
        let url = format!(
            "{BASE}{PATH}?freshness=All&itemsCount=10&locale=en-US&offset={offset}&query={}&spellcheck=True",
            encode(query),
        );
        Some(crate::engine::OutRequest {
            url,
            method: crate::engine::Method::Get,
            form: vec![],
            json: None,
            headers: vec![
                ("X-Request-Nonce".into(), nonce),
                ("X-Request-Signature".into(), sig),
            ],
        })
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        // Unwrap the JWT payload (middle segment).
        let payload = v.get("payload").and_then(|x| x.as_str()).unwrap_or("");
        let mut segs = payload.split('.');
        let inner: Value = match (segs.next(), segs.next()) {
            (_, Some(mid)) => match b64url_decode(mid) {
                Some(raw) => match serde_json::from_slice(&raw) {
                    Ok(j) => j,
                    Err(_) => return vec![],
                },
                None => return vec![],
            },
            _ => return vec![],
        };
        let empty = vec![];
        let items = inner
            .get("items")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        items
            .iter()
            .filter_map(|r| {
                if r.get("type").and_then(|x| x.as_str()) != Some("WebPage") {
                    return None;
                }
                let url = r.get("url")?.as_str()?.trim();
                let title = r.get("name").and_then(|x| x.as_str()).unwrap_or("").trim();
                if url.is_empty() || title.is_empty() {
                    return None;
                }
                let desc = r.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let mut res = SearchResult::basic(
                    "swisscows",
                    url.to_string(),
                    strip_tags(title),
                    strip_tags(desc),
                );
                if let Some(t) = r
                    .get("thumbnail")
                    .and_then(|t| t.get("url"))
                    .and_then(|x| x.as_str())
                {
                    if !t.trim().is_empty() {
                        res.thumbnail = t.trim().to_string();
                    }
                }
                Some(res)
            })
            .take(10)
            .collect()
    }
}

/// ROT13 with case flip (oracle `caesar_shift_with_switch_case`, offset 13).
fn caesar(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                let shifted = (c as u8 - base + 13) % 26 + base;
                let out = shifted as char;
                if c.is_ascii_uppercase() {
                    out.to_ascii_lowercase()
                } else {
                    out.to_ascii_uppercase()
                }
            } else {
                c
            }
        })
        .collect()
}

/// 32-char freshness nonce from time+pid entropy folded through SHA-256.
fn nonce() -> String {
    let seed = format!(
        "{:?}{}{}",
        std::time::SystemTime::now(),
        std::process::id(),
        nonce_counter()
    );
    let digest = sha256(seed.as_bytes());
    digest
        .iter()
        .take(32)
        .map(|b| NONCE_ALPHA[(b % 64) as usize] as char)
        .collect()
}

fn nonce_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
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
