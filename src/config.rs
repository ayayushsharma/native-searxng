//! TOML config, parsed once at startup. No reload in MVP.
//!
//! No TOML crate on purpose (RAM budget, PLAN.md §3): this is a
//! hand-rolled parser for exactly the subset `searxng.toml` uses —
//! `[section]` headers, `key = value` with `"strings"`, integers,
//! `true/false`, and single/multi-line string arrays. Unknown
//! sections and keys are ignored (forward-compat); malformed values
//! are hard errors, never silent defaults.

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub bind: String,
    pub engines: Vec<String>,
    pub timeout_ms: u64,
    /// Strangler proxy deadline. Full oracle searches fan out to many
    /// engines — they need headroom the 3 s per-engine cap can't give.
    pub proxy_timeout_ms: u64,
    pub body_cap_kb: usize,
    pub lang: String,
    pub user_agent: String,
    /// Kill-switch for local dev only. Default true (fail closed).
    /// `auth=off` in the conf runs open WITHOUT a token — greppable,
    /// never the default, never for a tunnelled host.
    pub require_auth: bool,
    /// Strangler fallback base URL (Python reference). Empty = disabled.
    pub proxy_to: String,
    /// Console verbosity: off|error|warn|info|debug (default info).
    /// Validated at parse — unknown names refuse to start.
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 8888,
            bind: "127.0.0.1".into(),
            engines: vec!["wikipedia".into(), "ddg".into()],
            timeout_ms: 3000,
            proxy_timeout_ms: 30000,
            body_cap_kb: 96,
            lang: "en".into(),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".into(),
            require_auth: true,
            proxy_to: String::new(),
            log_level: "info".into(),
        }
    }
}

impl Config {
    pub fn body_cap_bytes(&self) -> usize {
        self.body_cap_kb * 1024
    }
}

/// API keys for keyed engines. Convention: `SEARXNG_API_KEY_<ENGINE>`
/// (uppercased), env-only — never in the conf file, never in the repo.
/// Env wins because supervisors (runit, Termux:Boot, systemd) inject env
/// cleanly, files leak via backups and git, and rotation needs no edits.
/// Same-UID `/proc` visibility is the accepted residual (single-user box).
/// Returns None when unset/empty = engine stays dormant (see availability).
pub fn api_key(engine: &str) -> Option<String> {
    let var = format!("SEARXNG_API_KEY_{}", engine.to_uppercase());
    std::env::var(&var).ok().filter(|v| !v.trim().is_empty())
}

/// Bearer token(s) for the HTTP gate. Convention: `SEARXNG_API_TOKEN`,
/// env-only — same rationale as `api_key` (supervisors inject env,
/// files leak via backups/git). Comma-separated = all accepted, so
/// rotation needs no downtime: deploy `new`, roll devices, drop `old`.
/// Empty/unset = fail closed (main refuses to start unless the conf
/// explicitly says `auth=off` for local dev).
pub fn api_tokens() -> Vec<String> {
    std::env::var("SEARXNG_API_TOKEN")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Config path resolution (XDG-first, no new deps):
/// 1. `$SEARXNG_CONF` when set (explicit wins — supervisors/daemons).
/// 2. `$XDG_CONFIG_HOME/native-searxng/searxng.toml`.
/// 3. `$HOME/.config/native-searxng/searxng.toml`.
/// 4. `./searxng.toml` (dev back-compat; daemon CWDs are unreliable).
/// Missing files fall through to `Config::default()` via `load_default`.
/// NOTE: the legacy flat `searxng.conf` is NOT read — the parser rejects
/// section-less keys loudly instead of booting on silent defaults.
pub fn default_path() -> String {
    if let Ok(p) = std::env::var("SEARXNG_CONF") {
        if !p.is_empty() {
            return p;
        }
    }
    let rel = "native-searxng/searxng.toml";
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return format!("{xdg}/{rel}");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return format!("{home}/.config/{rel}");
        }
    }
    "searxng.toml".into()
}

/// Load from [`default_path`], falling back to `./searxng.toml`, then defaults.
pub fn load_default() -> Config {
    let p = default_path();
    if let Ok(cfg) = load(&p) {
        return cfg;
    }
    if p != "searxng.toml" {
        if let Ok(cfg) = load("searxng.toml") {
            return cfg;
        }
    }
    Config::default()
}

pub fn load(path: &str) -> Result<Config, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut cfg = Config::default();
    // Multi-line arrays: accumulate until brackets balance.
    let mut pending: Option<String> = None;
    let mut section = String::new();
    let mut saw_section = false;
    let mut saw_engines = false;
    for raw in text.lines() {
        let mut line = raw.trim().to_string();
        // A section header never continues an array: unterminated
        // array + new header = loud error, not silent merge.
        if line.starts_with('[') {
            if pending.take().is_some() {
                return Err("bad array".into());
            }
            if !balanced(&line) {
                return Err("bad array".into());
            }
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            saw_section = true;
            continue;
        }
        if pending.is_some() {
            // Full-line comments/blank lines never join an array body.
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            line = format!("{} {line}", pending.take().unwrap());
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            if pending.is_some() || !balanced(&line) {
                return Err("bad array".into());
            }
            section = line
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            saw_section = true;
            continue;
        }
        if !balanced(&line) {
            pending = Some(line);
            continue;
        }
        let line = strip_comment(&line);
        if line.is_empty() {
            continue;
        }
        if !saw_section {
            return Err("legacy flat config: use [sections] (see searxng.toml)".into());
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        // First [engines] group replaces the default pair; later groups
        // append in file order (conf order stays merge weight).
        if section == "engines" && !saw_engines {
            cfg.engines.clear();
            saw_engines = true;
        }
        apply(&mut cfg, &section, k.trim(), v.trim())?;
    }
    if pending.is_some() {
        return Err("bad array".into());
    }
    Ok(cfg)
}

/// `[`/`]` balance outside `"..."` quotes. Drives multi-line arrays.
fn balanced(s: &str) -> bool {
    let mut depth = 0i32;
    let mut quote = false;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            esc = false;
            continue;
        }
        match c {
            '\\' if quote => esc = true,
            '"' => quote = !quote,
            '[' if !quote => depth += 1,
            ']' if !quote => depth -= 1,
            _ => {}
        }
    }
    depth == 0
}

/// Cut a `#` comment, but only outside `"..."` (URLs carry `#`).
fn strip_comment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut quote = false;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            out.push(c);
            esc = false;
            continue;
        }
        match c {
            '\\' if quote => {
                out.push(c);
                esc = true;
            }
            '"' => {
                out.push(c);
                quote = !quote;
            }
            '#' if !quote => break,
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

/// `"quoted"` → unescaped string (`\"`, `\\`, `\n`, `\t`).
fn string(v: &str) -> Result<String, String> {
    let inner = v
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .ok_or_else(|| format!("bad string: {v}"))?;
    let mut out = String::with_capacity(inner.len());
    let mut esc = false;
    for c in inner.chars() {
        if esc {
            match c {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                c => out.push(c),
            }
            esc = false;
            continue;
        }
        if c == '\\' {
            esc = true;
            continue;
        }
        out.push(c);
    }
    Ok(out)
}

fn int(v: &str, what: &str) -> Result<u64, String> {
    v.parse().map_err(|_| format!("bad {what}"))
}

/// `key = value` under `[section]`. Unknown sections/keys are ignored
/// (forward-compat); bad values are hard errors. `[engines]` appends
/// every category array in FILE order — conf order stays merge weight.
/// Validated at parse time, so this cannot fail (the Info fallback
/// is unreachable armor, never a silent default for bad input).
pub fn log_level(cfg: &Config) -> crate::log::Level {
    crate::log::Level::parse(&cfg.log_level).unwrap_or(crate::log::Level::Info)
}

fn apply(cfg: &mut Config, section: &str, k: &str, v: &str) -> Result<(), String> {
    match section {
        "server" => match k {
            "port" => {
                let n = int(v, "port")?;
                if n == 0 || n > 65535 {
                    return Err("bad port".into());
                }
                cfg.port = n as u16;
            }
            "bind" => cfg.bind = string(v)?,
            "require_auth" => match v {
                "true" => cfg.require_auth = true,
                "false" => cfg.require_auth = false,
                _ => return Err("bad require_auth".into()),
            },
            _ => {}
        },
        "http" => match k {
            "timeout_ms" => cfg.timeout_ms = int(v, "timeout_ms")?,
            "proxy_timeout_ms" => cfg.proxy_timeout_ms = int(v, "proxy_timeout_ms")?,
            "body_cap_kb" => cfg.body_cap_kb = int(v, "body_cap_kb")? as usize,
            "language" => cfg.lang = string(v)?,
            "user_agent" => cfg.user_agent = string(v)?,
            _ => {}
        },
        "proxy" => match k {
            "url" => cfg.proxy_to = string(v)?,
            _ => {}
        },
        "logging" => match k {
            "level" => {
                let s = string(v)?;
                if crate::log::Level::parse(&s).is_none() {
                    return Err(format!("bad log level {:?} (off|error|warn|info|debug)", s));
                }
                cfg.log_level = s;
            }
            _ => {}
        },
        "engines" => {
            let body = v
                .strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .ok_or_else(|| format!("bad engines[{k}]"))?;
            for item in split_items(body) {
                let item = item.trim();
                if item.is_empty() {
                    continue; // trailing comma
                }
                cfg.engines.push(string(item)?);
            }
        }
        _ => {}
    }
    Ok(())
}

/// Split array items on commas outside `"..."` quotes.
fn split_items(body: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut quote = false;
    let mut esc = false;
    for (i, c) in body.char_indices() {
        if esc {
            esc = false;
            continue;
        }
        match c {
            '\\' if quote => esc = true,
            '"' => quote = !quote,
            ',' if !quote => {
                items.push(&body[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    items.push(&body[start..]);
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_str(s: &str) -> Result<Config, String> {
        let p = std::env::temp_dir().join(format!("cfgtest-{}.toml", s.len()));
        std::fs::write(&p, s).map_err(|e| e.to_string())?;
        let r = load(p.to_str().unwrap());
        let _ = std::fs::remove_file(&p);
        r
    }

    #[test]
    fn full_doc_parses() {
        let cfg = load_str(
            r#"
[server]
port = 8890
bind = "0.0.0.0" # comment
require_auth = false

[http]
timeout_ms = 3000
proxy_timeout_ms = 30000
body_cap_kb = 96
language = "de"
user_agent = "UA # not-a-comment"

[proxy]
url = "http://127.0.0.1:8889"

[logging]
level = "debug"

[engines]
general = ["wikipedia", "ddg",]
it = [
  # comment inside array
  "github",
  "flask",
]
[unknown_section]
future_key = 1
"#,
        )
        .expect("full doc");
        assert_eq!(cfg.port, 8890);
        assert_eq!(cfg.bind, "0.0.0.0");
        assert!(!cfg.require_auth);
        assert_eq!(cfg.lang, "de");
        assert_eq!(cfg.user_agent, "UA # not-a-comment");
        assert_eq!(cfg.proxy_to, "http://127.0.0.1:8889");
        assert_eq!(cfg.log_level, "debug");
        assert_eq!(cfg.engines, vec!["wikipedia", "ddg", "github", "flask"]);
    }

    #[test]
    fn legacy_flat_rejected() {
        let e = load_str("port=8888\nengines=wikipedia,ddg\n").unwrap_err();
        assert!(e.contains("legacy"), "{e}");
    }

    #[test]
    fn bad_values_error() {
        for doc in [
            "[server]\nport = 0\n",
            "[server]\nport = 70000\n",
            "[server]\nport = \"x\"\n",
            "[server]\nrequire_auth = off\n",
            "[logging]\nlevel = verbose\n",
            "[logging]\nlevel = 3\n",
            "[http]\ntimeout_ms = soon\n",
            "[engines]\ngeneral = [wikipedia]\n",
            "[engines]\ngeneral = [\"a\",\n",
        ] {
            assert!(load_str(doc).is_err(), "{doc}");
        }
    }

    #[test]
    fn missing_file_errors() {
        assert!(load("/nonexistent/searxng.toml").is_err());
    }

    #[test]
    fn shipped_toml_engine_list() {
        // CWD for cargo test is the crate root: this guards the real file
        // (53 engines, no dupes, fail-closed). Order = merge weight;
        // regroup order is canonical since the 2026-10-16 TOML migration.
        let cfg = load("searxng.toml").expect("shipped searxng.toml");
        let want = [
            "wikipedia", "ddg", "openlibrary", "mwmbl", "grokipedia", "startpagina",
            "swisscows", "boardreader", "bpb", "chatnoir", "dogpile", "gmx",
            "keenable", "neosearch", "stackoverflow", "superuser", "askubuntu",
            "github", "hackernews", "mdn", "microsoftlearn", "flask", "pipdocs",
            "cheatsh", "dockerhub", "gentoo", "archlinux", "nvd", "datatracker",
            "arxiv", "europepmc", "openalex", "crossref", "pubmed", "bingnews",
            "wikinews", "tagesschau", "ddgnews", "npm", "crates", "rubygems",
            "packagist", "hexpm", "pubdev", "maven", "nuget", "artifacthub",
            "terraform", "pkggodev", "sepiasearch", "jisho", "open_meteo",
        ];
        assert_eq!(cfg.engines, want);
        assert_eq!(cfg.port, 8888);
        assert_eq!(cfg.proxy_to, "http://127.0.0.1:8889");
        assert!(cfg.require_auth, "shipped file stays fail-closed");
        assert_eq!(cfg.log_level, "info");
    }
}
