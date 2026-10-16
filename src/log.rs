//! Zero-crate leveled logging (PLAN.md §3: no new deps without a RAM
//! line-item, and `log`+friends buy nothing here). One global atomic
//! holds the level — set once from `[logging] level` at startup — so
//! any module (`server`, `fetch`, `mcp`, engines) can emit without
//! threading a config through. Disabled levels skip formatting
//! entirely (the macros check `enabled()` first). All output goes to
//! stderr with `+elapsed level` prefix; MCP stdio is safe (protocol
//! rides stdout). Levels: off < error < warn < info < debug.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);
static START: OnceLock<Instant> = OnceLock::new();

/// Wire verbosity, least to most. Discriminants ARE the wire order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Level {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

impl Level {
    /// Parse a `[logging] level` value; unknown names are a loud
    /// config error (never a silent default).
    pub fn parse(s: &str) -> Option<Level> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Some(Level::Off),
            "error" => Some(Level::Error),
            "warn" => Some(Level::Warn),
            "info" => Some(Level::Info),
            "debug" => Some(Level::Debug),
            _ => None,
        }
    }

    fn tag(self) -> &'static str {
        match self {
            Level::Off => "OFF",
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }
}

/// Set once at startup from the conf (`main.rs`); tests may reset.
pub fn set(level: Level) {
    LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn enabled(level: Level) -> bool {
    level as u8 <= LEVEL.load(Ordering::Relaxed) && level != Level::Off
}

/// Formatted line to stderr. Prefer the macros (they skip formatting
/// when disabled); call directly only with a prebuilt string.
pub fn emit(level: Level, msg: String) {
    if !enabled(level) {
        return;
    }
    let start = START.get_or_init(Instant::now);
    let e = start.elapsed();
    eprintln!("[+{:>7.3}s] {:<5} {}", e.as_secs_f64(), level.tag(), msg);
}

/// Host of an upstream URL for warn lines (queries stay OUT of the
/// default level — full URLs only at debug). Hand-split, no deps.
pub fn host_of(url: &str) -> &str {
    let u = url.split("://").nth(1).unwrap_or(url);
    u.split('/').next().unwrap_or(u)
}

#[macro_export]
macro_rules! error {
    ($($t:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Error) {
            $crate::log::emit($crate::log::Level::Error, format!($($t)*))
        }
    };
}

#[macro_export]
macro_rules! warn {
    ($($t:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Warn) {
            $crate::log::emit($crate::log::Level::Warn, format!($($t)*))
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($t:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Info) {
            $crate::log::emit($crate::log::Level::Info, format!($($t)*))
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($t:tt)*) => {
        if $crate::log::enabled($crate::log::Level::Debug) {
            $crate::log::emit($crate::log::Level::Debug, format!($($t)*))
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_parse_and_order() {
        assert_eq!(Level::parse("debug"), Some(Level::Debug));
        assert_eq!(Level::parse(" WARN "), Some(Level::Warn));
        assert_eq!(Level::parse("verbose"), None);
        assert!(Level::Error < Level::Warn);
        assert!(Level::Warn < Level::Info);
        assert!(Level::Info < Level::Debug);
    }

    #[test]
    fn gate_filters() {
        set(Level::Warn);
        assert!(enabled(Level::Error));
        assert!(enabled(Level::Warn));
        assert!(!enabled(Level::Info));
        assert!(!enabled(Level::Debug));
        assert!(!enabled(Level::Off));
        set(Level::Info); // leave the default for everyone else
    }

    #[test]
    fn host_of_hides_query() {
        assert_eq!(host_of("https://api.example.com/search?q=secret+x"), "api.example.com");
        assert_eq!(host_of("http://h:8123/p"), "h:8123");
    }
}
