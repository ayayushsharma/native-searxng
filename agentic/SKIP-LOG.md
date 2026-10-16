# SKIP LOG — engines probed from home egress and deferred (2026-10-14)

Rule: anything needing keys, JS rendering, CAPTCHA solving, or TLS
impersonation = skip + log (AGENT-MISSION.md stop triggers). Guarded
engines stay behind the strangler (`proxy_to` → Python `:8889`).

## KEYED PARKING LOT (2026-10-15 — parked, free-tier ranked)

Parked, not dead: each gets a dormant `SEARXNG_API_KEY_<ENGINE>` slot
(env-only, never in repo) if ever pursued. Ranked by free-tier
generosity, verified 2026-10-15 unless noted.

| # | Engine | Free tier | Notes |
|---|---|---|---|
| 1 | exaapi | **$120/yr free credits** to start | Most generous. Search + contents — also a `web_fetch` candidate. |
| 2 | jina | **Free tier, works even keyless** (free key = better limits) | **PORTED 2026-10-15 as `/fetch`** (Reader extraction, `SEARXNG_API_KEY_JINA` optional). |
| 3 | braveapi | **$5 free monthly credits** | Directly fixes general (weakest category). Slot design already approved. |
| 4 | kagi | Paid plans only (no meaningful free tier) | Pursue only with a paid account on the table. |
| 5 | mojeek API | Paid plans (memory — reverify if pursued) | Walled HTML + paid API, same funnel as Brave. |
| 6 | marginalia | Unknown (indie; reverify if pursued) | — |
| 7 | bing API | Status uncertain (retirement rumors — verify before any work) | — |
| — | google CSE | **Dead to new customers, sunset 2027-01-01** | Off the table by Google's decision. |
| — | azure (ResourceGraph) | Requires ARM API key (`require_api_key: True` upstream) | Keyed cloud-inventory query, not docs search. Parked with keyed lot. |

## CLOUD-DOCS SURVEY (2026-10-15)

Probed because `it` returned zero official doc URLs (SO/GitHub/MDN only).

| Cloud | Verdict |
|---|---|
| Microsoft | **PORTED 2026-10-15 as `microsoftlearn`** — keyless `learn.microsoft.com/api/search` (mirrors upstream `microsoft_learn.py`: scoringprofile + `$top/$skip` + partnerId). Live 10 hits, real doc URLs + dates. Covers Azure/MS365/.NET. |
| AWS | PARKED — docs search is a JS shell (4KB boot page, endpoint buried in rotating hashed bundles). No stable keyless JSON. Revisit only if a stable endpoint surfaces. |
| GCP | PARKED — `cloud.google.com/search` is a 688KB JS shell (results hydrated client-side). No keyless JSON. Same revisit rule. |

Rule: no key work without a key in hand (user-owned, local XDG/env).
First slot if unlocked: braveapi (category impact) or exa (generosity).

## PORT QUEUE — 14 engines (2026-10-15, keyed APIs parked)

Order: simplest-first; POST-gated and complexity-decision items flagged.
Each: oracle survey (≤1 live probe) → fixture → port → `check.py` →
`quality.py` delta → one commit. Traffic budget: ≤14 probes + gates.

| # | Engine | Kind | Note |
|---|---|---|---|
| 1 | boardreader | session | **DONE 2026-10-15** (`cf004d7`) — homepage session id + return.php, 10 JSON results. |
| 2 | fireball | session (cookie) | **RE-SKIPPED 2026-10-15**: Cloudflare 403 host-wide from home egress (POST /settings AND GET / both `Attention Required!`); oracle concurs (`firewall: access denied`). IP-bound, not code. Watch only. |
| 3 | gmx | session | **DONE 2026-10-15** — hash page + desk JSON; rate-sensitive (oracle concurs). Shipped with status-gating + empty-body rules (below). |
| 4 | chatnoir | POST+cookie | **DONE 2026-10-15** — 3-leg chain (GET home → POST ?init → POST _search), Bearer+csrf, `SEARXNG_API_KEY_CHATNOIR` honored. quality 1/36 unresponsive. |
| 5 | dogpile | POST | **DONE 2026-10-15** — 2-leg POST (token refresh → /api/search JSON), live 2-8 results. |
| 6 | dictzone | plain HTML | **RE-SKIPPED 2026-10-15**: Cloudflare `Just a moment...` 403 from home egress; oracle returns 0 dictzone-attributed results too. Needs Translations/answers rendering anyway — revisit with translate vertical. |
| 7 | pypi | plain HTML | **RE-SKIPPED 2026-10-15**: `Client Challenge` anti-bot page (3KB shell) from home egress; oracle packages bench carries no pypi results either. Side-find: oracle packages bench is wider (voidlinux, pkg.go.dev, pub.dev, packagist, hex, hoogle, alpine, cachy) — future survey candidates. |
| 8 | openalex | keyless JSON | **DONE 2026-10-15** — replaces dead OpenAIRE; inverted-abstract rebuild; needed `body_cap_kb=300` (works ~250KB > 96KB default cap — truncated JSON parses to nothing). |
| 9 | rubygems | keyless JSON | **DONE 2026-10-15** — `/api/v1/search.json`, packages.html, live 7/7/6 round-robin. (Debug note: `[]` for non-matching queries is correct, not a bug.) |
| 10 | bpb | keyless JSON/HTML | **DONE 2026-10-15** — `/bpbapi/filter/search`, thumbnails + epoch→YYYY-MM-DD (civil-from-days, verified 1785344949→2026-07-29). |
| 11 | iseek | classify-first | **RE-SKIPPED 2026-10-15**: pure-calculation qToken (sha256→base64[0:4]) PROVEN IRRELEVANT — nginx plain `403 Forbidden` with a valid token (`n4bQ_1`); the wall is edge IP/UA filtering before any token check. Oracle: 0 iseek results. No SHA-256 code written (correctly — code can't fix a network wall). |
| 12 | swisscows | pure-math signing | **DONE 2026-10-15** — decision: PORT (400 proved the signature IS the gate, unlike iseek). `src/crypto.rs` (SHA-256+b64url, hashlib-verified vectors) + nonce/caesar/JWT-parse; web only. Live 2/hits, tags stripped via shared `engine::strip_tags`. |
| 13 | keenable | POST | **DONE 2026-10-15** — single POST to `/v1/search/public` (chain protocol already had POST from chatnoir/dogpile); env key upgrades to keyed `/v1/search`. Live 2 hits. |
| 14 | neosearch | session + POST | **DONE 2026-10-15** — 2-leg (GET home xsrf-meta → POST /search, last-line lenses JSON, 19 links live). QUEUE COMPLETE: 10 ported (boardreader, gmx*, chatnoir, dogpile, openalex, rubygems, bpb, swisscows, keenable, neosearch), 4 walled with evidence (fireball, dictzone, pypi, iseek). *gmx port-complete but live-walled (honest `empty response`). **P3-watch**: POST /search now 429s this egress IP (leg-1 home still 200; honest `http status: 429` surfaces it). Volume-triggered by survey bursts; recovery = time, not code. |

**Latency flag 2026-10-15 — RESOLVED same day**: 4 scoped lanes (GAP-PLAN P2) over contiguous engine slices, joined in cfg order. Worst general query 20.6s → 6.2s (per-engine threads hit 3.0s but cost +3MB RSS; 4 lanes cost +1.4MB: peak 16.3/avg 15.0 dev). Lessons: `?` never inside search-alternative loops; `move` closures need Copy captures (`qq`/`pageno` locals); `push(spawn(closure` closes `}))`. Next diet lever: drop `scraper` (only ddg/bingnews/ddgnews use it; 2 of 3 are challenge-walled).

Top of the old list (npm/crates/pubmed/ddgnews) is DONE — 21 native.
This queue takes us to ~30+ (minus drops), near the portable ceiling
from this network (blocked/dead/keyed correctly skipped).


## Challenged / rate-limited at IP level (verified live, ≤2 probes each)

| Engine | Signal | Verdict |
|---|---|---|
| DDG html + lite | `202 + anomaly.js?cc=botnet`, `challenge-form` | Challenged. Native `ddg` parses 0 + reports `unresponsive=[ddg, bot challenge]`. Fixture: `tests-fixture-ddg-anomaly.html`. |
| Mojeek | `200 + <title>Captcha</title>` | Challenged. Not ported. |
| Brave HTML | `429 Too Many Requests` on first touch | Rate-limited/gated. Not ported, no hammering. |
| Bing | Not probed (same class as DDG/Mojeek per PLAN §7; save live budget) | Assumed guarded, stays on strangler. |

## Needs signing / tokens / cookies (no budget-compatible path on phone)

| Engine | Signal | Verdict |
|---|---|---|
| swisscows | `400` without `X-Request-Nonce/Signature` (sha256-b64url + caesar-shift signing) | Skip. Hand-rolling signing ≈ 100 lines crypto-adjacent — rejected as non-simple. No fixture saved (400s carry no parse value). |
| fireball | `403` without settings cookie (+ mandatory Referer) | Skip. Cookie flow logged. |
| gmx | Token pre-fetch inside `request()` | Skip (multi-step session flow). |
| chatnoir / dogpile | Home/token pre-fetch flows | Skip (multi-step flows). |

## Needs credentials

| Engine | Verdict |
|---|---|
| marginalia, kagi, braveapi, exaapi | Skip — API keys. None exist on this box. |
| jina | API key (`jina_...`) — skip. |

## Challenge / token / POST gates (code-audited 2026-10-14, ≤1 live probe each)

| Engine | Signal | Verdict |
|---|---|---|
| startpage | Anubis PoW challenge (`_solve_anubis`) + `sc-code` token flow | Skip — challenge-solving class. |
| qwant | DataDome cookie (`datadome` in `CACHE`), API 404s without it | Skip — bot-protection class. |
| iseek | Per-query token (`_get_new_token`); probed 403 without it | Skip — token class, confirmed gated. |
| boardreader | Homepage-scraped `currentSessionId` pre-fetch | Skip — session flow. |
| neosearch | `X-XSRF-TOKEN` pre-fetch + POST | Skip — token flow + POST. |
| keenable | POST JSON (`/v1/search/public` keyless exists) | Deferred — native client is GET-only; POST wave later. |
| s1search | `base_url = ""` (dead) | Skip — dead. |
| bpb | Keyless JSON (`bpb.de/bpbapi`) but niche DE civic content | Skip — low general-web value. |
| 360search | `so.com`, CN-regional | Skip — region + likely gate. |
| currency_convert | Built on DDG api | Skip — challenged family. |

## Deferred to HTML wave (selectors + fixtures, after oracle exists)

dictzone, brave (HTML), 360search, bpb, mediawiki-generic (dup of wikipedia),
pypi (HTML scrape), gmx/fireball/chatnoir/dogpile (if flows ever simplified).

## Category waves (2026-10-14: it + science + news now native)

- `it` PORTED: stackoverflow/superuser/askubuntu (one parser),
  github, mdn, dockerhub, gentoo — oracle survey showed the whole
  bench green (85 results, 0 unresponsive).
- `science` PORTED: arxiv + europepmc + pubmed (new `follow_up`
  hook: esearch IDs → esummary bodies). Semantic scholar skipped
  twice over: website flow is POST + UI-version token AND the
  official API 429s this egress IP (probed + retried). Google
  scholar 429s on first touch. OpenAire dead upstream (oracle:
  unresponsive).
- `news` PORTED: bingnews (infinitescrollajax HTML, first HTML-wave
  engine) + wikinews (MediaWiki JSON) + ddgnews (same anomaly.js
  wall as ddg — reports unresponsive, auto-recovers with the IP).
- Still deferred: `packages` (npm, crates.io — keyless JSON, next
  wave), `map` (needs geo result keys — struct change).

## Live-query spend 2026-10-14 (fixture-first: ≤5/engine honored)

Step-0 walls 3 (DDG/Mojeek/Wikipedia) · fixtures 5 (ol 1, mwmbl 2 incl.
1 wrong-host 404, grokipedia 1, startpagina 1) · swisscows 2 (400s) ·
brave 1 (429) · fireball 1 (403) · gates via `check.py --limit 10`
(general/news/science/it via oracle + native). No ban observed; only
transient 429s (Wikipedia once under burst, Brave on first touch).

## Overnight session 2026-10-16 (DEV queue + survey P2 — all resolved)

Queue verdicts (all 9 closed: 7 ported, 2 proven-parked):
- packagist/maven/nuget/hexpm/pubdev/artifacthub/terraform/datatracker
  PORTED (keyless GET JSON; artifacthub kind→slug map; maven `G:A`
  coords; terraform `published_date`; datatracker Tastypie envelope
  + canonical `/doc/{name}/` + 400-char abstracts).
- docs.python.org PARKED: `searchindex.js` 4.1MB (10× class cap);
  flask 187KB + pip 226KB ported via new `src/sphinx.rs` class
  instead (token scoring terms+1/titleterms+2, title-substring
  fallback, int|array postings, tolerant envelope). django/fastapi
  parked (302→404 / 404, non-Sphinx).
- grep.app re-probe: STILL 429 Vercel checkpoint. P3-watch stands.
- cheat.sh PORTED (curl-UA text API + `?QT`, ANSI strip).
- Survey P2: tagesschau PORTED (512KB cap, detailsweb URLs),
  pkggodev PORTED (data-test-id hooks; 0.5s vs 8.8s cold shards →
  honest timeouts). metacpan 402×2 PARKED, lib_rs 403 PARKED.
- MCP: stdio (`--mcp`) + stateless `POST /mcp` DONE, E2E verified
  both transports; Inspector conformance still open.
- `engines=` filter fixed (intersect cfg natives; unknown → proxy).
- New-category items (jisho/open_meteo/sepiasearch) deferred to a
  dedicated session: each needs contract+conf+gate work; strangler
  proxies them meanwhile.

Probe spend: ~20 (all ≤2/engine) · gates: check.py 23/23 0/0.
Live `flask request context` 62 results · `router` (packages) 98 ·
`bundestag` (news) 24.

## 2026-10-16: reuters → SKIP (DataDome wall, 1 probe)
`www.reuters.com/pf/api/v3/content/fetch/articles-by-search-v2` (keyless
Arc JSON API) answers 401 + `var dd=` DataDome JS challenge to this
egress. Same revisit rule as other bot walls.
