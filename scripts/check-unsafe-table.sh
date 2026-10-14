#!/usr/bin/env bash
# Fails when the number of `unsafe {` blocks under crates/ordstack/src differs
# from the number of numbered rows in SOUNDNESS.md's table of unsafe blocks.
set -euo pipefail
cd "$(dirname "$0")/.."
blocks="$(cat crates/ordstack/src/*.rs | grep -o 'unsafe {' | wc -l | tr -d ' ')"
rows="$(grep -cE '^\| [0-9]+ \| `[a-z_]+\.rs` \|' SOUNDNESS.md || true)"
echo "unsafe blocks in crates/ordstack/src: $blocks; rows in SOUNDNESS.md: $rows"
if [ "$blocks" != "$rows" ]; then
    echo "SOUNDNESS.md's unsafe-block table is out of date" >&2
    exit 1
fi

# Also fails when the stated number of compile-fail cases is stale.
cases="$(ls crates/ordstack/tests/ui/fail/*.rs | wc -l | tr -d ' ')"
stated="$(grep -oE '\*\*Compile-fail tests\.\*\* [0-9]+ cases' SOUNDNESS.md | grep -oE '[0-9]+' || true)"
echo "compile-fail cases: $cases; stated in SOUNDNESS.md: $stated"
if [ "$cases" != "$stated" ]; then
    echo "SOUNDNESS.md's count of compile-fail cases is out of date" >&2
    exit 1
fi
