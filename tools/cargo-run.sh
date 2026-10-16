#!/bin/sh
# cargo-run.sh — portable cargo guard for mise tasks. Replaces the
# Termux-only ~/scripts/safe-build.sh (oom_adj, wake-lock) with
# behavior that works on Termux, macOS, and Linux:
#   - ONE cargo at a time (atomic mkdir lock; stale locks from a
#     killed run are removed on next success path — exit 1 tells you).
#   - Jobs auto-detect: nproc (Linux/Termux) -> sysctl (macOS) ->
#     fallback 4. Override with JOBS= (phone: JOBS=4 check/test,
#     JOBS=2 build; beefy hosts: JOBS=$(nproc) is the default anyway).
# Usage: sh tools/cargo-run.sh <check|test|build> [extra cargo args...]
set -u
LOCK="target/.cargo-lock"
if ! mkdir "$LOCK" 2>/dev/null; then
  echo "cargo-run: another cargo holds $LOCK (one at a time). rm it if stale." >&2
  exit 1
fi
trap 'rmdir "$LOCK" 2>/dev/null' EXIT INT TERM
if [ -n "${JOBS:-}" ]; then
  N="$JOBS"
else
  N="$( (nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4) | head -n 1)"
fi
case "$N" in
  ''|*[!0-9]*) echo "cargo-run: bad JOBS '$N'" >&2; exit 1;;
esac
exec cargo "$@" -j"$N"
