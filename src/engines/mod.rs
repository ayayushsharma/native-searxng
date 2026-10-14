//! Table of compiled-in engines. Adding one = one small file + one line here.

pub mod ddg;
pub mod mwmbl;
pub mod openlibrary;
pub mod wikipedia;

use crate::engine::Engine;

/// Engine lookup by configured name. Returns None for unknown names
/// (caller proxies to Python reference or 503s, per config).
pub fn find(name: &str) -> Option<Box<dyn Engine>> {
    match name {
        "wikipedia" => Some(Box::new(wikipedia::Wikipedia)),
        "ddg" => Some(Box::new(ddg::Ddg)),
        "openlibrary" => Some(Box::new(openlibrary::Openlibrary)),
        "mwmbl" => Some(Box::new(mwmbl::Mwmbl)),
        _ => None,
    }
}
