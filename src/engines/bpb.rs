//! bpb.de civic-education search — clean keyless JSON (queue #10).
//! `GET /bpbapi/filter/search?query[term]=..&page=..&sort[direction]=
//! descending&payload[nid]=65350` → `{"teaser": [{teaser: {title, text,
//! link: {url}, image: {sources: [{url}]}}, extension: {overline,
//! authors, publishingDate}}]}` (German content, general category).
//! Epoch dates render as YYYY-MM-DD via civil-from-days (no date dep).

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Bpb;

const BASE: &str = "https://www.bpb.de";

impl Engine for Bpb {
    fn name(&self) -> &'static str {
        "bpb"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        format!(
            "{BASE}/bpbapi/filter/search?query%5Bterm%5D={}&page={}&sort%5Bdirection%5D=descending&payload%5Bnid%5D=65350",
            encode(q),
            pageno.saturating_sub(1),
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v.get("teaser").and_then(|x| x.as_array()).unwrap_or(&empty);
        rs.iter()
            .filter_map(|r| {
                let t = r.get("teaser")?;
                let path = t
                    .get("link")
                    .and_then(|l| l.get("url"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .trim();
                let title = t.get("title").and_then(|x| x.as_str()).unwrap_or("").trim();
                if path.is_empty() || title.is_empty() {
                    return None;
                }
                let text = t.get("text").and_then(|x| x.as_str()).unwrap_or("");
                let mut res = SearchResult::basic(
                    "bpb",
                    format!("{BASE}{path}"),
                    title.to_string(),
                    text.split_whitespace().collect::<Vec<_>>().join(" "),
                );
                if let Some(srcs) = t
                    .get("image")
                    .and_then(|i| i.get("sources"))
                    .and_then(|x| x.as_array())
                {
                    if let Some(u) = srcs
                        .last()
                        .and_then(|s| s.get("url"))
                        .and_then(|x| x.as_str())
                    {
                        if !u.trim().is_empty() {
                            res.thumbnail = format!("{BASE}{}", u.trim());
                        }
                    }
                }
                if let Some(epoch) = r
                    .get("extension")
                    .and_then(|e| e.get("publishingDate"))
                    .and_then(|x| x.as_i64())
                {
                    res.published_date = Some(ymd(epoch));
                }
                Some(res)
            })
            .take(10)
            .collect()
    }
}

/// Epoch seconds → `YYYY-MM-DD` (civil-from-days, Hinnant's algorithm).
fn ymd(epoch: i64) -> String {
    let z = epoch.div_euclid(86400) + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
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
