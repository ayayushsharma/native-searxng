//! Yahoo news — HTML result cards (upstream `yahoo_news` twin).
//! `GET news.search.yahoo.com/search?p=&b=` → cards in
//! `searchCenterMiddle .compArticleList li.ov-a`: the thumb anchor carries
//! the real URL percent-encoded in `RU=.../RK=` plus `title="..."`, then
//! `s-source` (outlet), `s-time` (relative date) and `s-desc` (snippet).
//! Relative dates become YYYY-MM-DD at parse time so `rank()` recency
//! just works. 146KB pages → `body_cap_kb` 200.

use crate::engine::{civil_from_days, strip_tags, today_days, unescape, Engine, SearchResult};

pub struct YahooNews;

impl Engine for YahooNews {
    fn name(&self) -> &'static str {
        "yahoo_news"
    }

    fn category(&self) -> &'static str {
        "news"
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        "https://news.search.yahoo.com/search".to_string()
    }

    fn body_cap_kb(&self) -> Option<usize> {
        Some(200)
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
        let b = (pageno.saturating_sub(1)) * 10 + 1;
        Some(crate::engine::OutRequest {
            url: format!(
                "https://news.search.yahoo.com/search?p={}&b={b}",
                encode(query)
            ),
            method: crate::engine::Method::Get,
            form: vec![],
            json: None,
            headers: vec![],
        })
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let mut out = vec![];
        for card in body.split("<li class=\"ov-a").skip(1) {
            let Some(url) = redirect_url(card) else {
                continue;
            };
            let Some(title) = attr_value(card, "title=\"") else {
                continue;
            };
            let title = title.trim();
            if url.is_empty() || title.is_empty() {
                continue;
            }
            let snippet = card
                .find("<p class=\"s-desc")
                .and_then(|i| card[i..].find('>').map(|g| i + g + 1))
                .and_then(|s| card[s..].find("</p>").map(|e| strip_tags(&card[s..s + e])));
            let mut res = SearchResult::basic(
                "yahoo_news",
                url,
                unescape(title),
                snippet.unwrap_or_default(),
            );
            if let Some(rel) = time_text(card) {
                if let Some(d) = relative_to_date(&rel) {
                    res.published_date = Some(d);
                }
            }
            out.push(res);
            if out.len() >= 10 {
                break;
            }
        }
        out
    }
}

/// Real URL out of `...RU=<percent-encoded>/RK=...`. Hearts on sleeves:
/// must decode to http(s), else the card is an ad/nav link.
fn redirect_url(card: &str) -> Option<String> {
    let i = card.find("RU=")?;
    let rest = &card[i + 3..];
    let end = rest.find("/RK=")?;
    let url = pct_decode(&rest[..end]);
    if url.starts_with("http://") || url.starts_with("https://") {
        Some(url)
    } else {
        None
    }
}

fn attr_value(card: &str, name: &str) -> Option<String> {
    let i = card.find(name)?;
    let rest = &card[i + name.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// `<span class="s-time ...">2 hours ago &middot;` → relative part.
fn time_text(card: &str) -> Option<String> {
    let i = card.find("s-time")?;
    let rest = &card[i..];
    let s = rest.find('>')? + 1;
    let e = rest[s..].find("&middot")?;
    Some(rest[s..s + e].trim().to_string())
}

/// "N minute/hour/day/week/month/year(s) ago" → YYYY-MM-DD. Unknown
/// shapes → None (no date beats a wrong date for recency).
fn relative_to_date(rel: &str) -> Option<String> {
    let rel = rel.to_lowercase();
    if rel.contains("just now") || rel.contains("second") {
        return Some(today_string(0));
    }
    let num: i64 = rel.split_whitespace().next()?.parse().ok()?;
    let days = if rel.contains("minute") || rel.contains("hour") {
        0
    } else if rel.contains("day") {
        num
    } else if rel.contains("week") {
        num * 7
    } else if rel.contains("month") {
        num * 30
    } else if rel.contains("year") {
        num * 365
    } else {
        return None;
    };
    Some(today_string(days.max(0)))
}

fn today_string(ago_days: i64) -> String {
    let (y, m, d) = civil_from_days(today_days() - ago_days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn pct_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as char);
                i += 3;
                continue;
            }
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
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
