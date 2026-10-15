# DEV-SEARCH — coding-agent search strategy

> 2026-10-15. **Divergence note:** engine selection is now driven by
> *coding-agent value*, not SearXNG upstream parity. `ENGINE-SURVEY.md`
> remains the parity backlog; this file is the mission backlog.
> `SKIP-LOG.md` keeps every verdict (port / park / watch).

## Why agents need different engines

A browser user browses; a coding agent resolves **named things**:
a stdlib symbol, a package version, an error string, a CVE id, a config
key. That means: structured JSON APIs over official doc sets beat
general web search for 8 of the 10 recurring usecases below.
General engines stay as the fallback layer, not the answer layer.

## Agent usecase → source map

| # | Agent usecase | Best sources | Status |
|---|---|---|---|
| 1 | stdlib symbol lookup (signature, semantics) | MDN, cppreference, docs.python.org, pkg.go.dev, docs.rs | MDN ✅; cppreference ❌ (see below); rest to-probe/queued |
| 2 | package eval (pick crate/npm/gem, check downloads) | npm, crates, rubygems, packagist, PyPI | npm/crates/rubygems ✅; packagist 🟢 verified (queue #1); PyPI ❌ |
| 3 | error-string diagnosis | StackOverflow, SE sites, HN, dev.to | ✅ ✅ ✅ (dev.to to-probe) |
| 4 | "how do I X" recipes | ArchWiki, MDN, dev.to, SE | Arch ✅ MDN ✅ |
| 5 | code examples in the wild | GitHub (repo-level ✅), grep.app (code-level ❌ walled) | repo ✅ code ❌ |
| 6 | CVE / vuln check by id or product | NVD | ✅ |
| 7 | cloud/infra config reference | MS Learn, AWS docs, GCP docs, Terraform registry, ArtifactHUB | MS Learn ✅; AWS/GCP ❌ (JS shells); TF/AHUB to-probe |
| 8 | paper/algorithm background | arXiv, Crossref, OpenAlex | ✅ ✅ ✅ |
| 9 | CLI flag lookup | cheat.sh (answers-shaped), explainshell, man pages | to-probe |
| 10 | standards (RFC behavior, HTTP semantics) | IETF datatracker API | to-probe (high confidence) |

## Per-ecosystem matrix

`✅ live · 🟢 verified-green (to port) · 🟡 queued · 🔵 to-probe · ❌ parked`

| Ecosystem | Registry/meta | Stdlib/API docs | Notes |
|---|---|---|---|
| Rust | ✅ crates | 🔵 docs.rs search; static rustdoc searchindex (cache pattern) | core covered |
| Python | ❌ PyPI search walled (per-package `/pypi/<pkg>/json` works, no search API) | 🔵 docs.python.org search | **gap**: biggest agent ecosystem, weakest coverage |
| JS/TS | ✅ npm | ✅ MDN; 🔵 TypeScript docs search; node `all.json` dropped (9.2MB — un-fetchable live, un-cacheable per invariant; LSP covers signatures) | strong |
| Go | 🟡 pkg.go.dev (survey P2) | pkg.go.dev/std (same port) | queued |
| Java/Kotlin | 🔵 Maven Central `search.maven.org/solrsearch/select` (keyless Solr, high confidence) | — | high-value probe |
| .NET | 🔵 NuGet `azuresearch-usnc.nuget.org/query` (keyless, high confidence) | ✅ MS Learn | Learn covers docs; registry to-probe |
| PHP | 🟢 packagist `search.json` (verified 200 2026-10-15: `results[{name,description,url,repository,downloads,favers}]`) | 🔵 php.net search | port next |
| Ruby | ✅ rubygems | 🔵 ruby-doc stdlib | covered |
| Elixir / Dart | 🔵 hex.pm `/api/packages?search=` / pub.dev `/api/search` (both keyless, good confidence) | — | cheap probes |
| Perl / R / Haskell / Swift | 🟡 metacpan (P2) / 🔵 crandb / hackage / Swift Package Index (lower confidence) | — | after the big six |
| C/C++ | 🔵 conan/vcpkg (low) | ❌ cppreference (evidence below) | **gap**: no good stdlib source right now |
| IaC | 🔵 Terraform `registry.terraform.io/v1/modules/search` + ArtifactHUB `/api/v1/packages/search` (both keyless, good confidence) | 🔵 K8s docs (Algolia — policy-gray, see §4) | high agent value (configs are code) |
| Linux/CLI | ✅ arch, gentoo | 🔵 cheat.sh `cheat.sh/<cmd>?Q` (plain-text → answers engine), explainshell, man7 | cheat.sh is a ~30-line port |
| Web platform | ✅ MDN | 🔵 caniuse only via live paths (per-feature scrape or MDN compat tables — no snapshot, see pattern #1) | — |
| Prose tutorials | — | 🔵 dev.to API | after references work |

## Evidence log (2026-10-15, 5 probes)

- **packagist** 🟢 `GET /search.json?q=test` → 200, 4KB, clean
  `results[]`. No policy file conflicts (public site search). PORT NEXT.
- **cppreference** ❌ `api.php?action=query&list=search` → 200 with
  `{"error":{"code":"internal_api_error_Error"...}}` (their search
  backend broken); `action=opensearch` → Cloudflare `Just a moment...`
  (our own wall marker, eating itself). No robots.txt exists (serves a
  wiki page). PARKED. C/C++ stdlib has no keyless source in hand —
  alternatives: DevDocs offline bundles (user-side), Compiler Explorer
  API (execution, not docs — different engine class, later).
- **grep.app** ❌ `GET /api/search?q=test` → 429 `Vercel Security
  Checkpoint` (JS challenge, this egress). Normally an open API —
  P3-WATCH (recovery = time/IP, not code). Code-level search stays
  repo-level (GitHub) until it lifts.
- **GCP/AWS** ❌ (from cloud-docs survey): 688KB / 4KB JS shells.
- **MS Learn** ✅ ported (`microsoftlearn`, 10 hits live).

## New engine patterns (beyond one-URL-one-parser)

> **Statelessness invariant (2026-10-15):** an engine is a pure function
> of (query, page): fetch live, parse, drop. Anything requiring refresh
> schedules, version pinning, or schema migration is out of scope —
> that's a data product, not a search engine.

1. **Static-index + disk cache — KILLED (deferred).** node `all.json`
   (measured 9.2MB 2026-10-15, not 2–4MB), brew `formula.json`,
   caniuse `data.json` (measured 4.8MB, not ~2MB) are multi-MB single
   files: un-fetchable per query (96× body cap) and un-cacheable under
   the invariant (invalidation × refresh × schema drift × N languages
   = a second product). Resident parse would also blow the 4MB RAM
   target (~2–3× file size on heap). The data stays documented here;
   revisit only if a concrete usecase arrives that live sources cannot
   serve (e.g. fully-offline agent). `src/cache.rs` stays for HTTP
   TTLs, not language datasets. Note: LSP already covers node
   signatures project-locally (see discussion 2026-10-15) — `all.json`
   demoted regardless.
2. **Sphinx `searchindex.js` class.** Django/Flask/FastAPI/pip docs all
   ship a static search index. One fetcher + per-site table = many doc
   sets for one engine. Config-heavy — after single-site wins.
3. **Answers-shaped text APIs.** cheat.sh returns plain text for
   `cheat.sh/<cmd>` — maps to `answers[]`, not `results[]`. Tiny port,
   high agent value (no parsing ambiguity for the consumer).

## Policy rules (unchanged, extended)

- Fixture-first stays (≤2 probes per candidate; this survey used 5
  total across 3 candidates + 2 confirmations).
- **Frontend-key rule (new):** Algolia DocSearch keys embedded in docs
  sites (K8s, TS docs) are *their* frontend keys — do not lift them
  into our client. Use only documented public APIs or site search
  endpoints meant for direct calls (MS Learn, Maven Solr).
- Walls get P3-watch entries, not retries (grep.app model).

## Priority queue (coding-agent order)

> MCP server — DONE 2026-10-16 (overnight): `src/mcp.rs`, 2 tools
> (`search`, `fetch`), stdio (`--mcp`) + stateless Streamable HTTP
> (`POST /mcp`, shared `dispatch()` seam, 64KB POST cap, no SSE/sessions),
> zero new crates, pinned `2025-06-18`. E2E verified over both
> transports (pipe + curl). Inspector conformance still open before
> claiming pi compatibility. Replaces the bespoke pi TS adapter.

1. packagist ✅ PORTED (overnight)
2. Maven Central ✅ + NuGet ✅ PORTED (overnight, 1 probe each)
3. ArtifactHUB ✅ + Terraform registry ✅ PORTED (overnight)
4. hex.pm ✅ + pub.dev ✅ PORTED (overnight)
5. IETF datatracker ✅ PORTED (overnight; keyless 5 req/30s —
   honest `unresponsive`, recovers after wait)
6. docs.python.org search ❌ PARKED (overnight: `searchindex.js` is
   4.1MB `Search.setIndex`, 10× the class cap — no stored state per
   the statelessness invariant; PyPI-per-package + SO cover Python)
7. grep.app ❌ still walled (overnight re-probe: 429 Vercel
   checkpoint again — P3-watch stands)
8. ~~Static-index pattern design → node all.json, caniuse~~ (killed — see pattern #1; revisit on a concrete offline usecase)
9. cheat.sh ✅ PORTED (overnight: curl-UA text API, `?QT`, ANSI
   strip, `Unknown topic.` → `[]`)

Bonus overnight: Sphinx-index class (`src/sphinx.rs`, flask + pipdocs
live) — pattern #2 proved on two rows; django 302→404 + fastapi 404
(non-Sphinx) parked. `engines=` filter fixed (intersect cfg natives,
unknown → proxy). tagesschau + pkggodev (survey P2) also ported.
