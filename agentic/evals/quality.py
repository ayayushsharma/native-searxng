#!/usr/bin/env python3
"""Large quality comparison: N queries x 5 categories against BOTH the
native server (:8888) and the Python oracle (:8889).

Metrics per query: result counts, engine mix, URL overlap (normalized),
unresponsive_engines, elapsed. Summary aggregates by category.

LIVE traffic (fans out to upstreams) — local runs only, NEVER in CI.
Be polite: SLEEP_S between queries, sequential, one pass.
"""
import json
import os
import sys
import time
import urllib.parse
import urllib.request
from collections import Counter, defaultdict


def auth_headers():
    tok = os.environ.get("SEARXNG_API_TOKEN", "").split(",")[0].strip()
    return {"Authorization": f"Bearer {tok}"} if tok else {}

NATIVE = "http://127.0.0.1:8888"
ORACLE = "http://127.0.0.1:8889"
SLEEP_S = 2

QUERIES = [
    # general x14
    ("rust async runtime comparison", "general"),
    ("exynos 850 specifications", "general"),
    ("searxng json api format", "general"),
    ("openwrt musl static binary size", "general"),
    ("duckduckgo html endpoint parameters", "general"),
    ("termux runit termux-services sv", "general"),
    ("mojeek search api", "general"),
    ("wreq rust crate impersonate", "general"),
    ("encoding_rs whatwg gbk", "general"),
    ("jiff rust date parsing", "general"),
    ("sqlite vs postgres embedded", "general"),
    ("wireguard vs tailscale", "general"),
    ("qemu user mode static", "general"),
    ("restic backup s3", "general"),
    # it x6
    ("python requests connection timeout", "it"),
    ("docker compose restart policy", "it"),
    ("rust anyhow vs thiserror", "it"),
    ("typescript strict mode", "it"),
    ("kubernetes liveness probe", "it"),
    ("git rebase interactive", "it"),
    # science x6
    ("transformer architecture paper", "science"),
    ("crispr off-target detection", "science"),
    ("mrna vaccine mechanism", "science"),
    ("fusion ignition nif", "science"),
    ("james webb first deep field", "science"),
    ("antibiotic resistance review", "science"),
    # news x6
    ("bilateral trade agreement 2026", "news"),
    ("central bank rate decision", "news"),
    ("election results 2026", "news"),
    ("olympics host city", "news"),
    ("ceasefire negotiations", "news"),
    ("earthquake relief fund", "news"),
    # packages x4
    ("serde json", "packages"),
    ("tokio async", "packages"),
    ("react hooks", "packages"),
    ("ripgrep", "packages"),
]


def fetch(base, q, cat):
    params = urllib.parse.urlencode({"q": q, "format": "json", "categories": cat})
    t0 = time.time()
    try:
        with urllib.request.urlopen(urllib.request.Request(f"{base}/search?{params}", headers=auth_headers()), timeout=120) as r:
            d = json.load(r)
        return d, time.time() - t0, None
    except Exception as e:  # noqa: BLE001
        return None, time.time() - t0, f"{type(e).__name__}: {e}"


def norm(u):
    u = u.lower().split("#")[0].rstrip("/")
    for p in ("https://", "http://"):
        if u.startswith(p):
            u = u[len(p):]
            break
    if u.startswith("www."):
        u = u[4:]
    return u


def main():
    agg = defaultdict(lambda: {"n": 0, "nat_n": 0, "ref_n": 0, "overlap": 0,
                               "nat_t": 0.0, "ref_t": 0.0,
                               "nat_un": Counter(), "ref_un": Counter(),
                               "eng_hit": Counter(), "eng_seen": Counter()})
    fails = []
    for i, (q, cat) in enumerate(QUERIES):
        nd, nt, nerr = fetch(NATIVE, q, cat)
        time.sleep(1)
        rd, rt, rerr = fetch(ORACLE, q, cat)
        a = agg[cat]
        a["n"] += 1
        if nerr or rerr:
            fails.append((q, nerr, rerr))
            print(f"[{i+1:2d}/{len(QUERIES)}] {cat:8s} FAIL native={nerr} ref={rerr}", flush=True)
            time.sleep(SLEEP_S)
            continue
        n_urls = {norm(r["url"]) for r in nd["results"] if r.get("url")}
        r_urls = {norm(r["url"]) for r in rd["results"] if r.get("url")}
        ov = len(n_urls & r_urls)
        # Per-engine overlap: which natives contributed ref-matching URLs.
        url_eng = {}
        for r in nd["results"]:
            if r.get("url"):
                url_eng.setdefault(norm(r["url"]), []).extend(r.get("engines", []))
        for u in n_urls & r_urls:
            for e in set(url_eng.get(u, [])):
                a["eng_hit"][e] += 1
        for r in nd["results"]:
            for e in set(r.get("engines", [])):
                a["eng_seen"][e] += 1
        a["nat_n"] += len(nd["results"])
        a["ref_n"] += len(rd["results"])
        a["overlap"] += ov
        a["nat_t"] += nt
        a["ref_t"] += rt
        for e, _ in nd.get("unresponsive_engines", []):
            a["nat_un"][e] += 1
        for e, _ in rd.get("unresponsive_engines", []):
            a["ref_un"][e] += 1
        print(f"[{i+1:2d}/{len(QUERIES)}] {cat:8s} nat={len(nd['results']):2d} "
              f"ref={len(rd['results']):2d} overlap={ov:2d} "
              f"{nt:4.1f}s/{rt:4.1f}s  {q[:38]}", flush=True)
        time.sleep(SLEEP_S)
    print("\n=== by category (avg per query) ===")
    tot = {"n": 0, "nat_n": 0, "ref_n": 0, "overlap": 0, "nat_t": 0.0, "ref_t": 0.0}
    for cat, a in agg.items():
        n = a["n"]
        print(f"{cat:8s} q={n:2d} nat={a['nat_n']/n:4.1f} ref={a['ref_n']/n:4.1f} "
              f"overlap={a['overlap']/n:4.1f} t={a['nat_t']/n:4.1f}s/{a['ref_t']/n:4.1f}s")
        for k in tot:
            tot[k] += a[k]
    n = tot["n"]
    print(f"\nTOTAL q={n} nat={tot['nat_n']/n:.1f}/q ref={tot['ref_n']/n:.1f}/q "
          f"overlap={tot['overlap']/n:.1f}/q t={tot['nat_t']/n:.1f}s/{tot['ref_t']/n:.1f}s")
    print("native unresponsive:", dict(sum((agg[c]["nat_un"] for c in agg), Counter())))
    print("\n=== per-engine overlap (hit = ref-matching URLs contributed) ===")
    for cat, a in agg.items():
        rows = sorted(a["eng_hit"].items(), key=lambda kv: (-kv[1], kv[0]))
        detail = ", ".join(f"{e}:{h}/{a['eng_seen'][e]}" for e, h in rows)
        print(f"{cat:8s} {detail}")
    print("ref unresponsive:   ", dict(sum((agg[c]["ref_un"] for c in agg), Counter())))
    if fails:
        print("FAILED:", fails)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
