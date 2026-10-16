#!/usr/bin/env python3
"""Agent exam: ~20 realistic dev questions through POST /mcp (localhost).

Acts as the coding agent: tools/call search (max 5), record top-3, then
tools/call fetch on the top hit for the FETCH subset. Output is a judging
transcript — verdicts (useful/partial/miss) are assigned by a human from
this log, not by the script. Bearer from SEARXNG_API_TOKEN.

Usage: SEARXNG_API_TOKEN=<tok> python3 evals/agent_exam.py --base http://127.0.0.1:8888
"""
import argparse
import json
import os
import sys
import urllib.request

# (usecase, query, categories, engines-hint, fetch_top?)
EXAM = [
    ("pkg-eval", "serde json", "packages", "", True),
    ("pkg-eval", "pytest django", "packages", "", False),
    ("pkg-eval", "junit mockito", "packages", "maven", False),
    ("pkg-eval", "redis python client", "packages", "", False),
    ("pkg-eval", "terraform aws vpc", "packages", "terraform", True),
    ("pkg-eval", "go router", "packages", "pkggodev", False),
    ("docs", "flask request context", "it", "flask", True),
    ("docs", "pip install requirements", "it", "pipdocs", False),
    ("docs", "docker compose volumes", "it", "", False),
    ("docs", "archlinux wifi", "it", "archlinux", False),
    ("rfc", "rfc9000 QUIC handshake", "it", "datatracker", True),
    ("cheatsheet", "python list", "it", "cheatsh", False),
    ("error", "python requests SSLError", "it", "", False),
    ("cve", "log4j", "it", "nvd", True),
    ("science", "crispr off-target", "science", "", False),
    ("science", "james webb first deep field", "science", "", False),
    ("news", "bundestag", "news", "tagesschau", False),
    ("news", "central bank rate decision", "news", "", False),
    ("general", "rust async runtime comparison", "general", "", True),
    ("general", "exynos 850 specifications", "general", "", False),
]


def call(base, hdrs, name, args, _id):
    body = json.dumps({"jsonrpc": "2.0", "id": _id, "method": "tools/call",
                       "params": {"name": name, "arguments": args}}).encode()
    req = urllib.request.Request(base.rstrip("/") + "/mcp", data=body,
                                 headers={"Content-Type": "application/json", **hdrs})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.loads(r.read().decode())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://127.0.0.1:8888")
    args = ap.parse_args()
    tok = os.environ.get("SEARXNG_API_TOKEN", "").split(",")[0].strip()
    hdrs = {"Authorization": f"Bearer {tok}"} if tok else {}
    n = 0
    for use, q, cat, eng, do_fetch in EXAM:
        n += 1
        a = {"query": q, "categories": cat, "max_results": 5}
        if eng:
            a["engines"] = eng
        try:
            r = call(args.base, hdrs, "search", a, n)
            txt = r.get("result", {}).get("content", [{}])[0].get("text", "")
        except Exception as e:  # noqa: BLE001
            print(f"[{n:02d}] {use} :: {q!r} -> SEARCH FAILED: {e}")
            continue
        if r.get("result", {}).get("isError"):
            print(f"[{n:02d}] {use} :: {q!r} -> TOOL ERROR: {txt[:120]}")
            continue
        blocks = [b for b in txt.split("\n\n") if b.strip()]
        print(f"[{n:02d}] {use} :: {q!r} -> {len(blocks)} results")
        urls = []
        for b in blocks[:3]:
            lines = b.splitlines()
            title = lines[0][3:] if lines and lines[0][:3].rstrip(".") else "?"
            url = next((l.split("URL: ", 1)[1] for l in lines if "URL: " in l), "?")
            urls.append(url)
            print(f"      {title[:70]} | {url[:80]}")
        if do_fetch and urls and urls[0] != "?":
            try:
                f = call(args.base, hdrs, "fetch", {"url": urls[0]}, 100 + n)
                ftxt = f.get("result", {}).get("content", [{}])[0].get("text", "")
                first = ftxt.splitlines()[0] if ftxt else "(empty)"
                clen = len(ftxt)
                walled = "bot-walled" in ftxt or "challenge" in first.lower()
                print(f"      FETCH: {first[:80]} [{clen} chars]{' WALLED' if walled else ''}")
            except Exception as e:  # noqa: BLE001
                print(f"      FETCH FAILED: {e}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
