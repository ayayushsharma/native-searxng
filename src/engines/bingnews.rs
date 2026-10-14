//! Bing News (`bing.com/news`) — HTML scrape, news.
//! `https://www.bing.com/news/infinitescrollajax?q=..&first=1`
//! Response: HTML with `<div class="newsitem">` cards each carrying
//! `<a class="title">`, `<div class="snippet">`, optional source/author
//! metadata and `<a class="imagelink"><img>` thumbnails.
//! First HTML-wave engine: parsed with `scraper` (already a dependency);
//! truncated bodies (96KB cap) parse as far as they go, never panic.
//! Fixture: `tests-fixture-bingnews-test.json` (HTTP 200, 6 cards, clean).

use scraper::{Html, Selector};

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
        let doc = Html::parse_document(body);
        let Ok(card_sel) = Selector::parse("div.newsitem") else {
            return vec![];
        };
        let title_sel = Selector::parse("a.title").ok();
        let snippet_sel = Selector::parse("div.snippet").ok();
        let source_sel = Selector::parse("div.source span[aria-label]").ok();
        let img_sel = Selector::parse("a.imagelink img").ok();
        let mut out = Vec::new();
        for card in doc.select(&card_sel).take(10) {
            let link = title_sel
                .as_ref()
                .and_then(|s| card.select(s).next());
            let Some(link) = link else { continue };
            let url = link.value().attr("href").unwrap_or_default();
            let title: String = link.text().collect::<Vec<_>>().join(" ");
            let title = norm(&title);
            if url.is_empty() || title.is_empty() {
                continue;
            }
            let mut content = snippet_sel
                .as_ref()
                .and_then(|s| card.select(s).next())
                .map(|n| norm(&n.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            // Metadata mirrors SearXNG: source aria-label + data-author.
            let mut meta = Vec::new();
            if let Some(span) = source_sel
                .as_ref()
                .and_then(|s| card.select(s).next())
            {
                if let Some(label) = span.value().attr("aria-label") {
                    if !label.trim().is_empty() {
                        meta.push(label.trim().to_string());
                    }
                }
            }
            if let Some(author) = link.value().attr("data-author") {
                if !author.trim().is_empty() {
                    meta.push(author.trim().to_string());
                }
            }
            if !meta.is_empty() {
                if !content.is_empty() {
                    content.push_str(" | ");
                }
                content.push_str(&meta.join(" | "));
            }
            let mut res = SearchResult::basic("bingnews", url.into(), title, content);
            if let Some(src) = img_sel
                .as_ref()
                .and_then(|s| card.select(s).next())
                .and_then(|img| img.value().attr("src"))
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
