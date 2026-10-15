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
| 10 | bpb | keyless JSON/HTML | Survey; niche DE civic content, value TBD — drop if thin. |
| 11 | iseek | classify-first | Token origin audit: calculator or session fetch, then port. |
| 12 | swisscows | pure-math signing | COMPLEXITY DECISION: ~100 lines crypto-adjacent (SHA-256 + b64url + Caesar). No new deps. Port iff P0–P11 stay green. |
| 13 | keenable | POST | Needs POST support in native client (one small feature). |
| 14 | neosearch | session + POST | LAST — needs POST support too. |

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
