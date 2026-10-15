//! Flask docs via the Sphinx-index class (DEV-SEARCH pattern #2, site #1).
//! Index (187KB) fits the class cap; RTD dirhtml URLs (`/` suffix).
//! Scores client-side in `crate::sphinx`; `parse` is inert (the server
//! calls `parse_scored` with the query).

use crate::engine::Engine;
use crate::{engine::SearchResult, sphinx};

pub struct Flask;

const SITE: sphinx::Site = sphinx::Site {
    index_url: "https://flask.palletsprojects.com/en/stable/searchindex.js",
    base: "https://flask.palletsprojects.com/en/stable/",
    suffix: "/",
};

impl Engine for Flask {
    fn name(&self) -> &'static str {
        "flask"
    }

    fn category(&self) -> &'static str {
        "it"
    }

    fn body_cap_kb(&self) -> Option<usize> {
        Some(512)
    }

    fn build_url(&self, _q: &str, _pageno: u32) -> String {
        // Index is query-independent: same fetch every time, scoring is local.
        SITE.index_url.to_string()
    }

    fn parse(&self, _body: &str) -> Vec<SearchResult> {
        vec![]
    }

    fn parse_scored(&self, body: &str, query: &str) -> Vec<SearchResult> {
        sphinx::search(body, &SITE, query, "flask")
    }
}
