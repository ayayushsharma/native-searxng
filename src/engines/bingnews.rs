//! Bing News (`bing.com/news`) — HTML scrape, news.
//! `https://www.bing.com/news/infinitescrollajax?q=..&first=1`
//! Response: HTML with `<div class="newsitem">` cards each carrying
//! `<a class="title">`, `<div class="snippet">`, optional source/author
//! metadata and `<a class="imagelink"><img>` thumbnails.
//! Hand scanner over card substrings (no scraper); truncated bodies
//! (96KB cap) parse as far as they go, never panic.
//! Fixture: `tests-fixture-bingnews-test.json` (HTTP 200, 6 cards, clean).

use crate::engine::{Engine, SearchResult};

pub struct BingNews;

impl Engine for BingNews {
    fn name(&self) -> &'static str {
        "bingnews"
    }

    fn category(&self) -> &'static str {
        "news"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        // Exact SearXNG params: InfiniteScroll/SFX/form select the card
        // layout (`div.newsitem`); without them Bing serves a fallback page.
        let page = pageno.clamp(1, 3) - 1;
        format!(
            "https://www.bing.com/news/infinitescrollajax?q={}&InfiniteScroll=1&first={}&SFX={page}&form=PTFTNR",
            encode(q),
            page * 10 + 1,
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        // Card split on the `newsitem` marker (fixture-verified shape).
        let mut out = Vec::new();
        for card in body.split("newsitem").skip(1).take(10) {
            // Title anchor: the `<a>` tag carrying `class="title"`.
            let Some(ti) = card.find("class=\"title\"") else { continue };
            let Some(a_start) = card[..ti].rfind("<a") else { continue };
            let a_end = card[ti..].find('>').map(|g| ti + g).unwrap_or(card.len());
            let tag = &card[a_start..a_end];
            let url = crate::engine::attr(tag, "href").unwrap_or_default();
            // Title text: `<h2>` inside the anchor, else anchor inner text.
            let rest_a = &card[a_end.min(card.len())..];
            let title = match rest_a.find("<h2") {
                Some(h) => match rest_a[h..].find('>') {
                    Some(g) => {
                        let ts = h + g + 1;
                        let te = rest_a[ts..].find("</h2>").map(|e| ts + e).unwrap_or(rest_a.len());
                        norm(&crate::engine::strip_tags(&rest_a[ts..te]))
                    }
                    None => String::new(),
                },
                None => match rest_a.find("</a>") {
                    Some(e) => norm(&crate::engine::strip_tags(&rest_a[..e])),
                    None => String::new(),
                },
            };
            let url = url.trim();
            if url.is_empty() || title.is_empty() {
                continue;
            }
            let mut content = match card.find("<div class=\"snippet\"") {
                Some(si) => match card[si..].find('>') {
                    Some(g) => {
                        let cs = si + g + 1;
                        let ce = card[cs..].find("</div>").map(|e| cs + e).unwrap_or(card.len());
                        norm(&crate::engine::strip_tags(&card[cs..ce]))
                    }
                    None => String::new(),
                },
                None => String::new(),
            };
            // Metadata mirrors SearXNG: source aria-label + data-author.
            let mut meta = Vec::new();
            // Source logo link: `aria-label="Search news from X"`.
            if let Some(label) = card
                .find("aria-label=\"Search news from ")
                .and_then(|i| {
                    let s = &card[i + "aria-label=\"Search news from ".len()..];
                    s.find('"').map(|e| s[..e].trim().to_string())
                })
                .filter(|s| !s.is_empty())
            {
                meta.push(label);
            }
            if let Some(author) =
                crate::engine::attr(tag, "data-author").filter(|s| !s.trim().is_empty())
            {
                meta.push(author.trim().to_string());
            }
            if !meta.is_empty() {
                if !content.is_empty() {
                    content.push_str(" | ");
                }
                content.push_str(&meta.join(" | "));
            }
            let mut res = SearchResult::basic("bingnews", url.to_string(), title, content);
            if let Some(src) = card
                .find("class=\"imagelink\"")
                .and_then(|i| card[i..].find("<img").map(|m| i + m))
                .and_then(|m| card[m..].find('>').map(|g| card[m..m + g].to_string()))
                .as_deref()
                .and_then(|img| crate::engine::attr(img, "src"))
            {
                res.thumbnail = if src.starts_with("https://www.bing.com") {
                    src.into()
                } else {
                    format!("https://www.bing.com/{src}")
                };
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
