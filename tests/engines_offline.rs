//! Offline test scope: every engine parser runs against a RECORDED
//! fixture (or an inline minimal body). Zero network — CI-safe by design,
//! mirroring how upstream SearXNG tests engines (`tests/unit`: mocked
//! responses, never live hosts). Live behavior stays gated locally by
//! `agentic/evals/check.py` + `agentic/diff/diff.py` (never in CI).
//!
//! Covered: all 21 native engines (parse), malformed-input robustness
//! (never panics, never garbage), challenge detection, two-step
//! follow-up, query parsing, JSON contract keys, merge dedup.

use native_searxng::{
    engine::{Engine, SearchResult},
    engines, merge, query, render,
};

fn engine(name: &str) -> Box<dyn Engine> {
    engines::find(name).unwrap_or_else(|| panic!("unknown engine {name}"))
}

/// (engine name, fixture file under repo root or inline body, min results)
const INLINE_WIKIPEDIA: &str = r#"["t",["Title T"],["Desc D"],["https://en.wikipedia.org/wiki/T"]]"#;
const INLINE_DDG: &str = r#"<div class="result"><a class="result__a" href="https://example.com/x">Example Title</a><div class="result__snippet">Some snippet here</div></div>"#;

const CASES: &[(&str, Option<&str>, Option<&str>, usize)] = &[
    ("wikipedia", None, Some(INLINE_WIKIPEDIA), 1),
    ("ddg", None, Some(INLINE_DDG), 1),
    ("openlibrary", Some("tests-fixture-openlibrary-test.json"), None, 1),
    ("mwmbl", Some("tests-fixture-mwmbl-test.json"), None, 1),
    ("grokipedia", Some("tests-fixture-grokipedia-test.json"), None, 1),
    ("startpagina", Some("tests-fixture-startpagina-test.json"), None, 1),
    ("stackoverflow", Some("tests-fixture-stackexchange-test.json"), None, 5),
    ("superuser", Some("tests-fixture-stackexchange-test.json"), None, 1),
    ("askubuntu", Some("tests-fixture-stackexchange-test.json"), None, 1),
    ("github", Some("tests-fixture-github-test.json"), None, 5),
    ("mdn", Some("tests-fixture-mdn-test.json"), None, 5),
    ("dockerhub", Some("tests-fixture-dockerhub-test.json"), None, 5),
    ("gentoo", Some("tests-fixture-gentoo-test.json"), None, 1),
    ("arxiv", Some("tests-fixture-arxiv-test.json"), None, 5),
    ("europepmc", Some("tests-fixture-europepmc-test.json"), None, 5),
    ("pubmed", Some("tests-fixture-pubmed-test.json"), None, 5),
    ("bingnews", Some("tests-fixture-bingnews-test.json"), None, 1),
    ("wikinews", Some("tests-fixture-wikinews-test.json"), None, 5),
    ("ddgnews", None, Some(INLINE_DDG), 1),
    ("npm", Some("tests-fixture-npm-test.json"), None, 5),
    ("crates", Some("tests-fixture-crates-test.json"), None, 5),
];

fn body(file: Option<&str>, inline: Option<&str>) -> String {
    match (file, inline) {
        (Some(f), _) => std::fs::read_to_string(f)
            .unwrap_or_else(|_| panic!("missing fixture {f} — record with 1 live probe")),
        (_, Some(s)) => s.to_string(),
        _ => unreachable!(),
    }
}

#[test]
fn every_engine_parses_its_fixture() {
    for (name, file, inline, min) in CASES {
        let e = engine(name);
        let out = e.parse(&body(*file, *inline));
        assert!(
            out.len() >= *min,
            "{name}: only {} results (min {min})",
            out.len()
        );
        for r in &out {
            assert!(!r.url.is_empty(), "{name}: empty url");
            assert!(!r.title.is_empty(), "{name}: empty title");
            assert_eq!(r.engine, *name, "{name}: wrong engine tag");
            assert!(!r.engines.is_empty(), "{name}: empty engines vec");
        }
    }
}

#[test]
fn malformed_input_never_panics_never_garbage() {
    let bad = ["", "{}", "[]", "null", "<html><body>nope", "{{{{", "\u{0}\u{1}\u{2}"];
    for (name, _, _, _) in CASES {
        let e = engine(name);
        for b in bad {
            let out = e.parse(b);
            assert!(out.is_empty(), "{name}: garbage in -> results out");
            for r in &out {
                assert!(!r.url.is_empty() && !r.title.is_empty());
            }
        }
    }
}

#[test]
fn truncated_fixtures_parse_or_skip() {
    for (name, file, inline, _) in CASES {
        // Inline bodies are already minimal; truncate the recorded files.
        if file.is_none() {
            continue;
        }
        let e = engine(name);
        let full = body(*file, *inline);
        let half = &full[..full.len() / 2];
        let out = e.parse(half);
        for r in &out {
            assert!(!r.url.is_empty() && !r.title.is_empty(), "{name}: bad row from truncation");
        }
    }
}

#[test]
fn challenge_pages_report_unresponsive() {
    let anomaly = std::fs::read_to_string("tests-fixture-ddg-anomaly.html").expect("anomaly fixture");
    for name in ["ddg", "ddgnews"] {
        let e = engine(name);
        assert!(e.challenge(&anomaly).is_some(), "{name}: anomaly not detected");
        // Clean bodies must NOT trip the detector.
        assert!(e.challenge(INLINE_DDG).is_none(), "{name}: false positive");
    }
}

#[test]
fn pubmed_two_step_follow_up() {
    let e = engine("pubmed");
    // Leg 1 shape (inline, no network): IDs -> esummary URL.
    let leg1 = r#"{"esearchresult": {"idlist": ["1", "2"], "count": "2"}}"#;
    let url = e.follow_up(leg1).expect("pubmed follow_up");
    assert!(url.contains("esummary.fcgi"), "wrong leg-2 url: {url}");
    assert!(url.contains("id=1,2"), "IDs missing: {url}");
    // Empty ID list -> no follow-up (parse leg 1 directly = no results).
    assert!(e.follow_up(r#"{"esearchresult": {"idlist": []}}"#).is_none());
    assert!(e.parse(leg1).is_empty());
    // Single-step engines never follow up.
    assert!(engine("arxiv").follow_up(leg1).is_none());
}

#[test]
fn query_parsing() {
    let q = query::parse("/search?q=hello+world&format=json&categories=it&pageno=2");
    assert_eq!(q.q, "hello world");
    assert_eq!(q.format, "json");
    assert_eq!(q.categories, "it");
    assert_eq!(q.pageno, 2);
    let empty = query::parse("/search?format=json");
    assert!(empty.q.is_empty());
}

#[test]
fn json_contract_keys_byte_exact() {
    // Top-level + per-result key sets must match agentic/CONTRACT.md.
    let r = SearchResult::basic("wikipedia", "https://example.com".into(), "T".into(), "C".into());
    let out: serde_json::Value =
        serde_json::from_str(&render::json("q", &[r], &[])).expect("render json");
    let top: std::collections::HashSet<&str> =
        out.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    for k in ["query", "results", "answers", "corrections", "infoboxes", "suggestions", "unresponsive_engines"] {
        assert!(top.contains(k), "top-level key missing: {k}");
    }
    let row = &out["results"][0];
    let keys: std::collections::HashSet<&str> =
        row.as_object().unwrap().keys().map(|s| s.as_str()).collect();
    for k in [
        "url", "title", "content", "engine", "engines", "publishedDate", "positions",
        "score", "category", "template", "img_src", "thumbnail", "iframe_src", "audio_src",
    ] {
        assert!(keys.contains(k), "result key missing: {k}");
    }
}

#[test]
fn jina_parse_shapes() {
    use native_searxng::jina;
    // JSON envelope (Accept: application/json).
    let env = r#"{"code":200,"status":20000,"data":{"title":"Example Title","url":"https://example.com","content":"Hello world, this is the extracted page text.","links":["https://example.com/a","https://example.com/b"]}}"#;
    let r = jina::parse(env).expect("envelope");
    assert_eq!(r.title, "Example Title");
    assert!(r.content.contains("extracted page"));
    assert_eq!(r.links.len(), 2);
    // Non-200 code -> None (caller maps to {"error"}).
    assert!(jina::parse(r#"{"code":429,"status":42900,"data":{}}"#).is_none());
    // Empty content -> None.
    assert!(jina::parse(r#"{"code":200,"data":{"title":"T","content":"  "}}"#).is_none());
    // Plain-text fallback (no Accept header).
    let r = jina::parse("# Headline\n\nSome reasonably long body text here.").expect("text");
    assert!(!r.content.is_empty());
    // Garbage -> None, never panics.
    for bad in ["", "{}", "<html>", "xx"] {
        assert!(jina::parse(bad).is_none());
    }
}

#[test]
fn api_key_env_convention() {
    use native_searxng::config;
    let saved = std::env::var("SEARXNG_API_KEY_JINA").ok();
    std::env::remove_var("SEARXNG_API_KEY_JINA");
    assert!(config::api_key("jina").is_none());
    std::env::set_var("SEARXNG_API_KEY_JINA", "jina_testkey");
    assert_eq!(config::api_key("jina").as_deref(), Some("jina_testkey"));
    std::env::set_var("SEARXNG_API_KEY_JINA", "   ");
    assert!(config::api_key("jina").is_none(), "blank key = dormant");
    match saved {
        Some(v) => std::env::set_var("SEARXNG_API_KEY_JINA", v),
        None => std::env::remove_var("SEARXNG_API_KEY_JINA"),
    }
}

#[test]
fn config_xdg_resolution_order() {
    use native_searxng::config;
    // Save/restore: this is the only test touching these vars.
    let saved = ["SEARXNG_CONF", "XDG_CONFIG_HOME", "HOME"]
        .iter()
        .map(|k| (k, std::env::var(k).ok()))
        .collect::<Vec<_>>();
    let restore = || {
        for (k, v) in &saved {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
    };
    std::env::set_var("SEARXNG_CONF", "/tmp/x.conf");
    std::env::set_var("XDG_CONFIG_HOME", "/tmp/xdg");
    std::env::set_var("HOME", "/tmp/home");
    assert_eq!(config::default_path(), "/tmp/x.conf");
    std::env::remove_var("SEARXNG_CONF");
    assert_eq!(config::default_path(), "/tmp/xdg/native-searxng/searxng.conf");
    std::env::remove_var("XDG_CONFIG_HOME");
    assert_eq!(config::default_path(), "/tmp/home/.config/native-searxng/searxng.conf");
    std::env::remove_var("HOME");
    assert_eq!(config::default_path(), "searxng.conf");
    // Missing everywhere -> defaults, never an error.
    let cfg = config::load_default();
    assert_eq!(cfg.port, config::Config::default().port);
    restore();
}

#[test]
fn merge_dedups_and_caps() {
    let a = SearchResult::basic("wikipedia", "https://example.com/x".into(), "A".into(), "".into());
    let b = SearchResult::basic("github", "https://example.com/x".into(), "B".into(), "".into());
    let merged = merge::merge(vec![vec![a], vec![b]], 20);
    assert_eq!(merged.len(), 1, "same URL must dedup");
    let many: Vec<SearchResult> = (0..30)
        .map(|i| SearchResult::basic("mdn", format!("https://example.com/{i}"), "T".into(), "".into()))
        .collect();
    assert_eq!(merge::merge(vec![many], 20).len(), 20, "max_results cap");
}
