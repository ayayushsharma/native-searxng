//! Table of compiled-in engines. Adding one = one small file + one line here.

pub mod ddg;
pub mod dockerhub;
pub mod gentoo;
pub mod github;
pub mod grokipedia;
pub mod mdn;
pub mod mwmbl;
pub mod openlibrary;
pub mod stackexchange;
pub mod startpagina;
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
        "grokipedia" => Some(Box::new(grokipedia::Grokipedia)),
        "startpagina" => Some(Box::new(startpagina::Startpagina)),
        "stackoverflow" => Some(Box::new(stackexchange::StackExchange::new("stackoverflow"))),
        "superuser" => Some(Box::new(stackexchange::StackExchange::new("superuser"))),
        "askubuntu" => Some(Box::new(stackexchange::StackExchange::new("askubuntu"))),
        "github" => Some(Box::new(github::Github)),
        "mdn" => Some(Box::new(mdn::Mdn)),
        "dockerhub" => Some(Box::new(dockerhub::DockerHub)),
        "gentoo" => Some(Box::new(gentoo::Gentoo)),
        _ => None,
    }
}
