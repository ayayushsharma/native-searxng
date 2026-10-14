//! Single-threaded sync server (tiny_http). One request at a time —
//! queue depth 1 is the RAM-flatness guarantee (PLAN.md §1).
//! Routes: / -> form, /search -> pipeline, /healthz, /opensearch.xml.

use crate::{config::Config, engine::SearchResult, engines, fetch, merge, query, render};
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
        _ => (404, b"application/json", render::error_json("not found")),
    }
}

const OPENSEARCH: &str = r#"<?xml version="1.0"?><OpenSearchDescription xmlns="http://a9.com/-/spec/opensearch/1.1/"><ShortName>SearXNG</ShortName><Url type="text/html" template="/?q={searchTerms}"/></OpenSearchDescription>"#;

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
    if (!q.engines.is_empty() && engines::find(&q.engines).is_none()) || !native {
        if !cfg.proxy_to.is_empty() {
            return match client.proxy(&cfg.proxy_to, url) {
                Ok((s, b)) => (s, b"application/json", b),
                Err(e) => (502, b"application/json", render::error_json(&e)),
            };
        }
        return (503, b"application/json", render::error_json("category not ported yet"));
    }

    let mut lists: Vec<Vec<SearchResult>> = Vec::new();
    let mut unresponsive: Vec<(String, String)> = Vec::new();
    for name in &cfg.engines {
        let Some(engine) = engines::find(name) else {
            unresponsive.push((name.clone(), "unknown engine".into()));
            continue;
        };
        // Category gate: only run engines serving the requested category.
        if engine.category() != want {
            continue;
        }
        let fetch_url = engine.build_url(&q.q, q.pageno);
        match client.get(&fetch_url) {
            Ok(body) => {
                // Challenge/captcha pages -> 0 results + unresponsive entry
                // (iron rule 5), never silent empties or garbage.
                if let Some(reason) = engine.challenge(&body) {
                    unresponsive.push((name.clone(), reason));
                } else {
                    // Two-step engines: at most one follow-up fetch, then
                    // parse the final body (PubMed esearch -> esummary).
                    let final_body;
                    let body = match engine.follow_up(&body) {
                        Some(url2) => match client.get(&url2) {
                            Ok(b2) => {
                                final_body = b2;
                                &final_body
                            }
                            Err(e) => {
                                unresponsive.push((name.clone(), e));
                                continue;
                            }
                        },
                        None => &body,
                    };
                    // Stamp the served category (SearXNG tags each result
                    // with its search category; parsers default general).
                    let mut v = engine.parse(body);
                    for r in &mut v {
                        r.category = want.into();
                    }
                    lists.push(v);
                }
            }
            Err(e) => unresponsive.push((name.clone(), e)),
        }
    }
    let results = merge::merge(lists, cfg.max_results);
    if q.format == "json" {
        (200, b"application/json", render::json(&q.q, &results, &unresponsive))
    } else {
        (200, b"text/html", render::html(&q.q, &results))
    }
}
