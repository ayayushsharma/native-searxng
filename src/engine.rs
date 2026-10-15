//! Engine trait + SearXNG-compatible result shape (exact JSON key names).

use serde::Serialize;

/// One general result. Serializes with SearXNG's key names so existing
/// JSON consumers work unmodified (see agentic/CONTRACT.md).
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub engine: String,
    pub engines: Vec<String>,
    #[serde(rename = "publishedDate")]
    pub published_date: Option<String>,
    pub positions: Vec<usize>,
    pub score: f64,
    pub category: String,
    pub template: String,
    pub img_src: String,
    pub thumbnail: String,
    pub iframe_src: String,
    pub audio_src: String,
}

impl SearchResult {
    pub fn basic(engine: &str, url: String, title: String, content: String) -> Self {
        Self {
            url,
            title,
            content,
            engine: engine.into(),
            engines: vec![engine.into()],
            published_date: None,
            positions: vec![],
            score: 0.0,
            category: "general".into(),
            template: "default.html".into(),
            img_src: String::new(),
            thumbnail: String::new(),
            iframe_src: String::new(),
            audio_src: String::new(),
        }
    }
}

#[allow(dead_code)]
pub trait Engine: Send + Sync {
    fn name(&self) -> &'static str;
    fn build_url(&self, q: &str, pageno: u32) -> String;
    /// Parse a (possibly truncated) response body into results. Never panics
    /// on malformed input — skips what it can't parse.
    fn parse(&self, body: &str) -> Vec<SearchResult>;
    /// Bot-wall / challenge detection (iron rule 5). `Some(reason)` means the
    /// body is a captcha/challenge page: caller reports 0 results plus an
    /// `unresponsive_engines` entry instead of silent empties. Default: none.
    fn challenge(&self, _body: &str) -> Option<String> {
        None
    }
    /// Native category this engine serves (`general` default). The server
    /// only runs engines matching the requested category; anything else
    /// stays behind the strangler.
    fn category(&self) -> &'static str {
        "general"
    }
    /// Legs in this engine's request chain (1 default). If `request()`
    /// stops before this many responses exist, the scaffolding flow broke
    /// — the server reports unresponsive instead of parsing scaffolding
    /// as results. PubMed/boardreader/gmx = 2, chatnoir = 3.
    fn rounds(&self) -> usize {
        1
    }
    /// Response cap override in KB (None = server default). Fat-JSON
    /// APIs (OpenAlex works ~250KB with inverted abstracts) need room;
    /// the buffer is transient per fetch, freed after parse.
    fn body_cap_kb(&self) -> Option<usize> {
        None
    }
    /// Next fetch in a stateless chain. `prev` holds all responses so far
    /// (empty = first request). Return None to stop and parse the last
    /// body. Default: single GET of `build_url`, then stop.
    /// `query`/`pageno` are passed through because later legs usually
    /// re-state them (boardreader's return.php needs both + session).
    fn request(
        &self,
        query: &str,
        pageno: u32,
        prev: &[InResponse],
    ) -> Option<OutRequest> {
        if prev.is_empty() {
            return Some(OutRequest::get(self.build_url(query, pageno)));
        }
        None
    }
}

/// A request the server should execute. Multi-step flows (chatnoir:
/// GET home → POST ?init → POST _search) are expressed as a chain:
/// `request()` sees all previous responses and returns the next fetch,
/// or None to stop and parse the last body. Capped at 3 rounds.
#[derive(Debug)]
pub enum Method {
    Get,
    Post,
}

#[derive(Debug)]
pub struct OutRequest {
    pub url: String,
    pub method: Method,
    /// POST application/x-www-form-urlencoded pairs (fireball settings).
    pub form: Vec<(String, String)>,
    /// POST application/json body (chatnoir search). Beats `form` if set.
    pub json: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl OutRequest {
    pub fn get(url: String) -> Self {
        Self { url, method: Method::Get, form: vec![], json: None, headers: vec![] }
    }
}

/// What came back: status + raw Set-Cookie values + capped body.
#[derive(Debug)]
pub struct InResponse {
    pub status: u16,
    pub cookies: Vec<String>,
    pub body: String,
}

/// `name=value` out of `Set-Cookie: name=value; Path=/; …` values.
/// Engines pick what their next leg needs (chatnoir's csrftoken).
pub fn cookie(cookies: &[String], name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    for c in cookies {
        if let Some(rest) = c.strip_prefix(&prefix) {
            let end = rest.find(';').unwrap_or(rest.len());
            let v = rest[..end].trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Minimal HTML-entity unescape for engine titles/snippets
/// (`&quot;`, `&amp;`, …). Never panics; unknown entities pass through.
/// HTML-entity decoding for titles/snippets (oracle `html_to_text` twin):
/// named entities plus decimal/hex numeric refs (`&#47;`, `&#x2F;`).
/// Unknown entities pass through untouched; never panics.
pub fn unescape(s: &str) -> String {
    let named = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'");
    let mut out = String::with_capacity(named.len());
    let mut rest = named.as_str();
    while let Some(i) = rest.find("&#") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 2..];
        let (digits, hex) = match tail.strip_prefix('x').or_else(|| tail.strip_prefix('X')) {
            Some(h) => (h, true),
            None => (tail, false),
        };
        let n = digits
            .bytes()
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        let after = &digits[n..];
        if n > 0 && after.starts_with(';') {
            let num = &digits[..n];
            let code = if hex {
                u32::from_str_radix(num, 16).ok()
            } else if num.bytes().all(|b| b.is_ascii_digit()) {
                num.parse::<u32>().ok()
            } else {
                None
            };
            match code.and_then(char::from_u32) {
                Some(c) => out.push(c),
                None => out.push_str(&rest[i..i + 2 + n + 1]),
            }
            rest = &after[1..];
        } else {
            out.push_str("&#");
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// Strip `<tags>` and collapse whitespace (shared html_to_text twin).
/// Unknown/malformed markup passes through as text; never panics.
pub fn strip_tags(s: &str) -> String {
    let mut out = unescape(s);
    loop {
        let Some(a) = out.find('<') else { break };
        let Some(b) = out[a..].find('>') else { break };
        out.replace_range(a..a + b + 1, " ");
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normalize for dedup: lowercase, strip scheme noise, trailing slash.
pub fn normalize_url(u: &str) -> String {
    let mut s = u.trim().to_lowercase();
    for prefix in ["https://", "http://"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest.to_string();
            break;
        }
    }
    if s.starts_with("www.") {
        s = s[4..].to_string();
    }
    while s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    s
}
