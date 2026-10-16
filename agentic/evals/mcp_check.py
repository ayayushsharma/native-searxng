#!/usr/bin/env python3
"""MCP conformance for native-searxng (pinned 2025-06-18). Stdlib only.

Covers both transports: stdio (--mcp, newline-delimited) and Streamable
HTTP (POST /mcp, one JSON body in/out, no SSE). The MCP Inspector is the
reference client; until it runs against us (needs Node, better on a
laptop), this script asserts the same shapes it would check.

Usage: SEARXNG_API_TOKEN=<tok> python3 evals/mcp_check.py --base http://127.0.0.1:8888 [--mcp-bin ./target/debug/native-searxng]
Exit 0 = conformant; 1 = failure. Auth: HTTP calls send the bearer from
env (server refuses tokenless starts, so unset token = expect 401s).
"""
import argparse
import json
import os
import subprocess
import sys
import urllib.request

PROTO = "2025-06-18"


def token():
    return os.environ.get("SEARXNG_API_TOKEN", "").split(",")[0].strip()


def post(base, body, authed=True):
    data = json.dumps(body).encode()
    hdrs = {"Content-Type": "application/json"}
    if authed and token():
        hdrs["Authorization"] = f"Bearer {token()}"
    req = urllib.request.Request(base.rstrip("/") + "/mcp", data=data, headers=hdrs)
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            return r.status, json.loads(r.read().decode())
    except urllib.error.HTTPError as e:
        return e.code, {"_http_error": e.code}


def stdio(binpath, lines):
    p = subprocess.run(
        [binpath, "--mcp"], input="\n".join(lines) + "\n",
        capture_output=True, text=True, timeout=180,
        env={**os.environ},
    )
    return [json.loads(l) for l in p.stdout.splitlines() if l.strip()]


def check(name, cond, detail=""):
    print(("PASS " if cond else "FAIL ") + name + (f" — {detail}" if detail and not cond else ""))
    return cond


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base", default="http://127.0.0.1:8888")
    ap.add_argument("--mcp-bin", default="./target/debug/native-searxng")
    args = ap.parse_args()
    ok = True

    # --- Streamable HTTP ---
    s, init = post(args.base, {"jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {"protocolVersion": PROTO, "capabilities": {}, "clientInfo": {"name": "mcp_check", "version": "0"}}})
    if s == 401 and not token():
        print("SKIP http: server needs SEARXNG_API_TOKEN and none is set (expected 401).")
        print("Set it and re-run for the full HTTP half.")
    else:
        r = init.get("result", {})
        ok &= check("http initialize proto", r.get("protocolVersion") == PROTO, str(init)[:160])
        ok &= check("http initialize serverInfo", r.get("serverInfo", {}).get("name") == "native-searxng")
        ok &= check("http initialize tools cap", "tools" in r.get("capabilities", {}))

        s, lst = post(args.base, {"jsonrpc": "2.0", "id": 2, "method": "tools/list"})
        tools = {t["name"]: t for t in lst.get("result", {}).get("tools", [])}
        ok &= check("http tools/list search+fetch", set(tools) == {"search", "fetch"}, str(lst)[:160])
        ok &= check("http search schema requires query",
                    tools.get("search", {}).get("inputSchema", {}).get("required") == ["query"])
        ok &= check("http fetch schema requires url",
                    tools.get("fetch", {}).get("inputSchema", {}).get("required") == ["url"])

        s, sr = post(args.base, {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "search", "arguments": {"query": "flask request context",
                                                       "categories": "it", "max_results": 2}}})
        txt = (sr.get("result", {}).get("content", [{}])[0].get("text", ""))
        ok &= check("http search returns numbered text", txt.startswith("1. ") and "URL: " in txt, txt[:160])

        s, fr = post(args.base, {"jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "fetch", "arguments": {"url": "https://example.com"}}})
        ftxt = (fr.get("result", {}).get("content", [{}])[0].get("text", ""))
        ok &= check("http fetch Title/Content/Links",
                    ftxt.startswith("Title: ") and "Links found:" in ftxt, ftxt[:160])

        s, e1 = post(args.base, {"jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": {"name": "search", "arguments": {}}})
        ok &= check("http search w/o query isError",
                    e1.get("result", {}).get("isError") is True, str(e1)[:160])

        s, e2 = post(args.base, {"jsonrpc": "2.0", "id": 6, "method": "tools/call",
            "params": {"name": "nope", "arguments": {}}})
        ok &= check("http unknown tool -32602", e2.get("error", {}).get("code") == -32602, str(e2)[:160])

        s, e3 = post(args.base, {"jsonrpc": "2.0", "id": 7, "method": "nope/method"})
        ok &= check("http unknown method -32601", e3.get("error", {}).get("code") == -32601, str(e3)[:160])

        # garbage body -> -32700 (raw post to bypass json.dumps)
        req = urllib.request.Request(args.base.rstrip("/") + "/mcp", data=b"{bad",
                                     headers={"Content-Type": "application/json",
                                              **({"Authorization": f"Bearer {token()}"} if token() else {})})
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                g = json.loads(r.read().decode())
        except Exception:
            g = {}
        ok &= check("http garbage body -32700", g.get("error", {}).get("code") == -32700, str(g)[:160])

        s, notif = post(args.base, {"jsonrpc": "2.0", "method": "notifications/initialized"})
        ok &= check("http notification -> null result", notif.get("result") is None, str(notif)[:160])

        if token():
            s, _ = post(args.base, {"jsonrpc": "2.0", "id": 8, "method": "tools/list"}, authed=False)
            ok &= check("http tokenless -> 401", s == 401, f"status={s}")

    # --- stdio framing (no auth: OS trust boundary) ---
    out = stdio(args.mcp_bin, [
        json.dumps({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {"protocolVersion": PROTO, "capabilities": {}, "clientInfo": {"name": "t", "version": "0"}}}),
        json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json.dumps({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json.dumps({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": {"name": "search", "arguments": {"query": "rfc9000",
                        "categories": "it", "max_results": 1, "engines": "datatracker"}}}),
        "{bad json",
        json.dumps({"jsonrpc": "2.0", "id": 4, "method": "bogus"}),
    ])
    by_id = {d.get("id"): d for d in out if "id" in d and d.get("id") is not None}
    ok &= check("stdio 4 responses + silence (notif) + parse-err",
                len(out) == 5 and len(by_id) == 4, f"lines={len(out)}")
    ok &= check("stdio initialize proto", by_id.get(1, {}).get("result", {}).get("protocolVersion") == PROTO)
    ok &= check("stdio search works",
                "1. " in by_id.get(3, {}).get("result", {}).get("content", [{}])[0].get("text", ""),
                str(by_id.get(3))[:160])
    ok &= check("stdio bogus -> -32601", by_id.get(4, {}).get("error", {}).get("code") == -32601)

    print("MCP CONFORMANT" if ok else "MCP NON-CONFORMANT")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
