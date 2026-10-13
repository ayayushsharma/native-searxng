#!/bin/sh
# Footprint gate: binary size + peak RSS over 50 sequential queries.
# CI asserts: core-only peak <5MB (MVP), full <10MB (PLAN.md §3).
set -e
BIN="${1:-target/release/native-searxng}"
ls -la "$BIN"
size "$BIN" 2>/dev/null || true
BASE="${BASE:-http://127.0.0.1:8888}"
if ! curl -sf "$BASE/healthz" >/dev/null; then
  echo "server not running at $BASE - start it first"; exit 2
fi
if command -v /usr/bin/time >/dev/null; then
  /usr/bin/time -v sh -c "i=0; while [ \$i -lt 50 ]; do curl -sf '$BASE/search?q=test&format=json' >/dev/null; i=\$((i+1)); done" 2>&1 | grep -E "Maximum resident|Elapsed"
else
  i=0; while [ $i -lt 50 ]; do curl -sf "$BASE/search?q=test&format=json" >/dev/null; i=$((i+1)); done
  echo "50 queries done (install GNU time for RSS numbers)"
fi
