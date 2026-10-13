# agentic surface — agent-facing contract + verification (inside native-searxng)

Thin contract + verification layer over SearXNG. There is **no new API**:
the contract IS stock SearXNG's `/search&format=json` (see `CONTRACT.md`,
mirroring `../PLAN.md` §1). Any existing SearXNG JSON client
works unmodified against the native implementation.

## Layout

- `CONTRACT.md` — frozen request/response shape (params, top-level keys,
  result keys, error shape, example). Change requires a PLAN.md amendment.
- `evals/queries.json` — starter query set for regression + recall checks.
- `evals/check.py` — stdlib-only: hits native, asserts contract keys, field
  presence, records latency. Non-zero exit on hard failure.
- `diff/diff.py` — stdlib-only: same query → Python reference vs native,
  reports key-set drift and engine overlap per result.
- `clients/` — minimal `curl` + Python examples (copy-paste integration).

## Endpoints (strangler)

- Native (hot path): `http://127.0.0.1:8888`
- Python reference (fallback/oracle): `http://127.0.0.1:8889`
- Agents use ONE base URL (native). Unknown categories/engines proxy to
  Python transparently — coverage is complete from week one.

## Workflow

1. Add/port engine in `../src/engines/` (see `../PLAN.md` §12).
2. `python3 evals/check.py --base http://127.0.0.1:8888` — contract holds.
3. `python3 diff/diff.py --q "query" --ref http://127.0.0.1:8889 --new http://127.0.0.1:8888`
   — no key drift vs Python.
4. Extend `evals/queries.json` when a new category/vertical lands.
