//! Single-threaded sync server (tiny_http). One request at a time —
//! queue depth 1 is the RAM-flatness guarantee (PLAN.md §1).
//! Routes: / -> form, /search -> pipeline, /fetch -> page extract,
//! /healthz, /opensearch.xml.

use crate::{config::Config, engine::SearchResult, engines, fetch, jina, merge, query, render, scrape};
use tiny_http::{Header, Response, Server};

pub fn run(cfg: &Config) {
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let server = Server::http(&addr).expect("bind server");
    let client = fetch::Client::new(cfg);
    for req in server.incoming_requests() {
        let url = req.url().to_string();
        let (status, ctype, body) = route(cfg, &client, &url);
        let header = Header::from_bytes("Content-Type", ctype).unwrap();
        let resp = Response::from_string(body).with_status_code(status).with_header(header);
        let _ = req.respond(resp);
    }
}

fn route(cfg: &Config, client: &fetch::Client, url: &str) -> (u16, &'static [u8], String) {
    let path = url.split('?').next().unwrap_or("/");
    match path {
        "/" => (200, b"text/html", render::html("", &[])),
        "/healthz" => (200, b"text/plain", "ok".into()),
        "/opensearch.xml" => (200, b"text/xml", OPENSEARCH.into()),
        "/search" => search(cfg, client, url),
        "/fetch" => fetch_page(client, url),
        _ => (404, b"application/json", render::error_json("not found")),
    }
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
    // Native categories: general, it, science, news, packages.
    let want = if q.categories.is_empty() {
        "general"
    } else {
        q.categories.as_str()
    };
    let native = matches!(want, "general" | "it" | "science" | "news" | "packages");
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
            let mut v = engine.parse(&body);
            for r in &mut v {
                r.category = want.into();
            }
                    (v, None)
                } else {
                    (vec![], None)
                }
                }; // end run
                for engine in slice {
                    out.push(run(engine.as_ref()));
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
