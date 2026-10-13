//! Single reusable blocking HTTP client. Sequential use only (MVP:
//! one request at a time). Bodies are capped — over-cap responses are
//! truncated at a char boundary, never grown.

use crate::config::Config;
use std::time::Duration;

pub struct Client {
    agent: ureq::Agent,
    cap: usize,
}

impl Client {
    pub fn new(cfg: &Config) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(cfg.timeout_ms)))
            .user_agent(cfg.user_agent.clone())
            .build()
            .into();
        Self {
            agent,
            cap: cfg.body_cap_bytes(),
        }
    }

    /// GET url, return body truncated to cap. Err(String) is log-safe
    /// (no query content) — mark engine unresponsive on any error.
    pub fn get(&self, url: &str) -> Result<String, String> {
        let mut res = self.agent.get(url).call().map_err(|e| e.to_string())?;
        let body = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        if body.len() <= self.cap {
            return Ok(body);
        }
        let mut s = body;
        s.truncate(s.floor_char_boundary(self.cap).min(s.len()));
        Ok(s)
    }

    /// Strangler proxy: forward raw path+query to the Python reference.
    pub fn proxy(&self, base: &str, path_query: &str) -> Result<(u16, String), String> {
        let url = format!("{}{}", base.trim_end_matches('/'), path_query);
        let mut res = self.agent.get(&url).call().map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        Ok((status, body))
    }
}
