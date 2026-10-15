# native-searxng — Embedded Plan (≤10 MB RAM, single user)

Goal: native (no Python) reimplementation of SearXNG metasearch core for
embedded Linux (e.g. OpenWrt router class device). Target: **steady RSS
~5–6 MB, peak <10 MB**, single user, reduced features. (The 5 MB aspiration
is conceded; 10 MB is the ceiling and CI asserts it.)

> Full SearXNG (Python/Flask, 253 engines, Jinja, lxml, Babel/5.5 MB locales,
> Valkey, metrics) cannot fit 5–10 MB — CPython alone is ~15–20 MB RSS.
> So this is a **clean-room minimal core**, not a port. Reuse SearXNG engine
> *logic* (URLs + selectors) as spec only.

## 1. Scope: what stays / what goes

### Keep (MVP)
- `GET /` — tiny search form (single inline HTML, no JS)
- `GET /search?...` — **same contract as SearXNG, no new API**: params
  `q, pageno, language, safesearch, categories, engines, format=json|html`
  (SearXNG defaults `formats: [html]` — native enables `json` from day one since
  agents consume it); errors as `{"error": msg}` like `index_error`.
  JSON top-level keys identical: `query, results, answers, corrections, infoboxes,
  suggestions, unresponsive_engines`. Result objects carry the same keys agents
  already parse: `url, title, content, engine/engines, publishedDate, positions,
  score, category` (plus `template` defaulted, media keys empty for general results).
  Simple implementation: flat structs with these exact key names — no new schema,
  no new client code for existing SearXNG JSON consumers.
- `GET /healthz`, `GET /opensearch.xml` — health + browser integration
- General-web engines, sequential fetch, merge + dedup + trivial rank
- `searxng.conf` INI (no cookies / sessions / web preferences UI)
- Bounded JSON + HTML output (cap counts/lengths)
- Optional tiny in-RAM LRU cache (default OFF)
- Strangler `proxy_to`: unknown categories/engines forward to the Python
  reference transparently (full agent-visible coverage from day one)

### Cut (explicitly out of MVP)
- 249/253 engines. Order: JSON engines first (15, near-free: URL + serde),
  then high-value HTML (Wikipedia-live, DDG, Mojeek, Bing, Brave), long tail later.
  New engine = one file in `src/engines/` + one line in `mod.rs`.
- Verticals: images/video/news/files — scrape cost + RAM not worth it.
  General web only in MVP (news/science categories proxy to Python).
- `/preferences`, `/stats`, `/metrics`, `/image_proxy`, `/favicon_proxy`,
  `/autocompleter`, `/info/*`, translations (English only), themes, plugins,
  answerers, weather, bangs, Valkey/SQLite, rate limiter, metrics, POST search.
- No async runtime, no regex engine, no JS client bundle.

### Request pipeline (single-user, sequential to save RAM)
```
parse query (trim, +language, +pageno clamped 1..3, +safesearch flag)
  → strangler check (unknown category/engine → proxy or 503)
  → per configured engine, sequentially: build URL → GET (3 s timeout,
    body cap 96 KB, truncate at char boundary) → parse → ≤10 results
  → dedup by normalized URL → round-robin merge → positions assigned
  → render JSON (SearXNG keys) or minimal HTML → respond
```
No parallelism needed. Single-threaded server = queue depth 1 naturally.
Keeps heap flat.

## 2. Language / runtime choice — FROZEN: Rust (sync-only discipline)

Decided 2026-10-14 after on-phone benchmarks, confirmed by working scaffold.

Why Rust wins *here* despite slower cold builds:
- The consumer is agentic: recall under rate limits, clean fields, trustworthy dates
  need Chrome-impersonating fetch, selectors, encoding tables, date parsing — this
  ecosystem exists only in Rust. (Proven 2026-10-14: DDG serves an `anomaly.js?
  cc=botnet` challenge to plain clients — see `tests-fixture-ddg-anomaly.html`.
  Impersonation is not optional, it's the first engine ticket: `wreq`.)
- One process, one RSS budget, one toolchain — no hybrid, no sidecars, no FFI.
- Cold builds are slow but amortized: `target/` lives on 35 GB free disk, daily
  iteration is warm incrementals; `cargo check` skips LLVM entirely.

Measured on the F12 homeserver (Exynos 850 8×A55, Termux, rustc 1.99.0):
full `cargo build -j2` = 4m27s, cold `cargo check -j2` = 1m31s,
warm `cargo check` after touching one engine = **0.8 s**.
Debug binary 43 MB unstripped (expected); release (`opt-z`, strip,
`panic=abort`) targets <800 KB.

Discipline (binding — this is what keeps Rust inside budget):
- SYNC ONLY: blocking client + 4-lane scoped server threads. No tokio/async runtime.
- Minimal deps, all audited (Cargo.toml): `tiny_http` (server), `ureq` (client,
  rustls/ring + gzip only), `serde`/`serde_json`, hand-rolled INI + query
  parsing + HTML scanners + SHA-256/b64url (`src/crypto.rs`).
  REMOVED 2026-10-15: `scraper` (html5ever stack), `encoding_rs` — replaced by
  `engine::attr/strip_tags/ddg_parse` hand scanners (dev bin −10MB).
  REJECTED 2026-10-14: `wreq` (Chrome spoofing) — audited the
  crate (v0.16.1): async-first (tokio in default features, no blocking client),
  TLS is BoringSSL via btls-sys (bindgen + libclang + C compile at build time),
  111 packages (+40 over our tree). Violates all three hard constraints at once.
  Saved est. 2–4 MB by rejecting it. Impersonation strategy is avoid-first:
  native serves unchallenged endpoints (Wikipedia/JSON APIs verified from this
  network); guarded engines stay behind the strangler where Python keeps
  curl_cffi. New crate = RAM line-item.
- Phone builds: `cargo check -j4` default gate (light jobs), `cargo build -j2`
  only to run (codegen is the LMK risk — never -j8); always under
  `~/scripts/safe-build.sh`. Releases cross-built on PC
  (`aarch64-linux-android` for phone, `aarch64-unknown-linux-musl` for router).
- Zig stays only as emergency fallback (core-only lenient-engine build) if Rust
  RSS misses 10 MB in measurement — never both at once.

Rejected: C (vetoed — build/maintainability), Go (≥8–15 MB runtime misses budget),
Zig lead (would hand-roll spoofing/selectors/encoding/dates = C pain),
hybrid (two toolchains + supervision on a fragile box).

## 3. Memory budget (design to this)

| Region | Budget | How enforced |
|---|---|---|
| Binary text + rustls/ring (resident; no html5ever/encoding tables since 2026-10-15) | ~3–4 MB | `opt-level="z"`, `lto=true`, strip, `panic=abort`, system CA (no embedded roots) |
| 4 lane stacks (scoped threads, demand-paged) + 4 malloc arenas | <2 MB touched | lanes=4 fixed (GAP-PLAN P2); per-engine threads measured +3MB, rejected |
| Per-request buffers (one engine body 96 KB + results ≤20 × ~1 KB) | <256 KB peak | body cap + truncate, result caps, freed at end of scope |
| Global (config, engine table, UA strings) | ~50 KB | parsed once at startup |
| LRU cache (optional, default off) | 0 | fixed slots when enabled, never unbounded |
| **Total steady / peak** | **~5–6 MB / <10 MB** | `tools/measure.sh` (size + 50-query RSS); CI asserts peak <10 MB |

Rules: sequential fetch only; every network copy length-checked; truncate over
cap at char boundary; zero regex; parsers skip malformed input (never crash).
Golden fixtures per engine guard markup churn; anomaly/challenge pages must
yield 0 results + `unresponsive_engines` entry, never garbage.

## 4. Repo layout (actual — scaffold compiles, phone-verified)
```
native-searxng/
  PLAN.md            (this file)
  README.md          (build + footprint table)
  Cargo.toml / Cargo.lock (frozen dep set — lockfile is binding)
  searxng.conf       (port, bind, engines, timeouts, caps, proxy_to)
  src/
    main.rs          (load conf, run server)
    server.rs        (tiny_http loop; routes /, /search, /healthz, /opensearch.xml)
    config.rs        (hand INI)
    query.rs         (hand query-string parse + percent-decode)
    engine.rs        (Engine trait + SearchResult with SearXNG key names)
    engines/mod.rs   (find-by-name table)
    engines/wikipedia.rs (live: opensearch JSON API)
    engines/ddg.rs   (fixture-driven HTML selectors)
    fetch.rs         (ureq wrapper: timeouts, caps, strangler proxy)
    merge.rs         (dedup + round-robin + positions)
    render.rs        (SearXNG-exact JSON envelope + minimal HTML)
    cache.rs         (optional LRU, default off)
  agentic/           (agent surface: CONTRACT.md frozen keys, evals/check.py gate,
                      diff/diff.py Python-vs-native drift, clients/ examples)
  tools/measure.sh   (size + RSS footprint gate)
  tests-fixture-ddg-anomaly.html (real bot-challenge sample — parser must skip it)
```

## 5. Config (actual `searxng.conf`)
```ini
port=8888
bind=127.0.0.1
engines=wikipedia,ddg
timeout_ms=3000
body_cap_kb=96
max_results=20
lang=en
user_agent=Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36
proxy_to=http://127.0.0.1:8889   # Python reference; empty = 503 for unported
```

## 6. Build & verify
- Phone (Termux): `safe-build.sh cargo check -j4` to iterate (0.8 s warm),
  `safe-build.sh cargo build -j2` to run. Never -j8 (LMK kills the whole app).
  Phone pulls via `git pull` (own network + git client) — no manual syncs.
- GitHub Actions (`.github/workflows/build.yml`, see `AGENTS.md` for the credit
  policy): `cargo check` gate on every push; ONE release job (x86_64 +
  Android back-to-back, single runner) on `v*` tags or manual dispatch only.
  `aarch64-unknown-linux-musl` (router) deferred until a router ships.
  `concurrency: cancel-in-progress` so pushes never queue behind stale runs.
  CI owns all releases; nobody cross-builds by hand.
- Gate every engine: `python3 agentic/evals/check.py --base <native>` (contract
  keys + field presence + latency) and `agentic/diff/diff.py` vs Python ref
  (no key drift). Fixtures: save real engine responses, parse offline.
- Footprint: `tools/measure.sh` — asserts peak <10 MB over 50 sequential queries.

## 7. Phases
- **W1 (done: scaffold + vertical slice):** crate skeleton compiles clean;
  phone build proven (4m27s); `cargo check` loop proven (0.8 s warm);
  Wikipedia live end-to-end with exact SearXNG JSON keys; strangler proxy;
  eval/diff harness runs. [2026-10-14]
- **W1 remaining:** oracle-first: verify the Python reference passes engine walls
  from this network BEFORE porting any guarded engine (if the oracle is also
  challenged, STOP — network-level call). Then: Mojeek/Bing/DDG *attempts* via
  the oracle verdict, JSON-engine batch (near-free each), fixtures per engine.
  NOTE 2026-10-14: home egress IP is challenged by DDG-html, DDG-lite
  (`anomaly.js?cc=botnet`) AND Mojeek (`<title>Captcha</title>`) even with browser
  UAs — fingerprint/IP-level gating, not UA. Only Wikipedia/JSON verified clean.
- **W2:** top-10 HTML engines + codegen stubs from Python engine AST +
  differential tests green.
- **W3:** hardening (timeouts, `ban_time` per failing engine, dates via jiff,
  encoding coverage) + release profiles measured against the 10 MB ceiling.

## 8. Risks
- Engine HTML churn / bot walls (DDG/Bing challenge or 429 — OBSERVED).
  Mitigate: impersonation, 2+ engines, per-engine `ban_time`, fixtures.
- TLS handshake spikes (~100–300 KB transient). Mitigate: sequential fetch,
  connection reuse, small cipher list.
- DNS stalls blow budget → short connect timeout + overall deadline.
- Feature creep silently re-inflates RAM. Mitigate: every feature needs a RAM
  line-item.
- One-off infra flakes (ETXTBSY on a raced `target/`, tunnel drops) — keep
  build logs until root-caused, single writer per `target/`.

## 10. Dependency audit (2026-10-14, vs `~/projects/personal/searxng/`)

| Python dep | Role | MVP fate | Rust answer (in Cargo.toml) |
|---|---|---|---|
| curl_cffi, impersonate=`chrome` | TLS/HTTP2 fingerprint spoofing vs bot walls | KEEP (proven necessary — DDG challenges plain clients) | `ureq` now, `wreq` ticketed (pure-Rust Chrome spoofing) |
| lxml + XPath (94 engines) | HTML scraping, the dominant parser style | KEEP concept (selectors + skip-on-mismatch) | `scraper` |
| flask + server stack | server + static | trivial | `tiny_http` (sync, single-threaded) |
| jinja2, babel + 5.5 MB locales, yaml, valkey, markdown | templates/i18n/config/cache/docs | DROP (flat HTML, English-only, hand INI, optional LRU) | — |
| dateutil.parser, msgspec structs, certifi | fuzzy dates, result structs, CA bundle | KEEP trimmed (`jiff` later, serde structs now, system CA) | `serde`/`serde_json` now; `encoding_rs` in tree |

## 11. History (superseded — do not resurrect)
- C99, Go, Zig-lead, and Zig+Rust hybrid were evaluated and rejected (§2).
  Zig remains emergency fallback only. No sidecars, no FFI, one process.

## 12. Agentic porting speed (strangler + codegen)

Fastest agent-visible capability, in order:
1. **Strangler from day 1:** Python SearXNG on :8889; native on :8888 serves the
   hot path, unknown categories proxy through. Full agent coverage immediately,
   native takes over incrementally — no flag day.
2. **JSON engines first (15, near-free):** URL + serde structs, no selectors —
   and verified unchallenged from this network (unlike DDG/Mojeek/Bing, §7).
   Guarded HTML only after the oracle verdict.
3. **Codegen scaffolding:** Python engine AST (URL templates, XPaths, paging)
   → Rust `Engine` stubs; fix selectors against fixtures, not blank files.
4. **Table-driven engines:** URL + selector map + field map as data (~30–60
   lines each). Shared fetch/normalize/dedup does the rest.
5. **Python as oracle:** golden fixtures recorded once; differential tests
   auto-flag regressions. No live rate-limit burn during dev.
Defer: JS-required engines, API-key engines, images/torrents, HTML UI polish,
translations — agents consume JSON.
Implementation stays simple: one process, flat structs with SearXNG key names,
no new abstractions beyond `Engine` + merge + LRU. `agentic/` holds only evals +
diff runner + examples — the contract IS SearXNG's `/search&format=json`.

## 13. Open questions
1. Deploy target exactness (OpenWrt version? musl? router RAM/flash)? Build host
   is F12/Termux (decided); release cross happens on PC regardless.
2. Update story (opkg + static binary, or sysupgrade image)?
3. Release RSS measurement (vs 10 MB ceiling) — pending first `--release` on PC
   + `--target` phone/router builds. Rejection of `wreq` (tokio + BoringSSL +
   40 crates) keeps this ballpark comfortable; measure to confirm.
4. Where the Python oracle runs (strangler's :8889): PC, phone, or public
   instance as fallback? Undecided — the proxy needs a home before W1 ends.
