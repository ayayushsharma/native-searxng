//! Table of compiled-in engines. Adding one = one small file + one line here.

pub mod archlinux;
pub mod artifacthub;
pub mod arxiv;
pub mod bingnews;
pub mod boardreader;
pub mod bpb;
pub mod chatnoir;
pub mod cheatsh;
pub mod crates;
pub mod crossref;
pub mod datatracker;
pub mod ddg;
pub mod ddgnews;
pub mod dockerhub;
pub mod dogpile;
pub mod europepmc;
pub mod flask;
pub mod gentoo;
pub mod github;
pub mod gmx;
pub mod grokipedia;
pub mod hackernews;
pub mod hexpm;
pub mod jisho;
pub mod keenable;
pub mod maven;
pub mod mdn;
pub mod microsoftlearn;
pub mod mwmbl;
pub mod neosearch;
pub mod npm;
pub mod nuget;
pub mod nvd;
pub mod open_meteo;
pub mod openalex;
pub mod openlibrary;
pub mod packagist;
pub mod pipdocs;
pub mod pkggodev;
pub mod pubdev;
pub mod pubmed;
pub mod rubygems;
pub mod sepiasearch;
pub mod stackexchange;
pub mod startpagina;
pub mod swisscows;
pub mod swisscows_news;
pub mod tagesschau;
pub mod terraform;
pub mod wikinews;
pub mod wikipedia;
pub mod yahoo_news;

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
        "swisscows_news" => Some(Box::new(swisscows_news::SwisscowsNews)),
        "arxiv" => Some(Box::new(arxiv::Arxiv)),
        "europepmc" => Some(Box::new(europepmc::EuropePmc)),
        "bingnews" => Some(Box::new(bingnews::BingNews)),
        "boardreader" => Some(Box::new(boardreader::Boardreader)),
        "bpb" => Some(Box::new(bpb::Bpb)),
        "chatnoir" => Some(Box::new(chatnoir::ChatNoir)),
        "dogpile" => Some(Box::new(dogpile::Dogpile)),
        "wikinews" => Some(Box::new(wikinews::Wikinews)),
        "yahoo_news" => Some(Box::new(yahoo_news::YahooNews)),
        "ddgnews" => Some(Box::new(ddgnews::DdgNews)),
        "pubmed" => Some(Box::new(pubmed::Pubmed)),
        "npm" => Some(Box::new(npm::Npm)),
        "nvd" => Some(Box::new(nvd::Nvd)),
        "nuget" => Some(Box::new(nuget::Nuget)),
        "maven" => Some(Box::new(maven::Maven)),
        "datatracker" => Some(Box::new(datatracker::Datatracker)),
        "flask" => Some(Box::new(flask::Flask)),
        "pipdocs" => Some(Box::new(pipdocs::Pip)),
        "cheatsh" => Some(Box::new(cheatsh::Cheatsh)),
        "tagesschau" => Some(Box::new(tagesschau::Tagesschau)),
        "sepiasearch" => Some(Box::new(sepiasearch::SepiaSearch)),
        "jisho" => Some(Box::new(jisho::Jisho)),
        "open_meteo" => Some(Box::new(open_meteo::OpenMeteo)),
        "pkggodev" => Some(Box::new(pkggodev::PkgGoDev)),
        "pubdev" => Some(Box::new(pubdev::Pubdev)),
        "terraform" => Some(Box::new(terraform::Terraform)),
        "openalex" => Some(Box::new(openalex::OpenAlex)),
        "rubygems" => Some(Box::new(rubygems::Rubygems)),
        "crates" => Some(Box::new(crates::Crates)),
        "packagist" => Some(Box::new(packagist::Packagist)),
        "hexpm" => Some(Box::new(hexpm::Hex)),
        "artifacthub" => Some(Box::new(artifacthub::Artifacthub)),
        "crossref" => Some(Box::new(crossref::Crossref)),
        "stackoverflow" => Some(Box::new(stackexchange::StackExchange::new("stackoverflow"))),
        "superuser" => Some(Box::new(stackexchange::StackExchange::new("superuser"))),
        "askubuntu" => Some(Box::new(stackexchange::StackExchange::new("askubuntu"))),
        "github" => Some(Box::new(github::Github)),
        "hackernews" => Some(Box::new(hackernews::HackerNews)),
        "mdn" => Some(Box::new(mdn::Mdn)),
        "microsoftlearn" => Some(Box::new(microsoftlearn::MicrosoftLearn)),
        "dockerhub" => Some(Box::new(dockerhub::DockerHub)),
        "gentoo" => Some(Box::new(gentoo::Gentoo)),
        "archlinux" => Some(Box::new(archlinux::ArchLinux)),
        "gmx" => Some(Box::new(gmx::Gmx)),
        "keenable" => Some(Box::new(keenable::Keenable)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::find;

    /// Every non-general engine declares its category here. The default
    /// is `general`, so a news/science/it/packages engine WITHOUT a
    /// `category()` override silently serves zero queries in its own
    /// category (the server filters by it) — this map caught exactly
    /// that for swisscows_news/yahoo_news. New non-general engine =
    /// one line here.
    #[test]
    fn non_general_engines_declare_category() {
        let want = [
            ("stackoverflow", "it"),
            ("superuser", "it"),
            ("askubuntu", "it"),
            ("github", "it"),
            ("hackernews", "it"),
            ("mdn", "it"),
            ("microsoftlearn", "it"),
            ("flask", "it"),
            ("pipdocs", "it"),
            ("cheatsh", "it"),
            ("dockerhub", "it"),
            ("gentoo", "it"),
            ("archlinux", "it"),
            ("nvd", "it"),
            ("datatracker", "it"),
            ("arxiv", "science"),
            ("europepmc", "science"),
            ("openalex", "science"),
            ("crossref", "science"),
            ("pubmed", "science"),
            ("bingnews", "news"),
            ("wikinews", "news"),
            ("tagesschau", "news"),
            ("ddgnews", "news"),
            ("swisscows_news", "news"),
            ("yahoo_news", "news"),
            ("npm", "packages"),
            ("crates", "packages"),
            ("rubygems", "packages"),
            ("packagist", "packages"),
            ("hexpm", "packages"),
            ("pubdev", "packages"),
            ("maven", "packages"),
            ("nuget", "packages"),
            ("artifacthub", "packages"),
            ("terraform", "packages"),
            ("pkggodev", "packages"),
            ("sepiasearch", "videos"),
            ("jisho", "dictionaries"),
            ("open_meteo", "weather"),
        ];
        for (name, cat) in want {
            let e = find(name).unwrap_or_else(|| panic!("{name} not registered"));
            assert_eq!(e.category(), cat, "{name} serves {cat}");
        }
    }
}
