# Engine survey 2026-10-15 — keyless candidates beyond the 14-queue

> Mission split (2026-10-15): this file is the **upstream-parity backlog**.
> `DEV-SEARCH.md` is the **coding-agent mission backlog** (diverged
> prioritization). `SKIP-LOG.md` holds all verdicts.

Method: diffed upstream (252 engines) vs native roster (~33); pulled
`.py` sources for 30 keyless-declared candidates; live-probed 10 hosts,
1 query each (11 probes total: deezer×3, tagesschau×2 to nail shapes).
All `require_api_key: False` upstream. New hosts probed per user request.

## GREEN — verified live, port-ready (8)

| # | Engine | Cat (native) | Shape | Notes |
|---|---|---|---|---|
| 1 | crossref | science | GET `api.crossref.org/works?query=&rows=&offset=` → `message.items[]` (title/DOI/URL/author) | `status:ok` live; simplest port. POLICY OK: no robots.txt on API host; etiquette asks optional `mailto=` for the polite pool (no credential invented — descriptive UA only, 1 req/query, well within public-pool limits) |
| 2 | hackernews | it | GET `hn.algolia.com/api/v1/search?query=&page=` → `hits[]` (title/url/points) | 700k hits for "test"; trivial JSON. POLICY OK: public documented community endpoint, no key, no auth; upstream ships an engine against it; no prohibition found |
| 3 | archlinux | it | MediaWiki `api.php?action=query&list=search` | wikipedia-pattern clone. POLICY OK: robots.txt disallows `/index.php?` but NOT `api.php`. PORTED 2026-10-15 (opensearch tried first — title-prefix only, empty on normal queries; upstream scrapes the robots-disallowed index.php path, we use the clean API instead). body: full-text snippets, [[i18n]] links stripped |
| 4 | tagesschau | news | GET `www.tagesschau.de/api2u/search?searchText=&resultPage=` → `searchResults[25]` (title/date/content/teaserImage) | 477KB fat JSON → needs `body_cap_kb` (openalex pattern); URL assembly from sophoraId at port time |
| 5 | nvd | it | GET `services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch=&resultsPerPage=` → `vulnerabilities[]` (cve.id/descriptions) | keyless OK. POLICY OK: US federal public data; keyless tier (5 req/30s) comfortably fits 1 req/query sequential+SLEEP_S=2; use REST API, not upstream's nudp endpoint |
| 6 | jisho | dictionaries* | GET `jisho.org/api/v1/search/words?keyword=` → `data[20]` (slug/japanese/senses) | *new category → `answers[]` or results; needs gate+conf like packages wave |
| 7 | open_meteo | weather* | geo `geocoding-api…/v1/search?name=` → `results[0]` lat/lon, then `api…/v1/forecast?latitude=&longitude=&current=` → `current` block | two-step via `follow_up` (pubmed pattern); *new category → `answers[]` |
| 8 | sepiasearch | videos* | GET `sepiasearch.org/api/v1/search/videos?search=&start=&count=` → `data[]` (name/url/**embedUrl**) | `embedUrl` maps to existing `iframe_src` contract key; *new category needs gate+conf |

## DEGRADED (1)

- **deezer** (music): `total: 265/162` but `data: []` on every page shape
  tried (index=0, no-index, real query). API-side geo/restriction or
  regression — park, re-probe P3. Upstream file kept (`up-deezer.py` not
  saved; re-pull at re-probe).

## PARKED by shape, not by wall (4 → 2 left)

Resolved 2026-10-16 (overnight session):
- **tagesschau** ✅ PORTED — 477KB fat JSON → `body_cap_kb` 512
  (cap softened per session); `detailsweb` canonical URLs with
  `shareURL` fallback; `content[].value` join; ISO dates. Live 10.
- **pkg.go.dev** ✅ PORTED (`pkggodev`) — server-rendered
  `SearchSnippet` cards, `data-test-id` hooks; query-dependent
  latency (0.5s vs 8.8s cold shard) surfaces as honest timeout
  vs the 3s engine budget. Live 10. Parser lesson: never size a
  window from the *next* card (nested divs precede the hook) —
  cap forward from the hook instead (fixed a lane panic in review).
- **metacpan** PARKED (wall, not shape): `POST /v1/search` →
  `402` empty twice (GET `?q=` too). API shape changed or gated;
  re-probe P3, not code.
- **lib_rs** PARKED: `/search?q=` → `403` bot wall. Hand-scanner
  can't fix a wall; P3-watch.

Still parked:
- **metacpan** (packages/it): POST ES `_search` to `fastapi.metacpan.org`
  — chain/POST infra ready, template in upstream file. P2.
- **wikidata** (general): SPARQL POST with locale query-builder —
  complex, low result-value vs effort. Park.
- **google_news**: `google_request` helper = walled Google family. Skip.

## Proposed port order (simplest-first, existing categories first)

1. crossref → 2. hackernews → 3. archlinux → 4. nvd → 5. tagesschau ✅
6. metacpan (402-parked) → 7. lib_rs (403-parked) → 8. pkg_go_dev ✅
9. jisho → 10. open_meteo
11. sepiasearch (new categories need gate+contract+conf per wave)

Remaining (9–11) need NEW categories (dict/weather/video): contract
+ conf + gate work per wave — deferred to a dedicated session, not
this queue. Strangler proxies them meanwhile.

Traffic budget if approved: ≤2 probes/engine (fixture + live verify) +
gates, same protocol as the 14-queue.

## Sphinx-index survey (2026-10-16, 15 probes, 1 GET each)

Pattern: focused-library docs fit the class (index ≤400KB, cap already
512KB covers marginals); monorepo/data-science docs don't. Ports are
~15 min each (flask.rs clone: SITE const + fixture + gate).

| site | index size | verdict |
|---|---|---|
| requests.readthedocs.io | 87KB | GREEN — highest agent value (HTTP lib) |
| jinja.palletsprojects.com | 96KB | GREEN — pallets trio, flask shape |
| click.palletsprojects.com | 92KB | GREEN — pallets trio, flask shape |
| docs.aiohttp.org | 213KB | GREEN — async HTTP, high agent value |
| docs.pytest.org | 447KB | MARGINAL (+12% over 400KB guide, under 512 cap) |
| sphinx-doc.org | 493KB | MARGINAL, low value (meta docs) |
| pandas / numpy / matplotlib | 1.9–3.3MB | RED (fat) |
| sqlalchemy / celery | 1.0–1.4MB | RED (fat) |
| ansible | 25MB | RED (monorepo docs) |
| CPython | 4.1MB | RED (parked earlier) |
| pydantic / gunicorn | 404 | DEAD (mkdocs, no searchindex) |
| django / fastapi | 302 / 404 | DEAD (earlier) |

Recommended port order: requests → jinja → click → aiohttp (all GREEN,
all `it`), pytest iff the marginal is accepted. Rule of thumb: if the
library's whole docs fit in one narrative, its index fits the class.
