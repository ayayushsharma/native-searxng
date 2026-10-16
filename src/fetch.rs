//! Single reusable blocking HTTP client. Sequential use only (MVP:
//! one request at a time). Bodies are capped — over-cap responses are
//! truncated at a char boundary, never grown.

use crate::config::Config;
use std::time::Duration;
use ureq::tls::{Certificate, RootCerts, TlsConfig};

/// Custom CA bundle (`[http] ca_file`, work-machine MITM networks).
/// Reads every `-----BEGIN CERTIFICATE-----` block and trusts exactly
/// that set INSTEAD of Mozilla's bundled roots (on an intercepting
/// network the proxy re-signs everything, so the corp CA is the only
/// root needed). Empty path = bundled roots (current default).
/// Errors are loud — a bad file refuses to start, never silently
/// falls back to other roots. Validated in main before any client
/// is built; `Client::new` re-loads and expects success.
pub fn load_ca(path: &str) -> Result<Option<RootCerts>, String> {
    if path.trim().is_empty() {
        return Ok(None);
    }
    let bytes = std::fs::read(path).map_err(|e| format!("ca_file {path:?}: {e}"))?;
    let text = String::from_utf8(bytes).map_err(|_| format!("ca_file {path:?}: not UTF-8 PEM"))?;
    let mut certs: Vec<Certificate<'static>> = Vec::new();
    let mut rest = text.as_str();
    while let Some(b) = rest.find("-----BEGIN CERTIFICATE-----") {
        rest = &rest[b..];
        let Some(e) = rest.find("-----END CERTIFICATE-----") else {
            break;
        };
        let block = &rest[..e + "-----END CERTIFICATE-----".len()];
        match Certificate::from_pem(block.as_bytes()) {
            Ok(c) => certs.push(c),
            Err(e) => return Err(format!("ca_file {path:?}: bad PEM block: {e}")),
        }
        rest = &rest[e + "-----END CERTIFICATE-----".len()..];
    }
    if certs.is_empty() {
        return Err(format!("ca_file {path:?}: no certificates found"));
    }
    Ok(Some(RootCerts::new_with_certs(&certs)))
}

pub struct Client {
    agent: ureq::Agent,
    proxy_agent: ureq::Agent,
    cap: usize,
}

impl Client {
    pub fn new(cfg: &Config) -> Self {
        // Custom roots replace the bundle on BOTH agents (engines and
        // the strangler ride different deadlines, same trust).
        let roots = load_ca(&cfg.ca_file).expect("ca_file validated at startup");
        let base = || {
            let mut b = ureq::Agent::config_builder();
            if let Some(r) = roots.clone() {
                b = b.tls_config(TlsConfig::builder().root_certs(r).build());
            }
            b
        };
        let agent: ureq::Agent = base()
            .timeout_global(Some(Duration::from_millis(cfg.timeout_ms)))
            .user_agent(cfg.user_agent.clone())
            .build()
            .into();
        let proxy_agent: ureq::Agent = base()
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
        let req = crate::engine::OutRequest {
            url: url.to_string(),
            method: crate::engine::Method::Get,
            form: vec![],
            json: None,
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        };
        self.execute(&req).map(|r| (r.status, r.cookies, r.body))
    }

    /// Execute one engine-described request (GET or POST). POST carries
    /// either urlencoded `form` or a raw `json` body. Responses are
    /// capped like GETs; key material never enters errors or logs.
    pub fn execute(
        &self,
        req: &crate::engine::OutRequest,
    ) -> Result<crate::engine::InResponse, String> {
        self.execute_capped(req, None)
    }

    /// `execute` with a per-engine cap override (KB); None = client cap.
    /// Transport failures warn with the HOST only (queries stay out of
    /// the default level); the full URL rides at debug.
    pub fn execute_capped(
        &self,
        req: &crate::engine::OutRequest,
        cap_kb: Option<usize>,
    ) -> Result<crate::engine::InResponse, String> {
        crate::debug!("upstream {:?} {}", req.method, req.url);
        let out = self.execute_inner(req, cap_kb);
        if let Err(e) = &out {
            let mut short = e.clone();
            if short.len() > 120 {
                short.truncate(120);
            }
            crate::warn!("upstream {} -> {}", crate::log::host_of(&req.url), short);
        }
        out
    }

    fn execute_inner(
        &self,
        req: &crate::engine::OutRequest,
        cap_kb: Option<usize>,
    ) -> Result<crate::engine::InResponse, String> {
        let cap = cap_kb.map(|k| k * 1024).unwrap_or(self.cap);
        use crate::engine::Method;
        let mut res = match req.method {
            Method::Get => {
                let mut call = self.agent.get(&req.url);
                for (k, v) in &req.headers {
                    call = call.header(k.as_str(), v.as_str());
                }
                call.call().map_err(|e| e.to_string())?
            }
            Method::Post => {
                let mut call = self.agent.post(&req.url);
                for (k, v) in &req.headers {
                    call = call.header(k.as_str(), v.as_str());
                }
                if let Some(json) = req.json.as_ref() {
                    call = call.content_type("application/json");
                    call.send(json.as_bytes()).map_err(|e| e.to_string())?
                } else {
                    let form: String = req
                        .form
                        .iter()
                        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
                        .collect::<Vec<_>>()
                        .join("&");
                    call = call.content_type("application/x-www-form-urlencoded");
                    call.send(form.as_bytes()).map_err(|e| e.to_string())?
                }
            }
        };
        let status = res.status().as_u16();
        let mut cookies = vec![];
        for v in res.headers().get_all("set-cookie") {
            if let Ok(s) = v.to_str() {
                cookies.push(s.to_string());
            }
        }
        let body = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        if body.len() <= cap {
            return Ok(crate::engine::InResponse {
                status,
                cookies,
                body,
            });
        }
        let mut s = body;
        s.truncate(s.floor_char_boundary(cap).min(s.len()));
        Ok(crate::engine::InResponse {
            status,
            cookies,
            body: s,
        })
    }

    /// Strangler proxy: forward raw path+query to the Python reference.
    /// Uses the long-deadline agent (full oracle fan-out needs seconds).
    pub fn proxy(&self, base: &str, path_query: &str) -> Result<(u16, String), String> {
        let url = format!("{}{}", base.trim_end_matches('/'), path_query);
        let mut res = self
            .proxy_agent
            .get(&url)
            .call()
            .map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let body = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
        Ok((status, body))
    }
}

/// application/x-www-form-urlencoded pair encoding (POST forms).
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
