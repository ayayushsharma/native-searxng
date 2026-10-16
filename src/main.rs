//! Thin binary wrapper — all logic lives in the library (see lib.rs)
//! so integration tests can exercise engines offline.

use native_searxng::{config, log, mcp, server};

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
    // Logging first: level comes from `[logging]`, everything below
    // (including the refuse-to-start) is a leveled line on stderr.
    let level = config::log_level(&cfg);
    log::set(level);
    // Custom CA: fail the start on a bad file (no silent fallback),
    // warn when active (verification roots are replaced, not added).
    if let Err(e) = native_searxng::fetch::load_ca(&cfg.ca_file) {
        log::emit(log::Level::Error, format!("refusing to start: {e}"));
        std::process::exit(1);
    }
    if !cfg.ca_file.trim().is_empty() {
        native_searxng::info!(
            "custom CA active ({}): system/bundled roots replaced",
            cfg.ca_file
        );
    }
    // stdio MCP: OS trust boundary (local pipe to the agent host), so no
    // bearer — but the binary must still prove it CAN run (bad CA file
    // refuses above). Branches before the HTTP auth gate by design.
    if args.iter().any(|a| a == "--mcp") {
        mcp::run_stdio(&cfg);
        return;
    }
    // Fail closed: an unset token is a refused start, not an open server
    // (a cloudflared tunnel makes even loopback publicly reachable).
    // Local dev escape hatch is explicit: `auth=off` in the conf.
    let tokens = config::api_tokens();
    if tokens.is_empty() && cfg.require_auth {
        log::emit(
            log::Level::Error,
            "refusing to start: SEARXNG_API_TOKEN is unset and conf auth != off.".into(),
        );
        log::emit(
            log::Level::Error,
            "Set SEARXNG_API_TOKEN (comma-separated OK for rotation),".into(),
        );
        log::emit(
            log::Level::Error,
            "or add `auth=off` to the conf for local dev only — never on a tunnelled host.".into(),
        );
        std::process::exit(1);
    }
    native_searxng::info!(
        "native-searxng on {}:{} (auth {}) with {} engines (log {})",
        cfg.bind,
        cfg.port,
        if tokens.is_empty() { "OFF (dev)" } else { "on" },
        cfg.engines.len(),
        cfg.log_level
    );
    server::run(&cfg, &tokens);
}
