# native-searxng — Rust implementation (see PLAN.md)

Sync-only Rust (no async runtime), SearXNG-compatible `/search&format=json`.

## Build (via mise — portable: Termux, macOS, Linux)
```sh
mise run check            # iterate (jobs auto; phone: JOBS=4)
mise run build             # dev binary (phone: JOBS=2)
mise run build-release     # host release (CI owns; phone: NEVER)
mise run build-linux       # x86_64 release (needs toolchain)
mise run build-android     # arm64 release (needs NDK r27+)
mise run build-mac         # macOS release (runs on a Mac)
mise run debug             # build + serve, throwaway token (localhost)
```
Overrides: `JOBS=` parallelism, `BASE=` server URL, `BIN=` binary,
`TARGET=` triple, `REF=` oracle. `check` skips LLVM so its jobs are
light (proven phone-safe ceiling JOBS=4); codegen at -j4+ risks the
LMK whole-app kill — never `-j8`. First build compiles ~70 crates
(minutes, one time — `target/` is cached, never deleted).

## Run

```sh
cp searxng.toml my.toml   # edit port/engines/proxy_to
./target/debug/native-searxng          # reads ./searxng.toml
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
