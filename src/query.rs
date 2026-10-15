//! Query-string parsing for /search. Same params as SearXNG (see agentic/CONTRACT.md).

#[derive(Debug, Default)]
pub struct Query {
    pub q: String,
    pub format: String,
    pub pageno: u32,
    pub language: String,
    pub safesearch: u8,
    pub categories: String,
    pub engines: String,
}

fn percent_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let mut it = s.as_bytes().iter().peekable();
    while let Some(&b) = it.next() {
        if b == b'%' {
            let hi = it.next().copied().unwrap_or(b'0');
            let lo = it.next().copied().unwrap_or(b'0');
            out.push(hex_pair(hi, lo));
        } else if b == b'+' {
            out.push(b' ');
        } else {
            out.push(b);
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_pair(hi: u8, lo: u8) -> u8 {
    let hex = |c: u8| match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    };
    hex(hi) * 16 + hex(lo)
}

/// Single query-string param (percent-decoded), for non-/search routes.
pub fn param(url: &str, key: &str) -> String {
    let Some((_, qs)) = url.split_once('?') else {
        return String::new();
    };
    for pair in qs.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return percent_decode(v);
            }
        }
    }
    String::new()
}

pub fn parse(url: &str) -> Query {
    let mut q = Query {
        format: "html".into(),
        pageno: 1,
        ..Default::default()
    };
    let Some((_, qs)) = url.split_once('?') else {
        return q;
    };
    for pair in qs.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        let v = percent_decode(v);
        match k {
            "q" => q.q = v.trim().to_string(),
            "format" => {
                q.format = if v == "json" { "json".into() } else { "html".into() };
            }
            "pageno" => q.pageno = v.parse().unwrap_or(1).clamp(1, 3),
            "language" => q.language = v,
            "safesearch" => q.safesearch = v.parse().unwrap_or(0).clamp(0, 2),
            "categories" => q.categories = v,
            "engines" => q.engines = v,
            _ => {}
        }
    }
    q
}
