#!/bin/sh
# Memory stress test: concurrent heavy searches (worst realistic case:
# user + MCP + console firing together) with high-frequency RSS sampling.
# Bounded on purpose: C concurrent queries x R rounds, default 4x3, and
# a hard per-round deadline (180s) so a wedged upstream can't hang the
# harness. Completion is tracked via sentinel files (not `jobs`, whose
# table semantics vary across /bin/sh).
# Reports steady RSS, peak RSS, per-query latency, failures.
# Usage: SEARXNG_API_TOKEN=... sh tools/stress.sh [concurrency] [rounds]
set -u
BASE="${BASE:-http://127.0.0.1:8888}"
C="${1:-4}"
R="${2:-3}"
TOK="$(printf '%s' "${SEARXNG_API_TOKEN:-}" | cut -d, -f1)"
if [ -z "$TOK" ]; then echo "set SEARXNG_API_TOKEN"; exit 1; fi
if ! curl -sf "$BASE/healthz" >/dev/null; then echo "server not at $BASE"; exit 2; fi
PID=$(ps -e -o pid,args 2>/dev/null | grep '[n]ative-searxng$\|[n]ative-searxng ' | grep -v stress.sh | grep -v tmux | awk '{print $1}' | head -1)
if [ -z "$PID" ]; then echo "server process not found"; exit 2; fi
echo "stress pid=$PID concurrency=$C rounds=$R"
rss() { awk '/VmRSS/{print $2}' /proc/$PID/status 2>/dev/null || echo 0; }
STEADY=$(rss)
QUERIES="search?q=flask+request+context&format=json&categories=it
search?q=rust+async+runtime+comparison&format=json
search?q=serde+json&format=json&categories=packages
search?q=transformer+architecture+paper&format=json&categories=science"
TMP="$(mktemp -d)"
PEAK=$STEADY
round=1
while [ $round -le $R ]; do
  EXPECT=0
  n=0
  for q in $QUERIES; do
    if [ $n -ge $C ]; then break; fi
    n=$((n + 1))
    EXPECT=$((EXPECT + 1))
    ( T0=$(date +%s%N)
      code=$(curl -s --max-time 120 -H "Authorization: Bearer $TOK" -o /dev/null -w "%{http_code}" "$BASE/$q")
      T1=$(date +%s%N)
      echo "$code $(( (T1 - T0) / 1000000 ))ms $q" >> "$TMP/r$round"
      touch "$TMP/done.$round.$n" ) &
  done
  # Bounded sampler: all sentinels present, or 180s deadline.
  i=0
  while [ $i -lt 900 ]; do
    ALIVE=0
    s=1
    while [ $s -le $EXPECT ]; do
      [ -f "$TMP/done.$round.$s" ] || ALIVE=1
      s=$((s + 1))
    done
    NOW=$(rss); [ "$NOW" -gt "$PEAK" ] && PEAK=$NOW
    [ $ALIVE -eq 0 ] && break
    sleep 0.2
    i=$((i + 1))
  done
  wait
  echo "round $round done"
  round=$((round + 1))
done
echo "--- per-query (code latency query) ---"
cat "$TMP"/r*
FAIL=$(grep -h "^[^2]" "$TMP"/r* | wc -l)
echo "steady=${STEADY}kB peak=${PEAK}kB non200=${FAIL}"
rm -rf "$TMP"
