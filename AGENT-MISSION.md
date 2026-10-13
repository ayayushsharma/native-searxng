# AGENT MISSION — autonomous engine-porting run (native-searxng)

You are working in `~/projects/work/native-searxng/` on the phone homeserver.
PLAN.md is binding. This brief makes the run autonomous: follow the loop,
respect the stop triggers. Everything else is your judgment.

## Objective

Port engines in §12 order (JSON batch → Mojeek/Bing/DDG → top-10 HTML) keeping
all gates green. Strangler stays intact: anything unported proxies to Python
:8889, agents never see a regression.

## Step 0 — oracle health (do FIRST, blocks everything)

Start the Python reference on :8889 and query a guarded engine (DDG) through it.
If the oracle is ALSO challenged from this network: STOP, report — no ported engine
can pass those walls either, and the whole W1 order changes. Do not burn live
queries re-proving it (≤3 checks). If the oracle passes: guarded engines stay
behind the strangler; native ports only verified-clean endpoints until told otherwise.

## Iron rules (violations = failed run)

1. Sync-only Rust. No new crate without a RAM line-item in PLAN.md §3.
2. `safe-build.sh cargo check -j4` to iterate; `cargo build -j2` only to run.
   One cargo at a time per `target/`. Never -j8. On build failure KEEP
   `build.log` until root-caused (2026-10-14 lesson: log was deleted, cause lost).
3. JSON keys byte-exact per `agentic/CONTRACT.md` — run
   `python3 agentic/evals/check.py --base http://127.0.0.1:8888` after every engine.
4. Fixture-first: ≤5 live queries per engine to record `tests-fixture-<engine>-*.html`,
   then ALL iteration offline against fixtures. Live loops burn rate limits and get
   the home IP banned — this is the top ban-risk of the whole run.
5. Parsers skip malformed input, never crash; challenge/captcha pages → 0 results
   + `unresponsive_engines` entry (see `tests-fixture-ddg-anomaly.html`).
7. RAM hygiene (the compiler is the LMK risk, not the server — rustc peaks GBs,
   the server uses MBs): keep the server STOPPED during builds unless testing;
   prefer `check` over `build` (10x less pressure); release/LTO builds on PC
   ONLY, never on-phone; never `cargo clean` casually (forces cold rebuild =
   peak RAM + 4 minutes); glance at `free -m` before big builds, abort if
   MemAvailable <300 MB (`safe-build.sh` enforces this). If LMK kills happen:
   drop to -j1, preserve logs, report — do not retry harder.
8. Repo is github.com/ayayushsharma/native-searxng (already pushed); commit per
   engine (`engines: add <name> + fixtures + gate green`). Sync = `git pull/push`
   (phone has own network + git — no manual file transfers). CI builds all
   releases; on-phone builds are dev-only.

## Loop per engine (only for oracle-verified-clean endpoints)

1. Record fixture (≤5 live queries, save raw bodies).
2. Implement: one file `src/engines/<name>.rs` + one line in `mod.rs` + conf entry.
3. `cargo check -j4` → fixture test (parse offline, assert titles/urls) → `check.py`.
4. `diff.py` vs Python ref for the same query — no key drift.
5. Commit. Next engine.

## Stop triggers (STOP, report, do not improvise)

- DDG/Bing still challenged AFTER the `wreq` impersonation swap → the egress IP
  may be flagged; further live hammering risks a ban. Report, move to next engine.
- Any engine needing API keys, JS rendering, CAPTCHA solving, or TLS
  impersonation (no budget-compatible impersonator exists — `wreq` was audited
  and rejected: tokio + BoringSSL + 40 crates) → skip + log it.
- Release RSS >10 MB (`tools/measure.sh`) → ceiling breach, human-level call.
- Repeated LMK kills or `target/` corruption → stop, preserve logs.
- Anything requiring network/IP changes, new hosts, or credentials → stop.

## Done = all of

- Engine batch green: `check.py` 0 failures on full `queries.json` (extend it per engine).
- `diff.py` clean vs Python for 10+ representative queries.
- `tools/measure.sh` peak <10 MB; binary size recorded in README.
- Git log shows one commit per engine; PLAN.md §7 W1/W2 boxes ticked.
