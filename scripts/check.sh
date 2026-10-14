#!/usr/bin/env bash
# Every verification job for ordstack. FUZZ_SECONDS sets the fuzz budget.
#
# This is meant for a BIG machine (it builds std for ASan, runs Miri twice and
# fuzzes). Do not run it on the small dev box; use the train box.
set -euo pipefail
cd "$(dirname "$0")/.."
host="$(rustc -vV | sed -n 's/^host: //p')"
step() { printf '\n== %s ==\n' "$*"; }

step "format";                  cargo fmt --all --check
step "clippy";                  cargo clippy --workspace --all-targets -- -D warnings
step "unsafe table is current"; scripts/check-unsafe-table.sh
step "tests (stable, debug)";   cargo test --workspace
step "tests (stable, release)"; cargo test --workspace --release
step "ui twins are current"
scripts/gen-ui-pass.sh
if [ -n "$(git status --porcelain -- crates/ordstack/tests/ui/pass)" ]; then
    git status --short -- crates/ordstack/tests/ui/pass
    git diff -- crates/ordstack/tests/ui/pass
    echo "the committed passing twins are stale: run scripts/gen-ui-pass.sh and commit" >&2
    exit 1
fi
step "tests + ui (MSRV 1.85)";  ORDSTACK_UI=1 cargo +1.85 test --workspace
step "docs";                    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
step "miri (stacked borrows)";  cargo +nightly miri test -p ordstack
step "miri (tree borrows)";     MIRIFLAGS=-Zmiri-tree-borrows cargo +nightly miri test -p ordstack
step "address sanitizer";       RUSTFLAGS=-Zsanitizer=address cargo +nightly test -p ordstack --tests -Zbuild-std --target "$host"
step "fuzz";                    (cd crates/ordstack && cargo +nightly fuzz run model -- -max_total_time="${FUZZ_SECONDS:-60}" -max_len=4096)
printf '\nAll checks passed.\n'
