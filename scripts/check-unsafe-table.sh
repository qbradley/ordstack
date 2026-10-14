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

# Also fails when a module that resets the stack or writes a frame's `end`
# is missing from SOUNDNESS.md's list of `reset_to` call sites (Why T holds)
# or from the list of places that write `F.end` (Why frozen blocks stay
# valid), both of which claim to be complete.
ttable="$(awk '/^The call sites, all of them:/,/^`reset_to` checks/' SOUNDNESS.md)"
epara="$(awk '/^E holds because/,/^$/' SOUNDNESS.md)"
for f in $(grep -l 'reset_to(' crates/ordstack/src/*.rs | grep -v '/stack\.rs$'); do
    case "$(basename "$f" .rs)" in
        frame) ty=Frame ;; vec) ty=SVec ;; map) ty=SMap ;; *) ty="unlisted module $f" ;;
    esac
    if ! grep -q "\`$ty::" <<<"$ttable"; then
        echo "SOUNDNESS.md's reset_to call-site table has no $ty row ($f calls reset_to)" >&2
        exit 1
    fi
done
for f in $(grep -lE '(frame|self)\.end = ' crates/ordstack/src/*.rs); do
    case "$(basename "$f" .rs)" in
        frame) w=try_alloc_array ;; vec) w=SVec::freeze ;; map) w=SMap::freeze ;; *) w="unlisted module $f" ;;
    esac
    if ! grep -q "$w" <<<"$epara"; then
        echo "SOUNDNESS.md's list of F.end writers omits $w ($f writes a frame end)" >&2
        exit 1
    fi
done
echo "reset_to call sites and F.end writers: listed"
