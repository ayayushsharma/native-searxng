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
        }
    }
}

#[allow(dead_code)]
pub trait Engine {
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
