# DEPLOY — public hosting via cloudflared (single consumer)

Single user, multiple devices. Server binds loopback; the *only* way in
is the tunnel; the tunnel's callers still need the app token.

## Origin (your box)

`searxng.toml` (relevant lines — auth stays ON, no `require_auth = false` here):

```toml
bind = "127.0.0.1"
port = 8888
```

Environment (supervisor/runit/Termux:Boot/systemd — never the conf file,
never the repo):

```sh
export SEARXNG_API_TOKEN="<random-32+-chars>"
```

Generate: `python3 -c "import secrets; print(secrets.token_urlsafe(32))"`.
Same token on every device you query from (header below).

Start: `SEARXNG_CONF=… ./native-searxng`. Expect on stderr:

```text
native-searxng on 127.0.0.1:8888 (auth on)
```

No token + no `auth=off` → clean refusal (exit 1), not an open server.
`auth=off` exists for **local dev only** — greppable, never on this host.

## Tunnel (cloudflared, same box)

```sh
cloudflared tunnel --url http://127.0.0.1:8888
# or a named tunnel with ingress: service: http://127.0.0.1:8888
```

TLS terminates at the Cloudflare edge; tunnel-to-origin is encrypted.
Optional free extras, no code changes: WAF managed rules, rate limiting.

## Clients (your devices)

Every request carries the token (only `/healthz` is exempt):

```sh
curl -H "Authorization: Bearer $SEARXNG_API_TOKEN" \
  "https://<your-host>/search?q=flask+request+context&format=json&categories=it"
```

- pi MCP client / Inspector: set the `Authorization` header
  (`Bearer <token>`) on the `POST /mcp` endpoint / MCP server config.
- Failures are `401 {"error":"unauthorized"}` (+ `WWW-Authenticate:
  Bearer`). The server log records source (`cf:<ip>` via
  `CF-Connecting-IP`, trustworthy because loopback guarantees the
  tunnel is the sole client) + path — never the credential.

## Rotation (no downtime)

`SEARXNG_API_TOKEN` accepts a comma-separated list; any entry is valid:

1. Set `newtoken,oldtoken`, restart origin.
2. Update each device to `newtoken` at leisure.
3. Set `newtoken` alone, restart. `oldtoken` is dead.

## Local evals (on the box)

`check.py` / `quality.py` / `latency.py` / `diff.py` read the same env
var and attach the header automatically — export it before gating:

```sh
export SEARXNG_API_TOKEN="<token>"
python3 agentic/evals/check.py --base http://127.0.0.1:8888
```

## Work machine behind a TLS-intercepting proxy

Symptom: every engine fails with `invalid peer certificate` (the binary
trusts Mozilla's bundled roots, not your OS store, so the proxy's
re-signed certificates don't verify). Fix — point the server at your
corp CA file; nothing else changes:

```toml
[http]
ca_file = "/path/to/corp-ca.pem"
```

Rules: the file REPLACES the bundled roots (on an intercepting network
the proxy re-signs everything, so the corp CA is the only root needed);
unset it when off the corp network. Missing/unparseable file = refuse
to start (loud, never silent fallback). Startup logs `custom CA active`
as a reminder. Get the file from IT (or export it from the OS store
where the browser finds it). Never `disable_verification` — that turns
TLS into theater for all traffic, not just yours.
