#!/usr/bin/env python3
"""Differential check: Python reference vs native, same query. Stdlib only.

Usage: python3 diff/diff.py --q "query" [--ref ...] [--new ...]
Compares top-level keys and per-result key sets; reports drift and the
share of result URLs present in both (overlap). Scoring/ranking diff is
informational only — order may legitimately differ during the strangler phase.
"""
import argparse
import json
import sys
import urllib.parse
import urllib.request


def search(base, q):
    url = base.rstrip("/") + "/search?" + urllib.parse.urlencode(
        {"q": q, "format": "json", "pageno": 1, "language": "en"})
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.loads(r.read().decode("utf-8"))


def norm(u):
    return (u or "").lower().rstrip("/")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--q", required=True)
    ap.add_argument("--ref", default="http://127.0.0.1:8889")
    ap.add_argument("--new", default="http://127.0.0.1:8888")
    args = ap.parse_args()

    ref = search(args.ref, args.q)
    new = search(args.new, args.q)

    drift = []
    if set(ref) != set(new):
        drift.append(f"top-level keys: ref={sorted(ref)} new={sorted(new)}")
    rr, nr = ref.get("results", []), new.get("results", [])
    for i in range(min(len(rr), len(nr))):
        if set(rr[i]) != set(nr[i]):
            drift.append(f"result[{i}] keys: ref-only={sorted(set(rr[i]) - set(nr[i]))} "
                         f"new-only={sorted(set(nr[i]) - set(rr[i]))}")
            break
    r_urls = {norm(r.get("url")) for r in rr}
    n_urls = {norm(r.get("url")) for r in nr}
    overlap = len(r_urls & n_urls)
    print(f"ref={len(rr)} results new={len(nr)} results overlap={overlap} urls")
    print(f"ref unresponsive={ref.get('unresponsive_engines')} "
          f"new unresponsive={new.get('unresponsive_engines')}")
    for d in drift:
        print("DRIFT:", d)
    print("OK - contract keys match" if not drift else "DRIFT FOUND")
    return 1 if drift else 0


if __name__ == "__main__":
    sys.exit(main())
