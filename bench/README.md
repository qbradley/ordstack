# bench: is ordstack useful on two real workloads?

Two small programs from the malloc-stacks corpus, each built in seven
variants, timed against each other. Results and their reading:
[RESULTS.md](RESULTS.md).

| variant | what |
|---|---|
| `std` | the original program (`src/orig.rs`), system allocator (glibc) |
| `std-mimalloc` | the same source with `mimalloc` as `#[global_allocator]` |
| `std-control` | the original plus only the non-allocator source changes the ports made (see each file's header); a control, not part of the reading |
| `ordstack` | the port onto ordstack 0.1 (`src/bin/ordstack.rs`) |
| `ordstack-alloc` | the original with its containers on one ordstack frame through `FrameAlloc` (feature `allocator-api2`): allocator-api2 `Vec`/`Box` and hashbrown, otherwise unchanged (`src/bin/ordstack_alloc.rs`) |
| `bumpalo` | the port onto bumpalo 3 (`src/bin/bumpalo.rs`) |
| `bumpalo-frozen` | the bumpalo port with the ordstack port's node layout, frozen handles (`treebuild/src/bin/bumpalo_frozen.rs`; for serde, the same source as `bumpalo`, which already has it) |

Workloads: `treebuild` (a 5-ary tree of named nodes with a tag map each;
depth 7 = 97,656 nodes, depth 8 = 488,281) and `serde_json_xform` (parse an
array of records with serde_json, transform each, write a new array;
120,000 and 480,000 records). `treebuild/src/orig.rs` is byte-identical to
`malloc-stacks/stackfit_spike/orig/treebuild/src/main.rs`;
`serde_json_xform/src/orig.rs` is the original plus a phase timer (lines
marked `BENCH`, output to stderr only when `XFORM_PHASES` is set). Every
variant must print the same stdout as `std`, and `serde_json_xform` must
write a byte-identical output file; the harness exits non-zero otherwise.

The ports are safe code (`#![forbid(unsafe_code)]`). Comments marked
`PORT` record what the library's API forced; lines marked `BENCH` are
instrumentation and are excluded from the source-cost counts.

## Reproduce

On a Linux machine with Rust (stable), Python 3, `perf` (optional) and a C
compiler (for mimalloc):

```sh
bench/run.sh                         # release profile; the reading uses this
PROFILE=release-lto bench/run.sh     # fat LTO, 1 codegen unit; sensitivity check
```

`run.sh` records the machine and toolchain in `work/env-<profile>.txt`,
generates the two JSON inputs into `work/` if missing (`gen_json.py`, seed
1), builds every binary once (`--locked`), then runs `harness.py`:
output-identity check, then per input: pick the least-busy physical core
(never CPU 0), 3 warm-up rounds, then `RUNS` (default 25; the
published numbers use 31) timed rounds in which each variant runs once,
the order rotating each round, every run pinned to that CPU (`CORE` forces one;
set with `sched_setaffinity` in the child, as `taskset -c` does). A case
whose worst wall-time IQR exceeds 4% of the median is re-run (at most twice).
Load is recorded before and during each case. It
records wall time, `ru_maxrss`, user/sys time and minor faults per run, then
`perf stat -e instructions:u,instructions,cycles` (`PERF_RUNS`, default 7, rotated; skipped if perf
is unavailable), then one run of each ordstack and bumpalo variant with `ORDSTACK_STATS=1` for
arena statistics. `rss.py` then measures peak RSS under `/usr/bin/time -f %M`
(5 rotated runs; the harness's own wait4 figure is floored at its ~20 MB),
and
`summarise.py` turns the JSON into the tables in
`work/summary-<profile>.md` and applies the reading rule mechanically;
`srcstats.py` writes the source-cost table to `work/srcstats.md`. Generated
inputs, raw JSON and logs stay in `work/` (git-ignored).

`CASES=treebuild-d7,xform-120k` restricts the inputs, which is handy for a
quick check.
