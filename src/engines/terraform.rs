//! Terraform Registry module search — keyless JSON (DEV-SEARCH queue #3).
//! `GET /v1/modules/search?q=..&limit=10` → `{modules:[{namespace, name,
//! provider, version, description, published_at, ...}], meta}`.
//! URL assembled as `https://registry.terraform.io/modules/{ns}/{name}/
//! {provider}/{version}`; `published_at` date-stamped (bpb pattern).
//! Template `packages.html`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Terraform;

impl Engine for Terraform {
    fn name(&self) -> &'static str {
        "terraform"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://registry.terraform.io/v1/modules/search?q={}&limit=10",
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
            .get("modules")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let ns = g.get("namespace")?.as_str()?.trim();
                let name = g.get("name")?.as_str()?.trim();
                let prov = g.get("provider")?.as_str()?.trim();
                if ns.is_empty() || name.is_empty() || prov.is_empty() {
                    return None;
                }
                let ver = g.get("version").and_then(|x| x.as_str()).unwrap_or("");
                let url = if ver.is_empty() {
                    format!("https://registry.terraform.io/modules/{ns}/{name}/{prov}")
                } else {
                    format!("https://registry.terraform.io/modules/{ns}/{name}/{prov}/{ver}")
                };
                let desc = g.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let mut res = SearchResult::basic(
                    "terraform",
                    url,
                    format!("{ns}/{name}/{prov}"),
                    desc.split_whitespace().collect::<Vec<_>>().join(" "),
                );
                if let Some(pub_at) = g.get("published_at").and_then(|x| x.as_str()) {
                    if pub_at.len() >= 10 {
                        res.published_date = Some(pub_at[..10].to_string());
                    }
                }
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
