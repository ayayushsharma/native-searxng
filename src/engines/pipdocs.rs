//! pip docs via the Sphinx-index class (DEV-SEARCH pattern #2, site #2).
//! Index (226KB) fits the class cap; RTD dirhtml URLs (`/` suffix).
//! Same shape as Flask — the class earns its keep on the second row.

use crate::engine::Engine;
use crate::{engine::SearchResult, sphinx};

pub struct Pip;

const SITE: sphinx::Site = sphinx::Site {
    index_url: "https://pip.pypa.io/en/stable/searchindex.js",
    base: "https://pip.pypa.io/en/stable/",
    suffix: "/",
};

impl Engine for Pip {
    fn name(&self) -> &'static str {
        "pipdocs"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn body_cap_kb(&self) -> Option<usize> {
        Some(512)
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        SITE.index_url.to_string()
    }

    fn parse(&self, _body: &str) -> Vec<SearchResult> {
        vec![]
    }

    fn parse_scored(&self, body: &str, query: &str) -> Vec<SearchResult> {
        sphinx::search(body, &SITE, query, "pipdocs")
    }
}
