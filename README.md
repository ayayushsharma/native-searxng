# native-searxng — SearXNG metasearch core in sync-only Rust

Clean-room minimal core: SearXNG-compatible `/search?format=json`,
no Python, no async runtime, one static binary (~3 MB release,
dev RSS ~13 MB, release target <10 MB). **53 native engines** across
8 categories (`general it science news packages videos dictionaries
weather`); anything unknown proxies to the Python reference
(strangler) or 503s when no proxy is configured.

## Quickstart (localhost)

```sh
mise run debug            # build + serve on :8888, prints a throwaway token
curl -H "Authorization: Bearer <token>" \
  "http://127.0.0.1:8888/search?q=flask+request+context&format=json&categories=it" \
  | python3 -m json.tool
```

Open `http://127.0.0.1:8888/` in a browser for the console (search,
engine table, `/fetch` + MCP try-it). The console page itself is
unauthenticated; data calls send the bearer from the token box.

## Auth (fail-closed)

All routes except `/healthz` require a bearer token:

```sh
export SEARXNG_API_TOKEN="<secret>"   # comma-separated list = rotation
```

Unset token → the server **refuses to start**, unless `searxng.toml`
says `require_auth = false` (local-dev hatch only, never tunnelled).
MCP over stdio needs no token (OS trust boundary); MCP over HTTP does.
Eval scripts read the same env var — export before gating.

## Endpoints

| Method | Path | Shape |
|---|---|---|
| GET | `/search?q=&format=json&categories=&engines=&pageno=` | SearXNG JSON (`results answers corrections infoboxes suggestions unresponsive_engines`) |
| GET | `/search?q=` (no format) | HTML page |
| GET | `/fetch?url=<http(s)://…>` | `{url,title,content,links}`; walled pages → 502, never bodies in errors |
| POST | `/mcp` | Stateless Streamable MCP (tools: `search`, `fetch`), bearer-gated |
| GET | `/ /app.js /style.css /api/config /healthz` | Console statics, live engine manifest, liveness |

`./target/debug/native-searxng --mcp` speaks MCP on stdio (stdout stays
pure JSON-RPC — logs go to stderr, always). `engines=` intersects the
configured natives; unknown names fall through to the proxy.

Pi agent integration (global stdio server, no token, no daemon):
`pi mcp add searxng --env SEARXNG_CONF=<repo>/searxng.toml -- \
<repo>/target/debug/native-searxng --mcp` — then `search`/`fetch`
are agent tools (`pi -p` verified).

## Config (`searxng.toml`, XDG-aware)

Resolution: `SEARXNG_CONF` > `$XDG_CONFIG_HOME/searxng.toml` >
`~/.config/searxng.toml` > `./searxng.toml`. Sections:

- `[server]` `port bind require_auth` — loopback bind; cloudflared tunnel is the sole client publicly.
- `[http]` `timeout_ms proxy_timeout_ms body_cap_kb language user_agent ca_file` — `ca_file` = corp-CA PEM for TLS-intercepting work networks (replaces bundled roots; bad file refuses to start).
- `[proxy]` `url` — Python reference for unknown categories/engines; empty disables.
- `[perf]` `lanes` — `0` = auto (`min(cpus,4,engines)`); `64` = fully parallel (beefy); `1-2` = small machines.
- `[logging]` `level` — `off|error|warn|info|debug` (stderr; queries never logged; MCP-safe).
- `[engines]` per-category arrays — **file order = merge weight**; unknown categories proxy.

Secrets never live here (`SEARXNG_API_TOKEN`, `SEARXNG_API_KEY_<ENGINE>` env-only).

## mise tasks (Termux/macOS/Linux, `JOBS=`/`BASE=`/`REF=`/`BIN=`/`TARGET=` overrides)

| Task | What |
|---|---|
| `check` / `build` / `test` | Iterate (phone: `JOBS=4`/`2`/`4`), dev binary, offline suite |
| `fmt` / `fmt-check` | `cargo fmt --all` / `--check` (rustfmt only — safe anytime, no lock) |
| `debug` (= `debug-mac`) | Build + serve foreground, throwaway token, paste-ready curls |
| `build-release` `build-linux` `build-android` `build-mac` | Release binaries — **CI owns these; phone never runs them** |
| `run` / `mcp` | Serve / stdio-MCP the dev binary |
| `gate` / `conform` / `exam` / `diff` | Contract 26q / MCP 20-20 / agent 20q / native-vs-oracle diff |
| `measure` | Server RSS under load |

One cargo at a time (`tools/cargo-run.sh` lock); server STOPPED during
builds; `target/` is never deleted; `build.log` stays until root-caused.

## Docs

- `PLAN.md` — embedded budget, queues, RAM line-items (§3), auth (§14)
- `AGENTS.md` — phone rules, CI frugality (check-only per push, one release job on `v*`)
- `agentic/CONTRACT.md` — byte-exact JSON keys
- `agentic/DEPLOY.md` — cloudflared + rotation + work-machine TLS
- `agentic/DEV-SEARCH.md` — coding-agent mission backlog (why these engines)
- `agentic/ENGINE-SURVEY.md` + `SKIP-LOG.md` — parity backlog and wall verdicts
- `agentic/WEB-UI.md` — console design; `agentic/GAP-PLAN.md` — latency/count plan

## Discipline (binding)

Sync only. No new crate without a RAM line-item in `PLAN.md` §3.
Fixture-first (≤5 live queries/engine, then offline). Every engine gated
by `check.py` + one commit per change. Live harnesses never run in CI.
