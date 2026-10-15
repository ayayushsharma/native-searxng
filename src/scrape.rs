//! Native page scrape for `/fetch` — no third party, no new crates.
//! `scrape(client, target)`: GET (256KB cap, redirects followed) →
//! bot-wall sniff → title/content/links via hand scanners.
//! Thin or walled pages `Err` (server falls back to Jina Reader).
//! Reuses `jina::FetchResult` so the render contract is unchanged.
//! Empirical basis: 19/19 sampled result pages server-render full text;
//! failures were walls/dead-links, never JS shells (see session notes).

use crate::{
    engine::{self, OutRequest},
    fetch::Client,
    jina::FetchResult,
};

/// Over-cap HTML still parses (truncated tail = partial text, honest).
const PAGE_CAP_KB: usize = 256;
/// Below this the page is a shell/error/chrome — fall back to Reader.
const MIN_CONTENT_CHARS: usize = 120;
/// Match the Reader backend so responses stay comparable.
const MAX_CONTENT_CHARS: usize = 20_000;
const MAX_LINKS: usize = 50;

/// Bot-wall title markers (case-insensitive). Hit → `Err`, never junk.
const WALL_MARKERS: [&str; 5] = [
    "just a moment",
    "attention required",
    "access denied",
    "client challenge",
    "are you a robot",
];

/// Fetch + extract `target`. `Err` is log-safe status text (fallback signal).
pub fn scrape(client: &Client, target: &str) -> Result<FetchResult, String> {
    if !target.starts_with("http://") && !target.starts_with("https://") {
        return Err("fetch needs an http(s) URL".into());
    }
    let req = OutRequest::get(target.to_string());
    let res = client.execute_capped(&req, Some(PAGE_CAP_KB)).map_err(short_err)?;
    if res.status != 200 {
        return Err(format!("native scrape: http {}", res.status));
    }
    if res.body.trim().is_empty() {
        return Err("native scrape: empty response".into());
    }
    parse(&res.body, target).ok_or_else(|| "native scrape: no readable content".into())
}

/// Pure extract (offline-testable). `None` = thin/walled/garbage.
pub fn parse(html: &str, base: &str) -> Option<FetchResult> {
    let title = title_of(html);
    let low = title.to_lowercase();
    if WALL_MARKERS.iter().any(|m| low.contains(m)) {
        return None;
    }
    let content = content_of(html);
    if content.chars().count() < MIN_CONTENT_CHARS {
        return None;
    }
    let content: String = content.chars().take(MAX_CONTENT_CHARS).collect();
    Some(FetchResult { title, content, links: links_of(html, base) })
}

/// Bot-wall verdict over a finished extract (title + short body).
/// Used by the server to reject Reader-fallback junk: a real article
/// about captchas is long; a challenge page is short.
/// (Native `parse` sniffs titles only — it never sees short bodies.)
pub fn is_walled(title: &str, content: &str) -> bool {
    const BODY_MARKERS: [&str; 4] = [
        "just a moment",
        "verify you are human",
        "attention required",
        "client challenge",
    ];
    let low = title.to_lowercase();
    if WALL_MARKERS.iter().any(|m| low.contains(m)) {
        return true;
    }
    if content.chars().count() < 1000 {
        let low = content.to_lowercase();
        return BODY_MARKERS.iter().any(|m| low.contains(m));
    }
    false
}

/// `<title>` first, `og:title` fallback. May be empty (content rules).
fn title_of(html: &str) -> String {
    let low = html.to_lowercase();
    if let Some(i) = low.find("<title") {
        let start = html[i..].find('>').map(|e| i + e + 1).unwrap_or(html.len());
        if let Some(e) = low[start..].find("</title>") {
            let t = engine::strip_tags(&html[start..start + e]).trim().to_string();
            if !t.is_empty() {
                return t;
            }
        }
    }
    // og:title: `<meta property="og:title" content="…">` (either order).
    // Positions from the lowercased copy, slices from the original.
    let mut off = 0;
    while let Some(rel) = low[off..].find("og:title") {
        let i = off + rel;
        let tag_start = html[..i].rfind('<').unwrap_or(0);
        let tag_end = html[i..].find('>').map(|e| i + e).unwrap_or(html.len());
        if let Some(c) = engine::attr(&html[tag_start..tag_end], "content") {
            let t = c.trim().to_string();
            if !t.is_empty() {
                return t;
            }
        }
        off = i + 8;
        if off >= html.len() {
            break;
        }
    }
    String::new()
}

/// Drop script/style/noscript/nav blocks + comments, de-tag the rest.
fn content_of(html: &str) -> String {
    let mut t = remove_comments(html);
    for tag in ["script", "style", "noscript", "nav"] {
        t = remove_blocks(&t, tag);
    }
    engine::strip_tags(&t)
}

/// Case-insensitive `<tag…>…</tag>` removal; unclosed tail dropped.
fn remove_blocks(s: &str, tag: &str) -> String {
    let low = s.to_lowercase();
    let mut out = String::with_capacity(s.len());
    let mut pos = 0;
    loop {
        let open = low[pos..].find(&format!("<{tag}"));
        let Some(o) = open.map(|x| pos + x) else {
            out.push_str(&s[pos..]);
            break;
        };
        out.push_str(&s[pos..o]);
        let close = low[o..].find(&format!("</{tag}>"));
        match close.map(|x| o + x + tag.len() + 3) {
            Some(end) => pos = end,
            None => break, // unclosed: drop the tail (scripts at cap edge)
        }
    }
    out
}

fn remove_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pos = 0;
    while let Some(o) = s[pos..].find("<!--") {
        let o = pos + o;
        out.push_str(&s[pos..o]);
        match s[o..].find("-->") {
            Some(e) => pos = o + e + 3,
            None => return out, // unclosed: drop the tail
        }
    }
    out.push_str(&s[pos..]);
    out
}

/// `<a href>` → absolute http(s), resolved + deduped, fragment-cut.
fn links_of(html: &str, base: &str) -> Vec<String> {
    let low = html.to_lowercase();
    let mut out: Vec<String> = Vec::new();
    let mut pos = 0;
    while let Some(o) = low[pos..].find("<a") {
        let o = pos + o;
        let end = low[o..].find('>').map(|e| o + e).unwrap_or(html.len().min(o + 512));
        if let Some(href) = engine::attr(&html[o..end], "href") {
            let h = href.trim();
            if !h.is_empty()
                && !h.starts_with('#')
                && !h.to_lowercase().starts_with("javascript:")
                && !h.to_lowercase().starts_with("mailto:")
            {
                if let Some(abs) = resolve(base, h) {
                    if !out.contains(&abs) {
                        out.push(abs);
                        if out.len() >= MAX_LINKS {
                            break;
                        }
                    }
                }
            }
        }
        pos = end + 1;
        if pos >= html.len() {
            break;
        }
    }
    out
}

/// Minimal absolute-URL resolver (no new crate): absolute, protocol-,
/// root- and path-relative (with `.`/`..` collapse). http(s) only.
fn resolve(base: &str, href: &str) -> Option<String> {
    let h = href.split('#').next().unwrap_or("");
    if h.is_empty() {
        return None;
    }
    if h.starts_with("http://") || h.starts_with("https://") {
        return Some(h.to_string());
    }
    let (scheme, rest) = base.split_once("://")?;
    let host_end = rest.find('/').unwrap_or(rest.len());
    let origin = format!("{scheme}://{}", &rest[..host_end]);
    if let Some(stripped) = h.strip_prefix("//") {
        return Some(format!("{scheme}://{stripped}"));
    }
    if let Some(path) = h.strip_prefix('/') {
        return Some(format!("{origin}/{path}"));
    }
    let base_path = &rest[host_end..];
    let dir = base_path.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut segs: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();
    for part in h.split('/').filter(|s| !s.is_empty() && *s != ".") {
        if part == ".." {
            segs.pop();
        } else {
            segs.push(part);
        }
    }
    Some(format!("{origin}/{}", segs.join("/")))
}

/// Never leak bodies/keys: status-text shapes only.
fn short_err(e: String) -> String {
    let e = e.trim().to_string();
    if e.is_empty() {
        return "native scrape: fetch failed".into();
    }
    let first = e.lines().next().unwrap_or(&e);
    let short: String = first.chars().take(80).collect();
    format!("native scrape: {short}")
}

