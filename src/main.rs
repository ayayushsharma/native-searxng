//! Thin binary wrapper — all logic lives in the library (see lib.rs)
//! so integration tests can exercise engines offline.

use native_searxng::{config, mcp, server};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("native-searxng [--mcp]");
        eprintln!("  default: serve HTTP (SEARXNG_CONF, XDG, or ./searxng.toml)");
        eprintln!("  --mcp:   MCP server on stdio (protocol 2025-06-18: search, fetch)");
        eprintln!("  auth:     SEARXNG_API_TOKEN (comma-separated OK); no token +");
        eprintln!("            conf auth != off -> refuse to start (fail closed).");
        return;
    }
    let cfg = config::load_default();
    // Fail closed: an unset token is a refused start, not an open server
    // (a cloudflared tunnel makes even loopback publicly reachable).
    // Local dev escape hatch is explicit: `auth=off` in the conf.
    let tokens = config::api_tokens();
    if tokens.is_empty() && cfg.require_auth {
        eprintln!("refusing to start: SEARXNG_API_TOKEN is unset and conf auth != off.");
        eprintln!("Set SEARXNG_API_TOKEN (comma-separated OK for rotation),");
        eprintln!("or add `auth=off` to the conf for local dev only — never on a tunnelled host.");
        std::process::exit(1);
    }
    if args.iter().any(|a| a == "--mcp") {
        mcp::run_stdio(&cfg);
        return;
    }
    eprintln!("native-searxng on {}:{} (auth {})", cfg.bind, cfg.port, if tokens.is_empty() { "OFF (dev)" } else { "on" });
    server::run(&cfg, &tokens);
}
