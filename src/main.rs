//! Thin binary wrapper — all logic lives in the library (see lib.rs)
//! so integration tests can exercise engines offline.

use native_searxng::{config, mcp, server};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("native-searxng [--mcp]");
        eprintln!("  default: serve HTTP (SEARXNG_CONF, XDG, or ./searxng.conf)");
        eprintln!("  --mcp:   MCP server on stdio (protocol 2025-06-18: search, fetch)");
        return;
    }
    let cfg = config::load_default();
    if args.iter().any(|a| a == "--mcp") {
        mcp::run_stdio(&cfg);
        return;
    }
    eprintln!("native-searxng on {}:{}", cfg.bind, cfg.port);
    server::run(&cfg);
}
