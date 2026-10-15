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
