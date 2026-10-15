//! ArtifactHUB (Helm/K8s ecosystem) search — keyless JSON (DEV-SEARCH #3).
//! `GET /api/v1/packages/search?ts_query_web=..&limit=10` →
//! `{packages:[{normalized_name, name, description, version,
//! repository:{kind, name}}]}`. Page URL assembled from the numeric
//! `repository.kind` slug map; unknown kinds fall back to an AH search
//! URL (always resolves). Template `packages.html`.

use crate::engine::{Engine, SearchResult};
use serde_json::Value;

pub struct Artifacthub;

impl Engine for Artifacthub {
    fn name(&self) -> &'static str {
        "artifacthub"
    }

    fn category(&self) -> &'static str {
        "packages"
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://artifacthub.io/api/v1/packages/search?ts_query_web={}&limit=10",
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
            .get("packages")
            .and_then(|x| x.as_array())
            .unwrap_or(&empty);
        rs.iter()
            .filter_map(|g| {
                let name = g.get("normalized_name")?.as_str()?.trim();
                if name.is_empty() {
                    return None;
                }
                let repo = g.get("repository");
                let kind = repo.and_then(|r| r.get("kind")).and_then(|k| k.as_u64());
                let repo_name = repo
                    .and_then(|r| r.get("name"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                let url = match (kind, repo_name) {
                    (Some(k), r) if !r.is_empty() => match kind_slug(k) {
                        Some(slug) => {
                            format!("https://artifacthub.io/packages/{slug}/{r}/{name}")
                        }
                        None => fallback(name),
                    },
                    _ => fallback(name),
                };
                let desc = g.get("description").and_then(|x| x.as_str()).unwrap_or("");
                let ver = g.get("version").and_then(|x| x.as_str()).unwrap_or("");
                let title = if ver.is_empty() {
                    name.to_string()
                } else {
                    format!("{name} {ver}")
                };
                let mut res = SearchResult::basic(
                    "artifacthub",
                    url,
                    title,
                    desc.split_whitespace().collect::<Vec<_>>().join(" "),
                );
                res.template = "packages.html".into();
                Some(res)
            })
            .take(10)
            .collect()
    }
}

fn fallback(name: &str) -> String {
    format!(
        "https://artifacthub.io/packages/search?ts_query_web={}",
        encode(name)
    )
}

/// Numeric `repository.kind` → URL slug (ArtifactHUB kind registry).
/// Only well-known values are mapped; unknown kinds use the search-URL
/// fallback so results never 404.
fn kind_slug(kind: u64) -> Option<&'static str> {
    match kind {
        0 => Some("helm"),
        1 => Some("falco"),
        2 => Some("opa"),
        3 => Some("olm"),
        5 => Some("krew"),
        6 => Some("helm-plugin"),
        7 => Some("tekton-task"),
        8 => Some("keda-scaler"),
        9 => Some("coredns"),
        10 => Some("keptn"),
        11 => Some("tekton-pipeline"),
        12 => Some("container"),
        13 => Some("kubewarden"),
        14 => Some("gatekeeper"),
        15 => Some("kyverno"),
        16 => Some("knative"),
        17 => Some("backstage"),
        18 => Some("argo-template"),
        _ => None,
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
