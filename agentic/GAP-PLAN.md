# Gap plan: native vs oracle (from `agentic/evals/quality.py`, 36 queries)

Baseline (2026-10-14): native **18.1/q** vs ref **41.1/q**, overlap **7.8/q**,
latency 3.9s vs 2.0s. Zero failures either side.

## Gap inventory

| # | Gap | Size | Root cause | Fixable? |
|---|---|---|---|---|
| G1 | count gap | 18 vs 41/q | 20-result cap + fewer engines/category | yes (P1) |
| G2 | general overlap 19% | 3.4/q | ddg wall 14/14 (IP-bound), startpagina flake 5/36, mwmbl 2/36 | partly (P0/P3) |
| G3 | it overlap 42% | 8.3 of 20/q | engine breadth (oracle: 52/q incl. depth) | yes (P1) |
| G4 | science overlap 58% | 10.7 of 18/q | arxiv flake 5/36; s2/scholar blocked | partly (P0) |
| G5 | news overlap 49% | 8.0 of 16/q | ddgnews challenged 6/6 | IP-bound (P3) |
| G6 | latency ~2x (general/it) | 4.7s vs 1.5s | sequential fan-out vs oracle parallel | yes, RAM trade (P2) |
| — | brave.news/gnews/reuters, lib.rs, metacpan | — | dead for the ORACLE too (its unresponsive list) | no action (parity) |
| — | packages 100% subset | — | done; count 17.5 vs 83.5 only | P1 candidates |

## Phases (each: implement → `check.py` → `quality.py` delta → commit)

**P0 — flake diagnosis (offline-first).** startpagina (5/36), mwmbl (2/36),
arxiv (5/36 native vs 3/36 oracle). Read server logs + fixtures first;
≤3 live probes per engine to characterize (timeout vs block vs shape
change). Acceptance: flake rate halved or root-caused with a retry/timeout
rule that doesn't raise the traffic budget.

**P1 — count parity without RAM blowup.** Two levers, measure between them:
(a) raise the 20-result merge cap (each +10 costs ~transient RAM —
re-measure with `measure.sh`, release RSS must stay under ceiling);
(b) second engine-survey round, fixture-first, 1 live probe each:
packages (pypi, rubygems), it/science stragglers the oracle uses.
Expect some to fail exactly like the oracle does (lib.rs/metacpan
already do) — failure is data. Acceptance: native ≥25/q, overlap ≥12/q,
no category regression, RSS within ceiling.

**P2 — bounded concurrency (latency).** Config-gated lane count
(default 4?) for the engine fan-out. Explicit embedded trade: threads +
concurrent bodies/DOMs raise peak transient — re-gate RSS before and
after, document the latency-per-MB exchange rate. Acceptance: general/it
latency within 1.5x of oracle, RSS still under ceiling.

**P3 — recovery watch (near-zero traffic).** ddg/ddgnews/mojeek/brave are
IP-reputation-bound, and the code already auto-recovers (challenge →
unresponsive, no code change needed when the wall lifts). A weekly
single-probe check (3 queries, negligible) detects recovery; port
nothing until a probe goes green. No action otherwise.

## Non-goals (reaffirmed)

s2/scholar/brave (blocked this network), openaire (dead upstream),
keyed/JS/CAPTCHA engines (policy), `web_fetch` parity (needs full-page
extraction — separate project), CI live traffic (never).

## Traffic budget (moral limits, whole plan)

P0 ≤9 probes, P1 ≤8 probes (survey) + gate runs, P2 zero new probes
(uses existing fixtures + gate), P3 3 probes/week. All survey traffic
sequential with ≥2s spacing, identical in kind to normal search usage.
