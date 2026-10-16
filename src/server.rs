//! Single-threaded sync server (tiny_http). One request at a time —
//! queue depth 1 is the RAM-flatness guarantee (PLAN.md §1).
//! Routes: / -> console (static web/ files via include_str!), /app.js,
//! /style.css, /api/config (capability manifest), /search -> pipeline,
//! /fetch -> page extract, /healthz, /opensearch.xml.

use crate::{config::Config, crypto, engine::SearchResult, engines, fetch, jina, mcp, merge, query, render, scrape};
use tiny_http::{Header, Method, Response, Server};

/// Static console: real files in `web/` (`index.html`, `app.js`,
/// `style.css`), embedded at COMPILE time — authors edit plain HTML/JS,
/// the binary stays one file (Android-safe, no runtime path lookups,
/// no per-request disk I/O). Only `/api/config` is dynamic JSON.
const INDEX_HTML: &str = include_str!("../web/index.html");
const APP_JS: &str = include_str!("../web/app.js");
const STYLE_CSS: &str = include_str!("../web/style.css");

/// Bearer gate (pure function — unit-tested below). `tokens` empty =
/// dev-open; main only allows that with explicit `auth=off` in conf.
/// Scheme match is case-insensitive (RFC 7235); credential compare is
/// constant-time (`crypto::ct_eq`). Comma-loaded tokens: any entry ok.
pub fn authorized(auth_header: Option<&str>, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return true;
    }
    let h = auth_header.unwrap_or("").trim();
    let Some((scheme, cred)) = h.split_once(|c: char| c.is_whitespace()) else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("bearer") {
        return false;
    }
    let cred = cred.trim();
    tokens.iter().any(|t| crypto::ct_eq(t.as_bytes(), cred.as_bytes()))
}

fn header<'a>(req: &'a tiny_http::Request, name: &str) -> Option<&'a str> {
    req.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}

/// Paths serving the static console + liveness: open WITHOUT a token
/// (a token-entry page that 401s is a locked key-box). Public facts
/// only — every data route stays gated.
fn open_path(path: &str) -> bool {
    matches!(path, "/healthz" | "/" | "/app.js" | "/style.css" | "/api/config")
}

/// Capability manifest for the console (`GET /api/config`). Built from
/// the LIVE registry + conf order + MCP schema fns — the page can never
/// rot behind the binary. `{:?}` on &str is valid JSON quoting here.
fn api_config(cfg: &Config, tokens: &[String]) -> String {
    let mut engines = Vec::new();
    let mut cats: Vec<String> = Vec::new();
    for name in &cfg.engines {
        if let Some(e) = engines::find(name) {
            let c = e.category().to_string();
            if !cats.contains(&c) {
                cats.push(c.clone());
            }
            engines.push(format!("{{\"name\":{:?},\"category\":{:?}}}", e.name(), c));
        }
    }
    format!(
        concat!(
            "{{\"authRequired\":{},\"version\":{:?},\"engines\":[{}],",
            "\"categories\":[{}],",
            "\"topKeys\":[\"query\",\"results\",\"answers\",\"corrections\",",
            "\"infoboxes\",\"suggestions\",\"unresponsive_engines\"],",
            "\"resultKeys\":[\"title\",\"url\",\"content\",\"engine\",\"category\",",
            "\"template\",\"publishedDate\",\"thumbnail\",\"iframe_src\",",
            "\"img_src\",\"audio_src\"],",
            "\"mcp\":{{\"search\":{},\"fetch\":{}}}}}"
        ),
        !tokens.is_empty(),
        env!("CARGO_PKG_VERSION"),
        engines.join(","),
        cats.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(","),
        mcp::search_tool(),
        mcp::fetch_tool(),
    )
}
/// Who to blame in the log on a 401. Prefers Cloudflare's connecting
/// IP — trustworthy here ONLY because the documented deployment binds
/// loopback, so the tunnel daemon is the sole possible client.
/// Never logs tokens, only the source + path.
fn client_ip(req: &tiny_http::Request) -> String {
    if let Some(cf) = header(req, "CF-Connecting-IP") {
        let cf = cf.trim();
        if !cf.is_empty() {
            return format!("cf:{cf}");
        }
    }
    req.remote_addr().map(|a| a.to_string()).unwrap_or_else(|| "-".into())
}

/// Panic isolation (item 5): one engine blowing up must not abort its
/// whole lane (pkggodev's byte-index panic once poisoned 4 engines).
/// The lane loop wraps each engine's fetch+parse in this; a panic
/// becomes an honest `unresponsive` entry and the slice continues.
/// `AssertUnwindSafe` because the lane closure borrows the shared
/// client — a panic here is a parser bug, not unwind-sensitive state.
fn guard<T>(f: impl FnOnce() -> T) -> Option<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).ok()
}

pub fn run(cfg: &Config, tokens: &[String]) {
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let server = Server::http(&addr).expect("bind server");
    let client = fetch::Client::new(cfg);
    for mut req in server.incoming_requests() {
        let url = req.url().to_string();
        let path = url.split('?').next().unwrap_or("/");
        // Auth gate: everything except the static console + liveness.
        // 401s are contract-shaped JSON; the log carries source + path
        // only, never the presented credential.
        if !open_path(path) && !authorized(header(&req, "Authorization"), tokens) {
            eprintln!("unauthorized {} {}", client_ip(&req), path);
            let ct = Header::from_bytes("Content-Type", b"application/json".as_slice()).unwrap();
            let wa = Header::from_bytes("WWW-Authenticate", b"Bearer".as_slice()).unwrap();
            let resp = Response::from_string(render::error_json("unauthorized"))
                .with_status_code(401)
                .with_header(ct)
                .with_header(wa);
            let _ = req.respond(resp);
            continue;
        }
        // Stateless Streamable HTTP: POST /mcp with a JSON-RPC body.
        // No SSE, no sessions — one body in, one JSON body out.
        // POST bodies capped at 64KB (tool args are small).
        if req.method() == &Method::Post && url == "/mcp" {
            // Chunked read with a 64KB ceiling (tool args are small).
            let mut body = Vec::new();
            let mut buf = [0u8; 8192];
            let mut ok = true;
            let reader = req.as_reader();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        body.extend_from_slice(&buf[..n]);
                        if body.len() > 65536 {
                            ok = false;
                            break;
                        }
                    }
                    Err(_) => {
                        ok = false;
                        break;
                    }
                }
            }
            let out = match (ok, String::from_utf8(body)) {
                (true, Ok(b)) => mcp::handle_http(cfg, &client, &b),
                _ => "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32700,\"message\":\"parse error\"}}".into(),
            };
            let header = Header::from_bytes("Content-Type", b"application/json".as_slice()).unwrap();
            let resp = Response::from_string(out).with_status_code(200).with_header(header);
            let _ = req.respond(resp);
            continue;
        }
        let (status, ctype, body) = route(cfg, &client, tokens, &url);
        let header = Header::from_bytes("Content-Type", ctype).unwrap();
        let resp = Response::from_string(body).with_status_code(status).with_header(header);
        let _ = req.respond(resp);
    }
}

fn route(cfg: &Config, client: &fetch::Client, tokens: &[String], url: &str) -> (u16, &'static [u8], String) {
    let path = url.split('?').next().unwrap_or("/");
    match path {
        "/" => (200, b"text/html", INDEX_HTML.into()),
        "/app.js" => (200, b"text/javascript", APP_JS.into()),
        "/style.css" => (200, b"text/css", STYLE_CSS.into()),
        "/api/config" => (200, b"application/json", api_config(cfg, tokens)),
        "/healthz" => (200, b"text/plain", "ok".into()),
        "/opensearch.xml" => (200, b"text/xml", OPENSEARCH.into()),
        "/search" => search(cfg, client, url),
        "/fetch" => fetch_page(client, url),
        _ => (404, b"application/json", render::error_json("not found")),
    }
}

/// MCP reuse seam: same pipeline, no HTTP. The MCP server synthesizes
/// these URLs internally so both transports can never drift.
pub fn search_pub(cfg: &Config, client: &fetch::Client, url: &str) -> (u16, String) {
    let (s, _, b) = search(cfg, client, url);
    (s, b)
}

pub fn fetch_pub(client: &fetch::Client, url: &str) -> (u16, String) {
    let (s, _, b) = fetch_page(client, url);
    (s, b)
}

const OPENSEARCH: &str = r#"<?xml version="1.0"?><OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/"><ShortName>SearXNG</ShortName><Url type="text/html" template="/?q={searchTerms}"/></OpenSearchDescription>"#;

/// `/fetch?url=<http(s)://…>` — native scrape first (no third party),
/// Jina Reader fallback for bot-walled/JS-shell pages.
/// Keyless works (free tier); SEARXNG_API_KEY_JINA raises limits.
/// Failures are `{"error":…}`, never key material, never bodies.
fn fetch_page(client: &fetch::Client, url: &str) -> (u16, &'static [u8], String) {
    let target = query::param(url, "url");
    if target.is_empty() {
        return (400, b"application/json", render::error_json("No url"));
    }
    if let Ok(r) = scrape::scrape(client, &target) {
        return (200, b"application/json", render::fetch_json(&target, &r.title, &r.content, &r.links));
    }
    match jina::fetch(client, &target) {
        Ok(r) if scrape::is_walled(&r.title, &r.content) => (
            502,
            b"application/json",
            render::error_json("page is bot-walled (reader returned a challenge)"),
        ),
        Ok(r) => (200, b"application/json", render::fetch_json(&target, &r.title, &r.content, &r.links)),
        Err(e) => (502, b"application/json", render::error_json(&e)),
    }
}

fn search(cfg: &Config, client: &fetch::Client, url: &str) -> (u16, &'static [u8], String) {
    let q = query::parse(url);
    if q.q.is_empty() {
        return (400, b"application/json", render::error_json("No query"));
    }
    // Strangler: engine override for unknown names -> Python reference.
    // Native categories: general, it, science, news, packages, videos,
    // dictionaries, weather.
    let want = if q.categories.is_empty() {
        "general"
    } else {
        q.categories.as_str()
    };
    let native = matches!(want, "general" | "it" | "science" | "news" | "packages" | "videos" | "dictionaries" | "weather");
    // engines= filter (SearXNG compat): comma list restricts natives.
    // Unknown names -> whole query proxies (strangler owns the unknown).
    let filter: Vec<&str> = q.engines.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if (!filter.is_empty() && filter.iter().any(|n| engines::find(n).is_none())) || !native {
        if !cfg.proxy_to.is_empty() {
            return match client.proxy(&cfg.proxy_to, url) {
                Ok((s, b)) => (s, b"application/json", b),
                Err(e) => (502, b"application/json", render::error_json(&e)),
            };
        }
        return (503, b"application/json", render::error_json("category not ported yet"));
    }

    // Resolve + gate first (order = cfg order = merge weight order).
    let mut work: Vec<Box<dyn crate::engine::Engine>> = Vec::new();
    let mut unresponsive: Vec<(String, String)> = Vec::new();
    for name in &cfg.engines {
        let Some(engine) = engines::find(name) else {
            unresponsive.push((name.clone(), "unknown engine".into()));
            continue;
        };
        if engine.category() != want {
            continue;
        }
        if !filter.is_empty() && !filter.contains(&name.as_str()) {
            continue;
        }
        work.push(engine);
    }
    // Lanes: 4 scoped threads (GAP-PLAN P2), each running a contiguous
    // slice of engines sequentially. Joined in cfg order so merge
    // weights never shift. Each lane owns its chain state (prev bodies,
    // cookie jar); only the thread-safe client (shared ureq pool) and
    // the read-only query are shared. Latency drops from SUM(legs) to
    // ~SUM(slowest lane): ~20s worst-case general queries become ~4s.
    // Peak transient: one body cap per lane (4 x 96KB); 4 malloc arenas
    // instead of 12 (per-engine threads cost +3MB dev RSS — measured).
    let mut lists: Vec<Vec<SearchResult>> = Vec::new();
    // Copy-friendly lane captures (move closures would otherwise try to
    // move the String field `q.q` into the first lane).
    let qq: &str = q.q.as_str();
    let pageno: u32 = q.pageno;
    std::thread::scope(|s| {
        let lanes = 4usize;
        let chunk = work.len().div_ceil(lanes);
        let mut handles = Vec::with_capacity(lanes);
        for slice in work.chunks(chunk) {
            handles.push(s.spawn(move || {
                let mut out: Vec<(Vec<SearchResult>, Option<(String, String)>)> = vec![];
                // One engine at a time per lane; the chain below returns
                // per engine via this closure (a bare `return` would
                // abort the whole lane, skipping the slice remainder).
                let run = |engine: &dyn crate::engine::Engine| -> (
                    Vec<SearchResult>,
                    Option<(String, String)>,
                ) {
        // Request chain: the engine describes each fetch (GET or POST)
        // from all previous responses; max 3 rounds (chatnoir needs 3).
        // Every leg is gated the same way: non-200, challenge page, or
        // empty body reports honestly (iron rule 5) — never parsed.
        let mut prev: Vec<crate::engine::InResponse> = vec![];
        // Cookie jar: session legs (chatnoir ?init, fireball settings)
        // need the cookies minted by earlier legs — the oracle passes
        // `response.cookies` forward, so do we (per query, never shared).
        let mut jar: Vec<(String, String)> = vec![];
        let last_body: Option<String> = loop {
            let Some(req) = engine.request(qq, pageno, &prev) else {
                break if prev.is_empty() {
                    return (vec![], Some((engine.name().to_string(), "no request".into())));
                } else if prev.len() < engine.rounds() {
                    // Chain stopped early on scaffolding — flow broke.
                    // (Single-step engines stop after leg 1 by design.)
                    return (vec![], Some((engine.name().to_string(), "session flow failed".into())));
                } else {
                    prev.pop().map(|r| r.body)
                };
            };
            if prev.len() >= 3 {
                return (vec![], Some((engine.name().to_string(), "chain too long".into())));
            }
            let mut req = req;
            if !jar.is_empty() {
                let header = jar
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect::<Vec<_>>()
                    .join("; ");
                req.headers.push(("Cookie".into(), header));
            }
            match client.execute_capped(&req, engine.body_cap_kb()) {
                Ok(r) if r.status != 200 => {
                    return (vec![], Some((engine.name().to_string(), format!("http {}", r.status))));
                }
                Ok(r) if r.body.trim().is_empty() => {
                    return (vec![], Some((engine.name().to_string(), "empty response".into())));
                }
                Ok(r) => {
                    if let Some(reason) = engine.challenge(&r.body) {
                        return (vec![], Some((engine.name().to_string(), reason)));
                    }
                    for c in &r.cookies {
                        let pair = c.split(';').next().unwrap_or("");
                        if let Some((k, v)) = pair.split_once('=') {
                            let (k, v) = (k.trim().to_string(), v.trim().to_string());
                            if k.is_empty() || v.is_empty() {
                                continue;
                            }
                            if let Some(slot) = jar.iter_mut().find(|(ek, _)| *ek == k) {
                                slot.1 = v;
                            } else {
                                jar.push((k, v));
                            }
                        }
                    }
                    prev.push(r);
                }
                Err(e) => {
                    return (vec![], Some((engine.name().to_string(), e)));
                }
            }
        };
        if let Some(body) = last_body {
            // Stamp the served category (SearXNG tags each result with
            // its search category; parsers default general).
            let mut v = engine.parse_scored(&body, qq);
            for r in &mut v {
                r.category = want.into();
            }
                    (v, None)
                } else {
                    (vec![], None)
                }
                }; // end run
                for engine in slice {
                    match guard(|| run(engine.as_ref())) {
                        Some(pair) => out.push(pair),
                        None => out.push((
                            vec![],
                            Some((engine.name().to_string(), "panicked".into())),
                        )),
                    }
                }
                out
            })) // end lane (close closure, spawn, push)
        }
        // Join in cfg order: merge weights never shift across runs.
        for h in handles {
            let lane = h.join().unwrap_or_else(|_| {
                vec![(vec![], Some(("lane".to_string(), "panicked".to_string())))]
            });
            for (v, u) in lane {
                if let Some(u) = u {
                    unresponsive.push(u);
                }
                lists.push(v);
            }
        }
    });
    let results = merge::merge(lists);
    if q.format == "json" {
        (200, b"application/json", render::json(&q.q, &results, &unresponsive))
    } else {
        (200, b"text/html", render::html(&q.q, &results))
    }
}

#[cfg(test)]
mod tests {
    use super::authorized;

    fn toks() -> Vec<String> {
        vec!["s3cr3t".into(), "r0tat3d".into()]
    }

    #[test]
    fn auth_gate_shapes() {
        let t = toks();
        assert!(authorized(Some("Bearer s3cr3t"), &t));
        assert!(authorized(Some("Bearer r0tat3d"), &t)); // rotation entry
        assert!(authorized(Some("bearer s3cr3t"), &t)); // scheme case
        assert!(authorized(Some("Bearer  s3cr3t  "), &t)); // OWS
        assert!(!authorized(None, &t));
        assert!(!authorized(Some(""), &t));
        assert!(!authorized(Some("Bearer wrong"), &t));
        assert!(!authorized(Some("Bearer s3cr3"), &t)); // prefix != equal
        assert!(!authorized(Some("Bearer s3cr3t!"), &t));
        assert!(!authorized(Some("Basic c2Vjcm4n"), &t)); // wrong scheme
        assert!(!authorized(Some("s3cr3t"), &t)); // no scheme
        // Dev-open only when main allows it (explicit auth=off).
        assert!(authorized(None, &[]));
    }

    #[test]
    fn guard_isolates_panics() {
        assert_eq!(super::guard(|| 42), Some(42));
        let r: Option<()> = super::guard(|| panic!("boom"));
        assert!(r.is_none());
    }

    #[test]
    fn console_static_files_embedded() {
        // Real web/ files, compiled in: authors edit HTML/JS, the
        // binary stays one file. Markers both sides must agree on.
        assert!(super::INDEX_HTML.contains("/app.js"));
        assert!(super::INDEX_HTML.contains("/style.css"));
        assert!(super::INDEX_HTML.contains("token"));
        assert!(super::APP_JS.contains("/api/config"));
        assert!(super::APP_JS.contains("localStorage"));
        assert!(super::STYLE_CSS.contains(":root"));
        for p in ["/", "/app.js", "/style.css", "/api/config", "/healthz"] {
            assert!(super::open_path(p), "{p} must be token-free");
        }
        assert!(!super::open_path("/search"));
        assert!(!super::open_path("/mcp"));
    }

    #[test]
    fn api_config_shape() {
        let cfg = crate::config::Config::default();
        let authed = super::api_config(&cfg, &["t".into()]);
        let open = super::api_config(&cfg, &[]);
        let v: serde_json::Value = serde_json::from_str(&authed).expect("valid JSON");
        assert_eq!(v["authRequired"], true);
        assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(v["engines"][0]["name"], "wikipedia");
        assert_eq!(v["engines"][0]["category"], "general");
        assert!(v["categories"].as_array().unwrap().contains(&serde_json::json!("general")));
        assert!(v["mcp"]["search"]["name"] == "search");
        assert!(v["mcp"]["fetch"]["name"] == "fetch");
        let o: serde_json::Value = serde_json::from_str(&open).expect("valid JSON");
        assert_eq!(o["authRequired"], false);
    }
}
