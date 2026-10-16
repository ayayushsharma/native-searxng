#!/usr/bin/env python3
"""Merge-weight proposer (track #2): data-driven conf order.

Reads a quality.py run log (per-engine overlap table + native
unresponsive line) and the current searxng.toml [engines] arrays, then
prints proposed arrays ordered by score = hits - 2*unresponsive, ties
keeping current conf order (stable = merge weights never churn on
noise). Human reviews before applying: single-run 429 waves can demote
good engines, so the script shows the score behind every move.

Usage: python3 agentic/evals/weights.py <quality.log> [searxng.toml]
"""
import re
import sys
from collections import Counter

CATS = ["general", "it", "science", "news", "packages",
        "videos", "dictionaries", "weather"]


def parse_log(path):
    hits, seen, unresp = {}, Counter(), Counter()
    with open(path) as f:
        for line in f:
            m = re.match(r"^(\w+)\s+(.*)$", line.rstrip())
            if m and m.group(1) in CATS and ":" in m.group(2):
                cat = m.group(1)
                for em in re.finditer(r"(\w+):(\d+)/(\d+)", m.group(2)):
                    e, h, s = em.group(1), int(em.group(2)), int(em.group(3))
                    hits.setdefault(cat, {})[e] = h
                    seen[(cat, e)] = s
            m = re.match(r"^native unresponsive: (\{.*\})$", line.rstrip())
            if m:
                unresp = Counter(eval(m.group(1)))  # noqa: S307 - own log
    return hits, unresp


def parse_toml(path):
    order, cur = {}, None
    with open(path) as f:
        for line in f:
            s = line.strip()
            if s in [f"{c} = [" for c in CATS]:
                cur = s.split()[0]
                order[cur] = []
            elif re.match(r"^(\w+) = \[", s):
                # Single-line array: videos = ["sepiasearch"].
                m = re.match(r"^(\w+) = \[(.*)\]", s)
                order[m.group(1)] = re.findall(r'"([\w]+)"', m.group(2))
            elif cur:
                if s == "]":
                    cur = None
                else:
                    order[cur].extend(re.findall(r'"([\w]+)"', s))
    return order


def main():
    log, toml = sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else "searxng.toml"
    hits, unresp = parse_log(log)
    order = parse_toml(toml)
    for cat in CATS:
        arr = order.get(cat, [])
        h = hits.get(cat, {})
        scored = [(h.get(e, 0) - 2 * unresp.get(e, 0), -i, e)
                  for i, e in enumerate(arr)]
        scored.sort(key=lambda t: (-t[0], t[1]))
        new = [e for _, _, e in scored]
        mark = "  <-- CHANGED" if new != arr else ""
        print(f"{cat} = [")
        print("  " + ", ".join(f'"{e}"' for e in new) + ",")
        print(f"]{mark}")
        if mark:
            detail = ", ".join(f"{e}:{h.get(e,0)}h/{unresp.get(e,0)}u" for e in new)
            print(f"#   scores: {detail}")


if __name__ == "__main__":
    main()
