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
| JS/TS | ✅ npm | ✅ MDN; 🔵 node `all.json` (static, cache pattern); 🔵 typescriptlang search | strong |
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
| Web platform | ✅ MDN | 🔵 caniuse static `data.json` (cache pattern) | — |
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

1. **Static-index + disk cache.** node `all.json`, brew `formula.json`,
   caniuse `data.json` are multi-MB single files — per-query fetch
   breaks body caps. `src/cache.rs` already exists: one engine class
   that refreshes weekly, searches locally. New pattern, one design
   pass before first use.
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

1. packagist (verified green — port next)
2. Maven Central + NuGet (one probe each to confirm, then port)
3. ArtifactHUB + Terraform registry (IaC is code)
4. hex.pm + pub.dev (cheap, same shape as packagist)
5. IETF datatracker API (RFC behavior questions)
6. docs.python.org search (biggest gap, hardest — Sphinx/JS)
7. grep.app re-probe (P3-watch, weekly)
8. Static-index pattern design → node all.json, caniuse
9. cheat.sh answers engine (fast win, anytime)
