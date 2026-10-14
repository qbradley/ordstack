#!/usr/bin/env bash
# Builds every variant once, generates the inputs if missing, checks output
# identity, runs the timed benchmark and writes work/summary-<profile>.md.
# Run on the benchmark machine, from anywhere:
#
#   bench/run.sh                     # release profile (the reading)
#   PROFILE=release-lto bench/run.sh # sensitivity check
#
# Knobs (environment): RUNS (25), WARMUP (3), CORE (unset: harness picks an
# idle core per case), PERF_RUNS (7), NOISE_IQR (0.04), RETRIES (2),
# CASES (comma list: treebuild-d7,treebuild-d8,xform-120k,xform-480k).
set -eu
cd "$(dirname "$0")"
PROFILE=${PROFILE:-release}
mkdir -p work
ENVF=work/env-$PROFILE.txt

load_snapshot() {
  echo "## load ($1): $(date -u +%FT%TZ)"
  uptime
  free -m | sed -n 1,2p
  if command -v mpstat >/dev/null; then
    # per-CPU busy % (100 - %idle) over 3 s, compact: cpu:busy
    mpstat -P ALL 1 3 | awk '/^Average/ && $2 ~ /^[0-9]+$/ {printf "%s:%.0f ", $2, 100-$NF} END {print ""}'
  else
    top -bn1 | sed -n 1,20p
  fi
}

{
  echo "## date"; date -u
  echo "## host"; uname -a
  echo "## cpu"; lscpu | grep -E 'Model name|^CPU\(s\)|Thread|Socket|MHz|L[123]|NUMA' || true
  echo "## governor"; cat /sys/devices/system/cpu/cpu1/cpufreq/scaling_governor 2>/dev/null || echo n/a
  echo "## glibc"; ldd --version 2>&1 | sed -n 1p
  echo "## rustc"; rustc -Vv
  echo "## cargo"; cargo -V
  echo "## python"; python3 -V
  echo "## perf"; perf --version 2>/dev/null || echo "no perf"
  echo "## perf_event_paranoid"; cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo n/a
  echo "## thp"; cat /sys/kernel/mm/transparent_hugepage/enabled /sys/kernel/mm/transparent_hugepage/defrag 2>/dev/null || echo n/a
  echo "## smt"; cat /sys/devices/system/cpu/smt/active 2>/dev/null || echo n/a
} > "$ENVF" 2>&1

[ -f work/big.json ] || python3 -I gen_json.py 120000 work/big.json
[ -f work/big4.json ] || python3 -I gen_json.py 480000 work/big4.json
ls -l work/big.json work/big4.json >> "$ENVF"
sha256sum work/big.json work/big4.json >> "$ENVF"

cargo build --locked --profile "$PROFILE" --bins > work/build-$PROFILE.log 2>&1 \
  || { cat work/build-$PROFILE.log; exit 1; }
BIN=target/$PROFILE
cargo tree --locked -e normal --depth 1 >> "$ENVF"

# Only the std-mimalloc binaries may contain mimalloc.
for p in treebuild serde_json_xform; do
  for v in std std-mimalloc std-control ordstack bumpalo; do
    n=$(nm "$BIN/$p-$v" | grep -c ' T mi_free$' || true)
    echo "$p-$v mi_free symbols: $n" >> "$ENVF"
  done
done

load_snapshot "before timed runs" >> "$ENVF"
python3 -I harness.py "$BIN" work "work/results-$PROFILE.json"
python3 -I rss.py "$BIN" work "work/results-$PROFILE.json"
load_snapshot "after timed runs" >> "$ENVF"
python3 -I summarise.py "work/results-$PROFILE.json" > "work/summary-$PROFILE.md"
python3 -I srcstats.py > work/srcstats.md
cat "$ENVF" "work/summary-$PROFILE.md" work/srcstats.md
