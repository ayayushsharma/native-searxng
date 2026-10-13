mod cache;
mod config;
mod engine;
mod engines;
mod fetch;
mod merge;
mod query;
mod render;
mod server;

fn main() {
    let cfg = config::load("searxng.conf").unwrap_or_default();
    eprintln!("native-searxng on {}:{}", cfg.bind, cfg.port);
    server::run(&cfg);
}
