//! pkg.go.dev (Go packages) — server-rendered search cards (DEV parity leftover).
//! `GET pkg.go.dev/search?q=..` → `SearchSnippet` cards with stable
//! `data-test-id="snippet-title|snippet-synopsis"` hooks. No key, no
//! JS, 76KB pages. `packages.html` template.

use crate::engine::{Engine, SearchResult};

pub struct PkgGoDev;

impl Engine for PkgGoDev {
    fn name(&self) -> &'static str {
        "pkggodev"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, pageno: u32) -> String {
        if pageno > 1 {
            format!("https://pkg.go.dev/search?q={}&page={pageno}", encode(q))
        } else {
            format!("https://pkg.go.dev/search?q={}", encode(q))
        }
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let mut out = Vec::new();
        // One card per search result; title anchor carries the path.
        let mut rest = body;
        while let Some(a) = find_title_anchor(rest) {
            let (href, name, tail) = a;
            rest = tail;
            let syn = snippet_after(rest);
            let name = collapse(&strip(&name));
            if name.is_empty() || !href.starts_with('/') {
                continue;
            }
            let mut content = collapse(&strip(&syn));
            if content.chars().count() > 400 {
                content = content.chars().take(400).collect::<String>() + "…";
            }
            out.push(SearchResult::basic(
                "pkggodev",
                format!("https://pkg.go.dev{href}"),
                name,
                content,
            ));
            if out.len() >= 10 {
                break;
            }
        }
        out
    }
}

/// `<a href="..." ... data-test-id="snippet-title" ...>name…</a>`.
fn find_title_anchor(html: &str) -> Option<(String, String, &str)> {
    let hook = "data-test-id=\"snippet-title\"";
    let hi = html.find(hook)?;
    let a_start = html[..hi].rfind("<a ")?;
    let tag_end = html[a_start..].find('>')? + a_start;
    let tag = &html[a_start..tag_end];
    let href = attr(tag, "href")?;
    let close = html[tag_end..].find("</a>")? + tag_end;
    Some((
        href,
        html[tag_end + 1..close].to_string(),
        &html[close + 4..],
    ))
}

fn snippet_after(html: &str) -> String {
    let hook = "data-test-id=\"snippet-synopsis\"";
    let Some(hi) = html.find(hook) else {
        return String::new();
    };
    // Synopsis <p> ends at </p>; the 2KB cap bounds hostile pages.
    // (Byte indices: hi/gt land on ASCII, slicing is boundary-safe.)
    let window = &html[..html.len().min(hi + 2048)];
    let Some(gt) = window[hi..].find('>') else {
        return String::new();
    };
    let start = hi + gt + 1;
    let end = window[start..]
        .find("</p>")
        .map(|i| start + i)
        .unwrap_or(window.len());
    window[start..end].to_string()
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let i = tag.find(&key)? + key.len();
    let j = tag[i..].find('"')? + i;
    Some(tag[i..j].to_string())
}

fn strip(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

fn collapse(s: &str) -> String {
    // Drop the "(path)" echo pkg.go.dev appends inside the anchor.
    let s = s.split('(').next().unwrap_or(s);
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
