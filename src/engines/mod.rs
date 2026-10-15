//! Table of compiled-in engines. Adding one = one small file + one line here.

pub mod arxiv;
pub mod bingnews;
pub mod boardreader;
pub mod bpb;
pub mod chatnoir;
pub mod ddgnews;
pub mod ddg;
pub mod dockerhub;
pub mod dogpile;
pub mod europepmc;
pub mod gentoo;
pub mod github;
pub mod gmx;
pub mod keenable;
pub mod grokipedia;
pub mod hackernews;
pub mod crates;
pub mod crossref;
pub mod mdn;
pub mod npm;
pub mod mwmbl;
pub mod neosearch;
pub mod openalex;
pub mod openlibrary;
pub mod pubmed;
pub mod rubygems;
pub mod stackexchange;
pub mod startpagina;
pub mod swisscows;
pub mod wikinews;
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
        "neosearch" => Some(Box::new(neosearch::Neosearch)),
        "grokipedia" => Some(Box::new(grokipedia::Grokipedia)),
        "startpagina" => Some(Box::new(startpagina::Startpagina)),
        "swisscows" => Some(Box::new(swisscows::Swisscows)),
        "arxiv" => Some(Box::new(arxiv::Arxiv)),
        "europepmc" => Some(Box::new(europepmc::EuropePmc)),
        "bingnews" => Some(Box::new(bingnews::BingNews)),
        "boardreader" => Some(Box::new(boardreader::Boardreader)),
        "bpb" => Some(Box::new(bpb::Bpb)),
        "chatnoir" => Some(Box::new(chatnoir::ChatNoir)),
        "dogpile" => Some(Box::new(dogpile::Dogpile)),
        "wikinews" => Some(Box::new(wikinews::Wikinews)),
        "ddgnews" => Some(Box::new(ddgnews::DdgNews)),
        "pubmed" => Some(Box::new(pubmed::Pubmed)),
        "npm" => Some(Box::new(npm::Npm)),
        "openalex" => Some(Box::new(openalex::OpenAlex)),
        "rubygems" => Some(Box::new(rubygems::Rubygems)),
        "crates" => Some(Box::new(crates::Crates)),
        "crossref" => Some(Box::new(crossref::Crossref)),
        "stackoverflow" => Some(Box::new(stackexchange::StackExchange::new("stackoverflow"))),
        "superuser" => Some(Box::new(stackexchange::StackExchange::new("superuser"))),
        "askubuntu" => Some(Box::new(stackexchange::StackExchange::new("askubuntu"))),
        "github" => Some(Box::new(github::Github)),
        "hackernews" => Some(Box::new(hackernews::HackerNews)),
        "mdn" => Some(Box::new(mdn::Mdn)),
        "dockerhub" => Some(Box::new(dockerhub::DockerHub)),
        "gentoo" => Some(Box::new(gentoo::Gentoo)),
        "gmx" => Some(Box::new(gmx::Gmx)),
        "keenable" => Some(Box::new(keenable::Keenable)),
        _ => None,
    }
}
