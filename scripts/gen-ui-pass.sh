#!/usr/bin/env bash
# Regenerates tests/ui/pass from tests/ui/fail by deleting each `// ERROR` line.
set -euo pipefail
cd "$(dirname "$0")/../crates/ordstack/tests/ui"
rm -rf pass && mkdir pass
for f in fail/*.rs; do
    [ "$(grep -c '// ERROR' "$f")" = 1 ] || { echo "$f: need exactly one // ERROR line" >&2; exit 1; }
    { echo '#![allow(unused)]'; grep -v '// ERROR' "$f"; } > "pass/$(basename "$f")"
done
