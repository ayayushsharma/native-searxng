//! MCP server, stdio transport (queued integration — design in DEV-SEARCH).
//!
//! Stateless JSON-RPC 2.0 over stdin/stdout (newline-delimited, no
//! Content-Length framing). Pinned protocol version `2025-06-18`
//! (`--help` says so; we answer with ours regardless of the client's).
//! Two tools, no resources/prompts/sampling: `search` (the `/search`
//! pipeline) and `fetch` (the `/fetch` pipeline). Tool calls reuse the
//! exact server functions, so HTTP and MCP can never drift.
//!
//! Run: `native-searxng --mcp`. EOF on stdin exits 0. Zero new crates.
//!
//! Conformance gate: pi speaks MCP natively (see pi docs/mcp.md), so
//! this replaces the bespoke pi TS adapter. Verify with the Inspector
//! before claiming compatibility.

use crate::{config::Config, fetch, server};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

pub fn run_stdio(cfg: &Config) {
    let client = fetch::Client::new(cfg);
    let stdin = std::io::stdin();
    let out = std::io::stdout();
    let mut out = out.lock();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                respond(
                    &mut out,
                    json!({"jsonrpc": "2.0", "id": null,
                    "error": {"code": -32700, "message": "parse error"}}),
                );
                continue;
            }
        };
        if let Some(resp) = dispatch(cfg, &client, &req) {
            respond(&mut out, resp);
        }
    }
}

/// JSON-RPC dispatcher shared by stdio and Streamable HTTP.
pub fn dispatch(cfg: &Config, client: &fetch::Client, req: &Value) -> Option<Value> {
    let method = req.get("method")?.as_str()?;
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    // Notifications carry no id: acknowledge with silence.
    if req.get("id").is_none() {
        return None;
    }
    crate::debug!(
        "mcp {} {}",
        method,
        params.get("name").and_then(|n| n.as_str()).unwrap_or("")
    );
    let result = match method {
        "initialize" => Some(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "native-searxng", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" => Some(json!({})),
        "tools/list" => Some(json!({"tools": [search_tool(), fetch_tool()]})),
        "tools/call" => {
            // Unknown tools are a top-level JSON-RPC error (-32602), not
            // a result body (conformance probe caught the nesting bug).
            let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if !matches!(name, "search" | "fetch") {
                return Some(json!({"jsonrpc": "2.0", "id": id,
                    "error": {"code": -32602, "message": format!("unknown tool {name}")}}));
            }
            Some(call_tool(cfg, client, &params))
        }
        _ => {
            return Some(json!({"jsonrpc": "2.0", "id": id,
                "error": {"code": -32601, "message": format!("unknown method {method}")}}));
        }
    };
    result.map(|r| json!({"jsonrpc": "2.0", "id": id, "result": r}))
}

/// Streamable HTTP entry: one POST body → one JSON response (no SSE;
/// our latencies don't need streaming). Stateless: no session IDs.
/// Notifications return `null` result (nothing to resume). Errors that
/// aren't JSON-RPC-shaped get -32600.
pub fn handle_http(cfg: &Config, client: &fetch::Client, body: &str) -> String {
    let req: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => {
            return serde_json::to_string(&json!({"jsonrpc": "2.0", "id": null,
            "error": {"code": -32700, "message": "parse error"}}))
            .unwrap_or_default()
        }
    };
    match dispatch(cfg, client, &req) {
        Some(resp) => serde_json::to_string(&resp).unwrap_or_default(),
        None => serde_json::to_string(&json!({"jsonrpc": "2.0", "id": null, "result": null}))
            .unwrap_or_default(),
    }
}

pub fn search_tool() -> Value {
    json!({
        "name": "search",
        "description": "Search the web across native engines (docs, packages, Q&A, papers, news). Returns numbered title/URL/content results.",
        "inputSchema": {
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {"type": "string", "description": "Search query"},
                "categories": {"type": "string", "description": "general|it|science|news|packages (default general)"},
                "max_results": {"type": "integer", "description": "Max results, default 5"},
                "pageno": {"type": "integer", "description": "Page number, default 1"},
                "engines": {"type": "string", "description": "Comma-separated engine filter, e.g. github,arxiv"},
            },
        },
    })
}

pub fn fetch_tool() -> Value {
    json!({
        "name": "fetch",
        "description": "Fetch a page URL and extract title, text content and links (native scrape first, reader fallback).",
        "inputSchema": {
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": {"type": "string", "description": "http(s) URL to fetch"},
            },
        },
    })
}

fn call_tool(cfg: &Config, client: &fetch::Client, params: &Value) -> Value {
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);
    match name {
        "search" => {
            let q = args
                .get("query")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            if q.is_empty() {
                return tool_error("query is required");
            }
            let cat = args
                .get("categories")
                .and_then(|x| x.as_str())
                .unwrap_or("general");
            let max = args
                .get("max_results")
                .and_then(|x| x.as_u64())
                .unwrap_or(5)
                .clamp(1, 50) as usize;
            let page = args
                .get("pageno")
                .and_then(|x| x.as_u64())
                .unwrap_or(1)
                .clamp(1, 3);
            let eng = args.get("engines").and_then(|x| x.as_str()).unwrap_or("");
            let url = format!(
                "/search?q={}&format=json&categories={}&pageno={}&engines={}",
                encode(q),
                encode(cat),
                page,
                encode(eng)
            );
            let (status, body) = server::search_pub(cfg, client, &url);
            if status != 200 {
                return tool_error(&format!("search failed (http {status}): {}", short(&body)));
            }
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            if let Some(e) = v.get("error").and_then(|x| x.as_str()) {
                return tool_error(e);
            }
            let empty = vec![];
            let rs = v
                .get("results")
                .and_then(|x| x.as_array())
                .unwrap_or(&empty);
            if rs.is_empty() {
                let unresp: Vec<String> = v
                    .get("unresponsive_engines")
                    .and_then(|x| x.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|e| e.get(0)?.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let mut t = "No results found.".to_string();
                if !unresp.is_empty() {
                    t.push_str(&format!(" (unresponsive: {})", unresp.join(", ")));
                }
                return tool_text(&t);
            }
            let text = rs
                .iter()
                .take(max)
                .enumerate()
                .map(|(i, r)| {
                    format!(
                        "{}. {}\n   URL: {}\n   {}",
                        i + 1,
                        r.get("title").and_then(|x| x.as_str()).unwrap_or(""),
                        r.get("url").and_then(|x| x.as_str()).unwrap_or(""),
                        r.get("content").and_then(|x| x.as_str()).unwrap_or(""),
                    )
                })
                .collect::<Vec<_>>()
                .join("\n\n");
            tool_text(&text)
        }
        "fetch" => {
            let u = args
                .get("url")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .trim();
            if u.is_empty() {
                return tool_error("url is required");
            }
            let (status, body) = server::fetch_pub(client, &format!("/fetch?url={}", encode(u)));
            let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
            if status != 200 {
                let e = v
                    .get("error")
                    .and_then(|x| x.as_str())
                    .unwrap_or("fetch failed");
                return tool_error(e);
            }
            let title = v.get("title").and_then(|x| x.as_str()).unwrap_or("");
            let content = v.get("content").and_then(|x| x.as_str()).unwrap_or("");
            let links = v
                .get("links")
                .and_then(|x| x.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|l| l.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut t = format!(
                "Title: {title}\n\nContent:\n{content}\n\nLinks found: {}",
                links.len()
            );
            for l in links.iter().take(10) {
                t.push_str(&format!("\n  - {l}"));
            }
            tool_text(&t)
        }
        // Unreachable via dispatch (unknown names rejected above);
        // defensive isError so a direct caller never gets a nested envelope.
        _ => tool_error(&format!("unknown tool {name}")),
    }
}

fn tool_text(t: &str) -> Value {
    json!({"content": [{"type": "text", "text": t}]})
}

fn tool_error(e: &str) -> Value {
    json!({"content": [{"type": "text", "text": e}], "isError": true})
}

fn short(s: &str) -> String {
    s.chars().take(160).collect()
}

fn respond(out: &mut impl Write, v: Value) {
    let _ = writeln!(out, "{}", serde_json::to_string(&v).unwrap_or_default());
    let _ = out.flush();
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

#[cfg(test)]
mod tests {
    use super::{dispatch, PROTOCOL_VERSION};
    use serde_json::json;

    // Framing-only: no Config/client needed (tools/list, ping, errors).
    fn req(method: &str, id: i32) -> serde_json::Value {
        json!({"jsonrpc": "2.0", "id": id, "method": method})
    }

    #[test]
    fn mcp_framing_shapes() {
        let cfg = crate::config::Config::default();
        let client = crate::fetch::Client::new(&cfg);
        let r = dispatch(&cfg, &client, &req("initialize", 1)).unwrap();
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(r["result"]["serverInfo"]["name"], "native-searxng");
        let r = dispatch(&cfg, &client, &req("tools/list", 2)).unwrap();
        let names: Vec<&str> = r["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, vec!["search", "fetch"]);
        assert!(dispatch(&cfg, &client, &req("ping", 3)).unwrap()["result"].is_object());
        let r = dispatch(&cfg, &client, &req("nope", 4)).unwrap();
        assert_eq!(r["error"]["code"], -32601);
        // Notification (no id) -> silence.
        let n = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        assert!(dispatch(&cfg, &client, &n).is_none());
    }
}
