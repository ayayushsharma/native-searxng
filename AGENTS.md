# AGENTS.md — operating rules for agentic runs (phone homeserver)

Binding companions: `AGENT-MISSION.md` (mission loop + stop triggers),
`PLAN.md` (embedded budget + scope), `agentic/CONTRACT.md` (frozen keys).

## CI — sparing by default (runner budget)

- Every push / PR runs `check` ONLY (`cargo check --locked --all-targets`,
  one `ubuntu-latest` runner, cached). That is the whole per-push cost.
- Binaries come from ONE release job (`release`: x86_64 + Android built
  back-to-back on a single runner, shared cache, one artifact). It runs
  **only on `v*` tags or manual `workflow_dispatch`** — never per push.
- No musl/router target until a router actually ships (deferred to keep
  the matrix at one job).
- `concurrency: cancel-in-progress` is set — superseded runs cancel instead
  of queueing, so a fast push stream never blocks on stale runners.
- Do NOT add jobs/targets/matrix entries without deleting one first.
  New CI work = RAM/credit line-item, same discipline as `PLAN.md` §3.
- Phone never builds releases (`cargo build --release` / LTO / cross):
  rustc codegen peaks GBs and LMK-kills Termux. Phone proves correctness
  (`check -j4`, dev `build -j2`, `check.py`, fixtures); CI proves footprint.

## Phone rules (non-negotiable)

- Iterate: `~/scripts/safe-build.sh cargo check -j4`. Run: `cargo build -j2`.
  Never `-j8`. One cargo at a time. Server STOPPED during builds.
- Fixture-first: ≤5 live queries per engine, then offline. Live loops risk
  a home-IP ban — top ban-risk of the whole run.
- `check.py` gate per engine, one commit per engine, push checkpoints.
- `target/` is 4.5 min of deps: never `cargo clean`, never delete to "save
  space". `build.log` stays until root-caused.
