//! Thin binary wrapper — all logic lives in the library (see lib.rs)
//! so integration tests can exercise engines offline.

use native_searxng::{config, server};

fn main() {
    let cfg = config::load_default();
    eprintln!("native-searxng on {}:{}", cfg.bind, cfg.port);
    server::run(&cfg);
}
