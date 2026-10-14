# Footprint (disk + RAM), measured 2026-10-14

Method: `tools/measure.sh <bin>` — `ls/size` for disk, then 50 sequential
`q=test&format=json` general queries while polling the SERVER's RSS via
`ps` (an earlier version timed the curl client — wrong process, fixed).

## Disk

| Binary | Size | Notes |
|---|---|---|
| `target/debug/native-searxng` | 44.7 MB (44,662,792 B) | debuginfo + unopt; dev-only, never shipped |
| release x86_64 (CI artifact) | 3.32 MB (3,321,528 B) | `opt-z + strip + panic=abort + codegen-units=1`, no LTO yet |
| release Android aarch64 (CI artifact) | 3.01 MB (3,007,216 B) | same profile |

## RAM (server RSS, 50-query loop, polled 1/s)

| Binary | Peak | Avg | Ceiling | Status |
|---|---|---|---|---|
| dev (unoptimized) | 12.6 MB | 12.4 MB | 10 MB soft | ABOVE — expected for dev (upper bound) |
| release x86_64 | TBD | TBD | 10 MB soft / 5 MB core-only | must be measured in CI (phone never builds release) |

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
