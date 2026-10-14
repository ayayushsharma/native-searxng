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
    pub max_results: usize,
    pub lang: String,
    pub user_agent: String,
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
            max_results: 20,
            lang: "en".into(),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".into(),
            proxy_to: String::new(),
        }
    }
}

impl Config {
    pub fn body_cap_bytes(&self) -> usize {
        self.body_cap_kb * 1024
    }
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
            "max_results" => cfg.max_results = v.trim().parse().map_err(|_| "bad max_results")?,
            "lang" => cfg.lang = v.trim().to_string(),
            "user_agent" => cfg.user_agent = v.trim().to_string(),
            "proxy_to" => cfg.proxy_to = v.trim().to_string(),
            _ => {}
        }
    }
    Ok(cfg)
}
