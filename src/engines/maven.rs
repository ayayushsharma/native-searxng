//! Maven Central (Java/Kotlin) search — keyless Solr JSON (DEV-SEARCH #2).
//! `GET search.maven.org/solrsearch/select?q=..&rows=10&wt=json` →
//! `{response:{docs:[{g, a, latestVersion, p, timestamp...}]}}`.
//! URL assembled as `https://search.maven.org/artifact/{g}/{a}/{v}/jar`.
//! Template `packages.html`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Maven;

impl Engine for Maven {
    fn name(&self) -> &'static str {
        "maven"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://search.maven.org/solrsearch/select?q={}&rows=10&wt=json",
            encode(q)
        )
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let empty = vec![];
        let rs = v
            .get("response")
            .and_then(|r| r.get("docs"))
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let gid = g.get("g")?.as_str()?.trim();
                let aid = g.get("a")?.as_str()?.trim();
                if gid.is_empty() || aid.is_empty() {
                    return None;
                }
                let ver = g
                    .get("latestVersion")
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                let pack = g.get("p").and_then(|x| x.as_str()).unwrap_or("jar");
                let mut res = SearchResult::basic(
                    "maven",
                    format!("https://search.maven.org/artifact/{gid}/{aid}/{ver}/{pack}"),
                    if ver.is_empty() {
                        format!("{gid}:{aid}")
                    } else {
                        format!("{gid}:{aid} {ver}")
                    },
                    format!("{gid}:{aid} · {pack} packaging"),
                );
                res.template = "packages.html".into();
                Some(res)
            })
            .take(10)
            .collect()
    }
}

fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
