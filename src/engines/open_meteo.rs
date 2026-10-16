//! Open-Meteo (weather) — keyless JSON, two-step (new-category #3).
//! Leg 1: `GET geocoding-api.open-meteo.com/v1/search?name=..&count=1`
//! → `{results:[{latitude, longitude}]}`. Leg 2: `GET
//! api.open-meteo.com/v1/forecast?latitude=..&longitude=..&current=..`
//! (`rounds=2`, pubmed pattern). Parse emits ONE summary result —
//! upstream returns a `WeatherAnswer` into `answers[]`, but the native
//! pipeline has no answers producers (cheatsh/jisho precedent: normal
//! results), so conditions ride as a result row. Place name comes from
//! the forecast `timezone` (`Europe/Berlin` → Berlin; underscores →
//! spaces) because `parse` only sees the last body. Condition words are
//! upstream's `WMO_TO_CONDITION` table; wind is an 8-point compass.
//! Unknown place → leg 2 never derives → honest `session flow failed`.
//! Fixtures: `tests/fixtures/tests-fixture-openmeteo-{geo,test}.json` (HTTP 200, clean).

use crate::engine::{Engine, InResponse, OutRequest, SearchResult};
use serde_json::Value;

pub struct OpenMeteo;

const CURRENT: &str = "temperature_2m,apparent_temperature,relative_humidity_2m,cloud_cover,pressure_msl,wind_speed_10m,wind_direction_10m,weather_code";

impl Engine for OpenMeteo {
    fn name(&self) -> &'static str {
        "open_meteo"
    }

    fn category(&self) -> &'static str {
        "weather"
    }

    fn rounds(&self) -> usize {
        2
    }

    fn build_url(&self, q: &str, _pageno: u32) -> String {
        format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
            encode(q)
        )
    }

    fn request(&self, query: &str, pageno: u32, prev: &[InResponse]) -> Option<OutRequest> {
        if prev.is_empty() {
            return Some(OutRequest::get(self.build_url(query, pageno)));
        }
        if prev.len() > 1 {
            return None;
        }
        let v: Value = serde_json::from_str(&prev[0].body).ok()?;
        let r = v.get("results")?.as_array()?.first()?;
        let lat = r.get("latitude")?.as_f64()?;
        let lon = r.get("longitude")?.as_f64()?;
        Some(OutRequest::get(format!(
            "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current={CURRENT}&daily=temperature_2m_max,temperature_2m_min&forecast_days=1&timezone=auto"
        )))
    }

    fn parse(&self, body: &str) -> Vec<SearchResult> {
        let v: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(_) => return vec![],
        };
        let cur = match v.get("current") {
            Some(c) => c,
            None => return vec![],
        };
        let num = |k: &str| cur.get(k).and_then(|x| x.as_f64());
        let (temp, feels, hum, cloud, pres, wind, wdir, code) = (
            num("temperature_2m"), num("apparent_temperature"), num("relative_humidity_2m"),
            num("cloud_cover"), num("pressure_msl"), num("wind_speed_10m"),
            num("wind_direction_10m"), num("weather_code"),
        );
        // All eight current fields required — a partial block is a
        // schema change, not weather (skip, don't half-report).
        let (temp, feels, hum, cloud, pres, wind, wdir, code) =
            match (temp, feels, hum, cloud, pres, wind, wdir, code) {
                (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f), Some(g), Some(h)) => {
                    (a, b, c, d, e, f, g, h)
                }
                _ => return vec![],
            };
        let place = v
            .get("timezone")
            .and_then(|x| x.as_str())
            .and_then(|tz| tz.rsplit('/').next())
            .map(|s| s.replace('_', " "))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown location".to_string());
        let daily = v.get("daily");
        let day = |k: &str| {
            daily
                .and_then(|d| d.get(k))
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .and_then(|x| x.as_f64())
        };
        let mut content = format!(
            "{temp:.1}°C, {}. Feels like {feels:.1}°C. Wind {wind:.1} km/h from {}. Humidity {hum:.0}%. Pressure {pres:.0} hPa. Cloud cover {cloud:.0}%.",
            condition(code as i64),
            compass(wdir),
        );
        if let (Some(hi), Some(lo)) = (day("temperature_2m_max"), day("temperature_2m_min")) {
            content.push_str(&format!(" High {hi:.0}° / Low {lo:.0}°."));
        }
        let title = format!("{place} — current weather (Open-Meteo)");
        let mut res = SearchResult::basic(
            "open_meteo",
            "https://open-meteo.com/".to_string(),
            title,
            content,
        );
        if let Some(t) = cur.get("time").and_then(|x| x.as_str()).filter(|s| s.len() >= 10) {
            res.published_date = Some(t[..10].into());
        }
        vec![res]
    }
}

/// Upstream `WMO_TO_CONDITION` (open_meteo.py). Unlisted codes fall back
/// to the raw number — new WMO codes must never blank the row.
fn condition(code: i64) -> &'static str {
    match code {
        0 => "clear sky",
        1 => "fair",
        2 => "partly cloudy",
        3 => "cloudy",
        45 | 48 => "fog",
        51 | 53 | 55 => "light rain",
        56 => "light sleet showers",
        57 => "light sleet",
        61 => "light rain",
        63 => "rain",
        65 => "heavy rain",
        66 => "light sleet showers",
        67 => "light sleet",
        71 => "light sleet",
        73 => "sleet",
        75 => "heavy sleet",
        77 => "snow",
        80 => "light rain showers",
        81 => "rain showers",
        82 => "heavy rain showers",
        85 => "snow showers",
        86 => "heavy snow showers",
        95 => "rain and thunder",
        96 => "light snow and thunder",
        99 => "heavy snow and thunder",
        _ => "unknown conditions",
    }
}

fn compass(deg: f64) -> &'static str {
    const PTS: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];
    PTS[((deg / 45.0).round() as usize) % 8]
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
