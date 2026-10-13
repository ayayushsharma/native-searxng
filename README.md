# native-searxng — Rust implementation (see PLAN.md)

Sync-only Rust (no async runtime), SearXNG-compatible `/search&format=json`.

## Build

PC (fast, releases + cross):
```sh
cargo build                    # dev
cargo build --release          # release (opt-z, stripped, <800KB goal)
cargo build --release --target aarch64-unknown-linux-musl   # router
cargo build --target aarch64-linux-android                  # phone deploy
```

Phone homeserver (Termux — capped parallelism per LMK rules):
```sh
~/scripts/safe-build.sh cargo check -j4   # default gate: ~1s warm, ~90s cold
~/scripts/safe-build.sh cargo build -j2   # only to run; codegen stays -j2
```
`check` skips LLVM so its jobs are light enough for -j4 (proven safe ceiling);
codegen at -j4+ risks the LMK whole-app kill — never `-j8`. First build compiles
~70 crates (minutes, one time — `target/` is cached on 35GB disk).

## Run

```sh
cp searxng.conf my.conf   # edit port/engines/proxy_to
./target/debug/native-searxng          # reads ./searxng.conf
curl '127.0.0.1:8888/search?q=test&format=json' | python3 -m json.tool
```

Strangler: set `proxy_to=http://127.0.0.1:8889` (Python ref) and unknown
categories/engines proxy through transparently.

## Verify

```sh
python3 agentic/evals/check.py --base http://127.0.0.1:8888
python3 diff.py --q "..."   # see agentic/diff/diff.py
sh tools/measure.sh         # size + RSS footprint
```

Discipline (binding, PLAN.md §2): sync only, audited deps only, every new
crate needs a RAM line-item. New engine = one file in `src/engines/` + one
line in `src/engines/mod.rs`.
