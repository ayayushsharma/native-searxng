//! native-searxng library root (binary is a thin wrapper).
//! Exposes the engine table + result types so `tests/` can run fully
//! offline fixture assertions (SearXNG-style: mocked responses, zero
//! network — safe for CI, which must never hammer upstream hosts).

pub mod cache;
pub mod config;
pub mod crypto;
pub mod engine;
pub mod engines;
pub mod fetch;
pub mod jina;
pub mod mcp;
pub mod merge;
pub mod query;
pub mod render;
pub mod scrape;
pub mod server;
pub mod sphinx;
