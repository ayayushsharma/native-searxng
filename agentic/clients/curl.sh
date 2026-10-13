#!/bin/sh
# Minimal agent integration example (curl + jq/python).
BASE="${1:-http://127.0.0.1:8888}"
Q="${2:-rust async runtime}"
curl -s --get "$BASE/search" \
  --data-urlencode "q=$Q" \
  --data-urlencode "format=json" \
  --data-urlencode "pageno=1" \
  --data-urlencode "language=en" \
| python3 -c 'import json,sys; d=json.load(sys.stdin);
print(f"query: {d[\"query\"]} ({len(d[\"results\"])} results)");
[print(f"- {r[\"title\"]}\n  {r[\"url\"]}") for r in d["results"][:5]]'
