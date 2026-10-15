//! Single reusable blocking HTTP client. Sequential use only (MVP:
//! one request at a time). Bodies are capped — over-cap responses are
//! truncated at a char boundary, never grown.

use crate::config::Config;
use std::time::Duration;

pub struct Client {
    agent: ureq::Agent,
    proxy_agent: ureq::Agent,
    cap: usize,
}

impl Client {
    pub fn new(cfg: &Config) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(cfg.timeout_ms)))
            .user_agent(cfg.user_agent.clone())
            .build()
            .into();
        let proxy_agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(cfg.proxy_timeout_ms)))
            .user_agent(cfg.user_agent.clone())
            .build()
            .into();
        Self {
            agent,
            proxy_agent,
            cap: cfg.body_cap_bytes(),
        }
    }

    /// GET url, return body truncated to cap. Err(String) is log-safe
    /// (no query content) — mark engine unresponsive on any error.
    pub fn get(&self, url: &str) -> Result<String, String> {
        self.get_with(url, &[])
    }

    /// GET with extra headers (e.g. `Authorization: Bearer <key>`).
    /// Key material never appears in errors — ureq errors carry status
    /// text only, and we never log request headers.
    pub fn get_with(&self, url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
        self.get_full(url, headers).map(|(_, _, body)| body)
    }

    /// Full fetch: (HTTP status, set-cookie values, body truncated to cap).
    /// Status is surfaced so the server can report non-200 legs honestly
    /// (303 → spamprotection, 403, 429) instead of parsing block pages
    /// as results. Cookies are returned raw; engines pick what they need.
    pub fn get_full(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<(u16, Vec<String>, String), String> {
        let mut call = self.agent.get(url);
        for (k, v) in headers {
            call = call.header(*k, *v);
        }
        let mut res = call.call().map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let mut cookies = vec![];
        for v in res.headers().get_all("set-cookie") {
            if let Ok(s) = v.to_str() {
                cookies.push(s.to_string());
            }
        }
        let body = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        if body.len() <= self.cap {
            return Ok((status, cookies, body));
        }
        let mut s = body;
        s.truncate(s.floor_char_boundary(self.cap).min(s.len()));
        Ok((status, cookies, s))
    }

    /// Strangler proxy: forward raw path+query to the Python reference.
    /// Uses the long-deadline agent (full oracle fan-out needs seconds).
    pub fn proxy(&self, base: &str, path_query: &str) -> Result<(u16, String), String> {
        let url = format!("{}{}", base.trim_end_matches('/'), path_query);
        let mut res = self.proxy_agent.get(&url).call().map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        Ok((status, body))
    }
}
