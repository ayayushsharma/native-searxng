# Footprint (disk + RAM), measured 2026-10-14

Method: `tools/measure.sh <bin>` — `ls/size` for disk, then 50 sequential
`q=test&format=json` general queries while polling the SERVER's RSS via
`ps` (an earlier version timed the curl client — wrong process, fixed).

## Disk

| Binary | Size | Notes |
|---|---|---|
| `target/debug/native-searxng` | 36.1 MB (36,093,824 B) | debuginfo + unopt; dev-only, never shipped (−10MB after scraper+encoding_rs removal) |
| release x86_64 (CI artifact) | 3.32 MB (3,321,528 B) | `opt-z + strip + panic=abort + codegen-units=1`, no LTO yet |
| release Android aarch64 (CI artifact) | 3.01 MB (3,007,216 B) | same profile |

## RAM (server RSS, 50-query loop, polled 1/s)

| Binary | Peak | Avg | Ceiling | Status |
|---|---|---|---|---|
| dev (unoptimized), 4 lanes, no scraper | 15.8 MB | 14.2 MB | 10 MB soft | ABOVE — expected for dev (upper bound); lanes +1.4, scraper-drop −0.8 |
| release x86_64 (CI, run 38613483575) | 9.7 MB | 9.5 MB | 10 MB soft / 5 MB core-only | PASSES with 0.3 MB margin — margin too thin; diet queue (GAP-PLAN P2 / LTO / TLS-pool / scraper) is justified |
| release Android on-device (run 38682374578, N=200) | 11.4 MB | 9.0 MB | 10 MB soft | BREACH on extended run — peak over ceiling while avg holds. Trend unknown (pool fill vs slow growth); needs per-sample series + diet before next wave. |
| dev (unoptimized), 2026-10-15 re-run, N=10 | 17.6 MB | 15.6 MB | 10 MB soft | Run variance ±2MB (pool fill + which legs hang); trend flat within run |
| **oracle python (dev server, same 10×q=test loop)** | **184.4 MB** | **184.2 MB** | n/a | Flat mature heap — no growth. Native dev is **~12× smaller**; native release (~10MB) is **~18× smaller** |

## Head-to-head 2026-10-15 (36q quality.py + 8q latency.py, SLEEP_S=2, local-only)

| | native | oracle | note |
|---|---|---|---|
| results/query (total) | 18.6 | 41.4 | gap = our 20-result merge cap vs oracle uncapped fan-out + dead engines (ddg/gmx/429s) |
| general count | 20.0 | 19.4 | **parity** — weakest category closed (GAP G2) |
| it / science / packages | 20.0 / 19.2 / 15.5 | 52.5 / 73.0 / 83.5 | capped at 20 by merge; overlap 8–14/q |
| news | 15.3 | 21.8 | closest uncapped category; latency at parity |
| URL overlap | 7.9/q | — | up from 7.8 (lanes+scanners didn't shift content) |
| latency mean / p50 / max (8q) | 3.5s / 3.0s / 7.4s | 2.1s / 2.1s / 3.9s | small categories native ≤ oracle; **general lags 2–3×** (12 engines, 3s-timeout legs on ddg/startpagina/gmx hang the lanes) |
| unresponsive (native / ref) | ddg 14, gmx 14, neosearch 14(429), startpagina 9 | brave 14, duckduckgo 14, news-verticals 6 | both sides carry dead weight; ours is honest-statused |

## Reading the numbers

- Dev RSS overstates release: no optimizations, debug assertions, fatter
  codegen. It is a ceiling probe, not the grade.
- If release RSS lands 6–9 MB: optimize before the next engine wave
  (body-cap, DOM lifetimes, per-engine body retention).
- If release RSS lands >10 MB: breach-report per PLAN.md §3, stop new
  engines until resolved.
- Suspects if high: 96KB body cap × 10 concurrent engines (~1 MB live),
  `scraper` DOMs for large HTML (Bing ~280KB in), serde_json Values for
  65–70KB API bodies (GitHub, Europe PMC).
