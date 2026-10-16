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
    /// Query-aware parse for engines whose response is an index, not
    /// answers (Sphinx `searchindex.js`, cheat.sh topics). Default:
    /// plain `parse` (back-compatible — every existing engine and test
    /// is untouched). The server always calls this, never `parse`.
    fn parse_scored(&self, body: &str, _query: &str) -> Vec<SearchResult> {
        self.parse(body)
    }
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
    fn request(&self, query: &str, pageno: u32, prev: &[InResponse]) -> Option<OutRequest> {
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
        Self {
            url,
            method: Method::Get,
            form: vec![],
            json: None,
            headers: vec![],
        }
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
        let n = digits.bytes().take_while(|b| b.is_ascii_hexdigit()).count();
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

/// Attribute value from a raw tag slice (`href="..."`, either quote).
/// Returns None when the attribute is absent; never panics.
pub fn attr(tag: &str, name: &str) -> Option<String> {
    for q in ['"', '\''] {
        let pat = format!("{name}={q}");
        if let Some(i) = tag.find(&pat) {
            let rest = &tag[i + pat.len()..];
            let end = rest.find(q).unwrap_or(rest.len());
            return Some(unescape(&rest[..end]));
        }
    }
    None
}

/// Civil-date math without deps (Howard Hinnant's algorithms):
/// `days_from_civil` converts YYYY-MM-DD to days since 1970-01-01,
/// `civil_from_days` converts back. `today_days` is now (UTC, via the
/// wall clock — a day off at timezone edges is noise for ranking).
/// All three feed recency: rank() boosts fresh news, engines convert
/// relative dates ("2 hours ago") to absolute ones at parse time.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Inverse of [`days_from_civil`]: days since epoch → (year, month, day).
pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Today as days since epoch. Clock failure → 0 (recency then
/// yields no boost rather than a wrong one).
pub fn today_days() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| (d.as_secs() / 86400) as i64)
        .unwrap_or(0)
}

/// Leading `YYYY-MM-DD` (or full ISO datetime) → days. Anything else
/// → None (no boost rather than a mis-boost).
pub fn parse_iso_date(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 10 {
        return None;
    }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let y: i64 = s[0..4].parse().ok()?;
    let m: i64 = s[5..7].parse().ok()?;
    let d: i64 = s[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// Minimal DuckDuckGo `/html` result scanner (shared by `ddg` +
/// `ddgnews`; replaces `scraper` + `encoding_rs` for these two).
/// DDG serves UTF-8, so no transcoding is needed. Shape (either tag):
/// `<a class="result__a" href="URL">TITLE</a>` then
/// `<a|div class="result__snippet...">SNIPPET</a|div>`.
/// Malformed cards are skipped; never panics.
pub fn ddg_parse(body: &str, engine: &'static str) -> Vec<SearchResult> {
    let mut out = Vec::new();
    let mut rest = body;
    while out.len() < 10 {
        let Some(ai) = rest.find("result__a\"") else {
            break;
        };
        let Some(tag_start) = rest[..ai].rfind("<a") else {
            break;
        };
        // Whole anchor tag (href sits after the class in DDG's HTML).
        let tag_end = rest[ai..].find('>').map(|g| ai + g).unwrap_or(rest.len());
        let tag = &rest[tag_start..tag_end];
        let url = attr(tag, "href").unwrap_or_default();
        let text_start = tag_end + 1;
        let text_end = rest[text_start..]
            .find("</a>")
            .map(|e| text_start + e)
            .unwrap_or(rest.len());
        let title = strip_tags(&rest[text_start..text_end]);
        rest = &rest[text_end..];
        if url.trim().is_empty() || title.is_empty() {
            continue;
        }
        // Snippet: nearest `result__snippet` element before the next link.
        let next_link = rest.find("result__a\"").unwrap_or(rest.len());
        let window = &rest[..next_link];
        let content = match window.find("result__snippet") {
            Some(si) => {
                // Nearest opener before the marker decides the closer.
                let back = &window[..si];
                let (a_pos, d_pos) = (back.rfind("<a"), back.rfind("<div"));
                let close_tag = match (a_pos, d_pos) {
                    (Some(a), Some(d)) => {
                        if d > a {
                            "</div>"
                        } else {
                            "</a>"
                        }
                    }
                    (Some(_), None) => "</a>",
                    _ => "</div>",
                };
                match window[si..].find('>') {
                    Some(g) => {
                        let cs = si + g + 1;
                        let ce = window[cs..]
                            .find(close_tag)
                            .map(|e| cs + e)
                            .unwrap_or(window.len());
                        strip_tags(&window[cs..ce])
                    }
                    None => String::new(),
                }
            }
            None => String::new(),
        };
        out.push(SearchResult::basic(
            engine,
            url.trim().to_string(),
            title,
            content,
        ));
    }
    out
}

/// Normalize for dedup: lowercase, strip scheme noise, trailing slash.
/// Dedup key: scheme/www/case/trailing-slash PLUS fragment and
/// tracking params (?utm_*, fbclid, gclid, ref, si...) — the same
/// article shared with different trackers must collapse to one row.
/// Meaningful params (?v=, ?q=, ?id=) are kept: only a fixed tracker
/// list is stripped, the rest of the query stays in the key.
const TRACKERS: &[&str] = &[
    "fbclid",
    "gclid",
    "gclsrc",
    "ref",
    "ref_src",
    "ref_url",
    "si",
    "mc_cid",
    "mc_eid",
    "igshid",
    "vero_conv",
    "vero_id",
];

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
    if let Some(h) = s.find('#') {
        s.truncate(h);
    }
    if let Some(q) = s.find('?') {
        let (base, qs) = s.split_at(q);
        let kept: Vec<&str> = qs[1..]
            .split('&')
            .filter(|p| {
                let k = p.split('=').next().unwrap_or("");
                !(k.starts_with("utm_") || TRACKERS.contains(&k))
            })
            .collect();
        s = if kept.is_empty() {
            base.to_string()
        } else {
            format!("{}?{}", base, kept.join("&"))
        };
    }
    while s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    s
}
