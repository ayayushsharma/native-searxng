#!/bin/sh
# Footprint gate: binary size on disk + SERVER peak/avg RSS over N queries.
# NOTE: an earlier version timed the curl client (`time -v` on the loop),
# which measures the wrong process. This polls the server's own RSS via ps.
# Dev binaries read higher than release (no opt, debuginfo, debug
# assertions) — dev numbers are an upper bound; CI asserts release RSS.
# CI asserts: core-only peak <5MB (MVP), full <10MB (PLAN.md §3).
set -e
BIN="${1:-target/release/native-searxng}"
N="${N:-50}"
ls -la "$BIN"
size "$BIN" 2>/dev/null || true
BASE="${BASE:-http://127.0.0.1:8888}"
# Bearer from env when the server runs authed (401s still exercise the
# server, but real fan-out is the honest load — same convention as evals).
AUTH=""
if [ -n "${SEARXNG_API_TOKEN:-}" ]; then
  TOK="$(printf '%s' "$SEARXNG_API_TOKEN" | cut -d, -f1)"
  AUTH="-H Authorization: Bearer $TOK"
fi
if ! curl -sf "$BASE/healthz" >/dev/null; then
  echo "server not running at $BASE - start it first"; exit 2
fi
# Match the server binary precisely: exclude this script's own process
# (its args contain the binary path) and any wrapper shells.
PID=$(ps -e -o pid,args 2>/dev/null | grep '[n]ative-searxng$\|[n]ative-searxng ' | grep -v measure.sh | grep -v tmux | awk '{print $1}' | head -1)
if [ -z "$PID" ]; then echo "server process not found"; exit 2; fi
echo "server pid=$PID queries=$N"
TMP="$(mktemp)"
i=0
( while [ $i -lt "$N" ]; do
    curl -sf $AUTH "$BASE/search?q=test&format=json" >/dev/null
    i=$((i+1))
  done ) &
LOOP=$!
PEAK=0; SUM=0; CNT=0
while kill -0 $LOOP 2>/dev/null; do
  RSS="$(ps -o rss= -p "$PID" 2>/dev/null | tr -d ' ')"
  if [ -n "$RSS" ]; then
    SUM=$((SUM + RSS)); CNT=$((CNT + 1))
    if [ "$RSS" -gt "$PEAK" ]; then PEAK=$RSS; fi
  fi
  sleep 1
done
wait $LOOP
AVG=$((SUM / (CNT + 1)))
echo "server RSS over $N queries: peak=${PEAK}KB avg=${AVG}KB samples=$CNT"
