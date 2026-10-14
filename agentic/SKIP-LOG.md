# SKIP LOG — engines probed from home egress and deferred (2026-10-14)

Rule: anything needing keys, JS rendering, CAPTCHA solving, or TLS
impersonation = skip + log (AGENT-MISSION.md stop triggers). Guarded
engines stay behind the strangler (`proxy_to` → Python `:8889`).

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
| marginalia, kagi, braveapi | Skip — API keys. None exist on this box. |

## Deferred to HTML wave (selectors + fixtures, after oracle exists)

dictzone, brave (HTML), 360search, bpb, mediawiki-generic (dup of wikipedia),
pypi (HTML scrape), gmx/fireball/chatnoir/dogpile (if flows ever simplified).

## Deferred to category wave (need `proxy_to` oracle or category allowlist)

- `it/packages`: npm (JSON), crates.io (JSON), docker_hub — keyless and
  parse-simple, but hot path is general-only per PLAN §1 MVP.
- `science`: arxiv (keyless API) — same reason.
- `map`: openstreetmap/nominatim — same reason.

## Live-query spend 2026-10-14 (fixture-first: ≤5/engine honored)

Step-0 walls 3 (DDG/Mojeek/Wikipedia) · fixtures 5 (ol 1, mwmbl 2 incl.
1 wrong-host 404, grokipedia 1, startpagina 1) · swisscows 2 (400s) ·
brave 1 (429) · fireball 1 (403) · gates via `check.py --limit 10`
(general-only; news/science need the oracle). No ban observed; only
transient 429s (Wikipedia once under burst, Brave on first touch).
