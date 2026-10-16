#!/usr/bin/env python3
"""Contract + health check for native-searxng. Stdlib only.

Usage: python3 evals/check.py --base http://127.0.0.1:8888 [--limit 0]
Exit 0 = contract holds; 1 = hard failure (HTTP error / missing keys).
Empty result lists are warnings (engines may be banned), not failures.
"""
import argparse
import json
import os
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent


def auth_headers():
    """Bearer header from SEARXNG_API_TOKEN (first entry); {} when unset.
    Unset = server runs dev-open (auth=off) or refuses to start."""
    tok = os.environ.get("SEARXNG_API_TOKEN", "").split(",")[0].strip()
    return {"Authorization": f"Bearer {tok}"} if tok else {}

HERE = Path(__file__).resolve().parent

TOP_KEYS = {"query", "results", "answers", "corrections",
            "infoboxes", "suggestions", "unresponsive_engines"}
RESULT_KEYS = {"url", "title", "content", "engine", "engines",
               "publishedDate", "positions", "score", "category"}


def fetch(base, q, categories, language):
    params = {"q": q, "format": "json", "pageno": 1, "language": language}
    if categories and categories != "general":
        params["categories"] = categories
    url = base.rstrip("/") + "/search?" + urllib.parse.urlencode(params)
    t0 = time.monotonic()
    with urllib.request.urlopen(urllib.request.Request(url, headers=auth_headers()), timeout=30) as r:
        body = r.read().decode("utf-8")
    dt = time.monotonic() - t0
    return json.loads(body), dt


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://127.0.0.1:8888")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--language", default="en")
    args = ap.parse_args()

    with open(HERE / "queries.json", encoding="utf-8") as f:
        queries = json.load(f)["queries"]
    if args.limit:
        queries = queries[: args.limit]

    failures, warnings = [], []
    for item in queries:
        q = item["q"]
        try:
            data, dt = fetch(args.base, q, item.get("categories", "general"), args.language)
        except Exception as e:  # noqa: BLE001 - report, don't crash the run
            failures.append(f"{q!r}: fetch failed: {e}")
            continue
        missing = TOP_KEYS - set(data)
        if missing:
            failures.append(f"{q!r}: missing top keys {sorted(missing)}")
            continue
        results = data.get("results", [])
        print(f"[{dt:5.1f}s] {len(results):3d} results  {q}")
        if not results:
            warnings.append(f"{q!r}: zero results (unresponsive: {data.get('unresponsive_engines')})")
            continue
        for i, r in enumerate(results):
            miss = RESULT_KEYS - set(r)
            if miss:
                failures.append(f"{q!r} result[{i}]: missing keys {sorted(miss)}")
                break
            if not r.get("url"):
                warnings.append(f"{q!r} result[{i}]: empty url")
                break

    print(f"--- {len(queries)} queries, {len(failures)} failures, {len(warnings)} warnings")
    for w in warnings:
        print("WARN:", w)
    for fl in failures:
        print("FAIL:", fl)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
