# WEB-UI — search-engine console at `/` (plan, 2026-10-16)

Yes, buildable with **zero new crates, zero build step, zero JS
framework**: one HTML string served by `tiny_http` (like today's
`render::html`), vanilla `fetch()` for the try-it console, token in
`localStorage`. Server RAM line-item: one ~20KB transient string per
`GET /` — zero steady-state, JS runs client-side.

## The one structural change (auth chicken-and-egg)

Today the gate exempts only `/healthz`. A token-entry page that 401s
without a token is useless — so **`GET /` becomes unauthenticated**,
serving the static console. This leaks nothing: capabilities are
public facts, and the page carries no data until the browser calls
the (still-gated) APIs with a token. Changes:

- `server.rs` gate: exempt `/` alongside `/healthz` (log line stays
  for API 401s; page loads are not logged — avoid log spam).
- Server injects `AUTH_REQUIRED=true/false` into the page at serve
  time (it knows: tokens empty or not). The token box renders **iff**
  true — your swagger-like conditional, with no client guessing.
- CONTRACT.md + DEPLOY.md note: `/` and `/healthz` open, everything
  else gated. `/fetch` and `/mcp` behavior unchanged.

## Page sections (single file, three acts)

1. **Search app** (default view, proper-engine look): big centered
   box, category chips (from injected list), `engines=` optional,
   results render below as cards (title/url/content + thumbnail for
   videos, date when present). No-JS fallback stays: plain form
   `GET /search` → existing `html()` results, curl unaffected.
2. **Capabilities** (the swagger half): engine table grouped by
   category (name + transport + 1-line source note), route cards
   (`/search`, `/fetch`, `/mcp`, `/healthz`, `/opensearch.xml` with
   params), response top-keys from CONTRACT, MCP tool schemas.
3. **Try-it console**: tabs for Search / Fetch / MCP (`tools/list`,
   `tools/call search|fetch`): form → `fetch()` with the bearer
   header → pretty JSON + 401 hint that focuses the token box.

## No-drift rule (capabilities never rot)

The page embeds one server-generated JSON blob per request, built
from live structures — never hand-maintained:

- `engines:[{name,category}]` from the `engines::find` registry +
  `searxng.conf` order (same source as merge weights),
- `categories` from the server gate list,
- MCP `search`/`fetch` input schemas by calling the existing
  `mcp::search_tool()` / `fetch_tool()` fns (single source of truth),
- `authRequired`, `version` (Cargo `CARGO_PKG_VERSION`).

A `cargo test` asserts the blob: every conf engine present, every
category non-empty, schemas valid. If the registry changes and the
page doesn't follow, the test fails.

## Security notes

- Token: `localStorage` + "forget" button; never in URL (no history/
  log leak); never logged server-side (existing rule). XSS surface:
  remote titles/descriptions render through the existing `escape()`.
- Tunnel deployments unchanged: TLS at Cloudflare edge, bearer at
  origin; page works identically on localhost and public hostname.
- `auth=off` dev mode: `AUTH_REQUIRED=false`, token box hidden,
  calls go out bare — page visibly badges "DEV OPEN" so a tunnelled
  host can't mistake itself for secured.

## Build steps (est. 1 session)

1. Gate exemption + `AUTH_REQUIRED` + capability blob (`server.rs`,
   `render.rs::console()`), unit test on the blob.
2. Search view + result cards (reuse `escape()`, thumbnail/date
   slots for videos).
3. Capabilities + try-it tabs (vanilla JS, ~300 lines).
4. Live verify both modes (token shown/hidden, 401 path, no-JS
   form), `check.py` gate (new `/` must not break contract tests),
   `measure.sh` re-gate (string cost), commit + push.
