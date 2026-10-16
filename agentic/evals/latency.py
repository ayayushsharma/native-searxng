#!/usr/bin/env python3
"""Dedicated latency probe: same 8 queries against native :8888 and
oracle :8889, sequential, 2s politeness sleep. Reports per-query +
mean/p50/max per side. Local-only, never in CI."""
import json
import os
import time
import urllib.parse
import urllib.request

QUERIES = [
    ("test", "general"),
    ("rust async runtime comparison", "general"),
    ("python requests", "it"),
    ("docker compose", "general"),
    ("crispr off-target", "science"),
    ("central bank rate", "news"),
    ("serde json", "packages"),
    ("olympics host city", "news"),
]
SIDES = {"nat": "http://127.0.0.1:8888", "ref": "http://127.0.0.1:8889"}
SLEEP_S = 2


def timed(base, q, cat):
    url = f"{base}/search?q={urllib.parse.quote(q)}&format=json&categories={cat}"
    tok = os.environ.get("SEARXNG_API_TOKEN", "").split(",")[0].strip()
    hdrs = {"Authorization": f"Bearer {tok}"} if tok else {}
    t0 = time.time()
    with urllib.request.urlopen(urllib.request.Request(url, headers=hdrs), timeout=60) as r:
        d = json.load(r)
    dt = time.time() - t0
    return dt, len(d["results"]), len(d["unresponsive_engines"])


rows = []
for q, cat in QUERIES:
    row = {"q": q, "cat": cat}
    for side, base in SIDES.items():
        try:
            dt, n, u = timed(base, q, cat)
            row[side] = f"{dt:5.1f}s n={n:2d} u={u}"
            row[side + "_t"] = dt
        except Exception as e:  # noqa: BLE001 — report, don't stop
            row[side] = f"FAIL {type(e).__name__}"
            row[side + "_t"] = None
        time.sleep(SLEEP_S)
    rows.append(row)
    print(f"{cat:8s} {q[:32]:32s} nat={row['nat']}  ref={row['ref']}")

for side in SIDES:
    ts = sorted(r[side + "_t"] for r in rows if r[side + "_t"] is not None)
    if ts:
        mean = sum(ts) / len(ts)
        p50 = ts[len(ts) // 2]
        print(f"{side}: n={len(ts)} mean={mean:.1f}s p50={p50:.1f}s max={max(ts):.1f}s")
