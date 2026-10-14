#!/usr/bin/env bash
# Builds every variant once, generates the inputs if missing, checks output
# identity, runs the timed benchmark and writes work/summary-<profile>.md.
# Run on the benchmark machine, from anywhere:
#
#   bench/run.sh                     # release profile (the reading)
#   PROFILE=release-lto bench/run.sh # sensitivity check
#
# Knobs (environment): RUNS (25), WARMUP (3), CORE (11), PERF_RUNS (5),
# CASES (comma list: treebuild-d7,treebuild-d8,xform-120k,xform-480k).
set -euo pipefail
cd "$(dirname "$0")"
PROFILE=${PROFILE:-release}
mkdir -p work

{
  echo "## date"; date -u
  echo "## host"; uname -a
  echo "## cpu"; lscpu | grep -E 'Model name|^CPU\(s\)|Thread|Socket|MHz|L[123]'
  echo "## governor"; cat /sys/devices/system/cpu/cpu${CORE:-11}/cpufreq/scaling_governor 2>/dev/null || echo n/a
  echo "## load"; cat /proc/loadavg
  echo "## glibc"; ldd --version | head -1
  echo "## rustc"; rustc -Vv
  echo "## cargo"; cargo -V
  echo "## python"; python3 -V
  echo "## perf"; perf --version 2>/dev/null || echo "no perf"
  echo "## perf_event_paranoid"; cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo n/a
} > work/env-$PROFILE.txt 2>&1

[ -f work/big.json ] || python3 -I gen_json.py 120000 work/big.json
[ -f work/big4.json ] || python3 -I gen_json.py 480000 work/big4.json
ls -l work/big.json work/big4.json >> work/env-$PROFILE.txt
sha256sum work/big.json work/big4.json >> work/env-$PROFILE.txt

cargo build --locked --profile "$PROFILE" --bins 2>&1 | tail -3
BIN=target/$PROFILE
cargo tree --locked -e normal --depth 1 >> work/env-$PROFILE.txt

# Only the std-mimalloc binaries may contain mimalloc.
for p in treebuild serde_json_xform; do
  for v in std std-mimalloc std-control ordstack bumpalo; do
    printf '%s-%s mi_free symbols: %s\n' "$p" "$v" \
      "$(nm "$BIN/$p-$v" | grep -c ' T mi_free$' || true)" >> work/env-$PROFILE.txt
  done
done

python3 -I harness.py "$BIN" work "work/results-$PROFILE.json"
python3 -I summarise.py "work/results-$PROFILE.json" > "work/summary-$PROFILE.md"
python3 -I srcstats.py > work/srcstats.md
cat work/env-$PROFILE.txt work/summary-$PROFILE.md work/srcstats.md
