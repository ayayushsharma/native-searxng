//! Minimal INI config, parsed once at startup. No reload in MVP.

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
/// 2. `$XDG_CONFIG_HOME/native-searxng/searxng.conf`.
/// 3. `$HOME/.config/native-searxng/searxng.conf`.
/// 4. `./searxng.conf` (dev back-compat; daemon CWDs are unreliable).
/// Missing files fall through to `Config::default()` via `load_default`.
pub fn default_path() -> String {
    if let Ok(p) = std::env::var("SEARXNG_CONF") {
        if !p.is_empty() {
            return p;
        }
    }
    let rel = "native-searxng/searxng.conf";
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
    "searxng.conf".into()
}

/// Load from [`default_path`], falling back to `./searxng.conf`, then defaults.
pub fn load_default() -> Config {
    let p = default_path();
    if let Ok(cfg) = load(&p) {
        return cfg;
    }
    if p != "searxng.conf" {
        if let Ok(cfg) = load("searxng.conf") {
            return cfg;
        }
    }
    Config::default()
}

pub fn load(path: &str) -> Result<Config, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut cfg = Config::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k.trim() {
            "port" => cfg.port = v.trim().parse().map_err(|_| "bad port")?,
            "bind" => cfg.bind = v.trim().to_string(),
            "engines" => {
                cfg.engines = v.split(',').map(|s| s.trim().to_string()).collect();
            }
            "timeout_ms" => cfg.timeout_ms = v.trim().parse().map_err(|_| "bad timeout_ms")?,
            "proxy_timeout_ms" => {
                cfg.proxy_timeout_ms = v.trim().parse().map_err(|_| "bad proxy_timeout_ms")?
            }
            "body_cap_kb" => cfg.body_cap_kb = v.trim().parse().map_err(|_| "bad body_cap_kb")?,
            "lang" => cfg.lang = v.trim().to_string(),
            "user_agent" => cfg.user_agent = v.trim().to_string(),
            "proxy_to" => cfg.proxy_to = v.trim().to_string(),
            "auth" => cfg.require_auth = v.trim() != "off",
            _ => {}
        }
    }
    Ok(cfg)
}
