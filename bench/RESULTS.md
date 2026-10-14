# ordstack benchmark: results

## Reading rule (fixed before any run; copied verbatim from the brief)

> ordstack is *useful on this evidence* if, on at least one workload at both input sizes, the ordstack port beats `std` by at least 10% in median wall time or peak RSS, with non-overlapping interquartile ranges for time. It is *competitive* if it is within 10% of `bumpalo` on the same measure. Otherwise the reading is *not shown useful by these workloads*. Report source cost too (lines, lifetime parameters added, PORT notes) since a speedup that needs 4x the code is part of the answer.

Also fixed before any run:

- The reading uses the plain `release` profile (opt-level 3, no LTO, the
  default codegen units) for every variant. A `release-lto` run (fat LTO,
  one codegen unit) is a sensitivity check only and does not change the
  reading.
- "Beats by at least 10% in median wall time" means
  `median(ordstack) <= 0.9 * median(std)` and the ordstack time IQR lies
  entirely below std's. "Peak RSS" is the median of `ru_maxrss` over the
  timed runs; RSS has no IQR condition.
- Time and RSS are judged separately: a workload qualifies if one of the
  two measures meets the bar at both input sizes.

Added after the owner's note on train-box load, before the runs reported
below (a first full run had already been taken; see "Preliminary run"):

- Before each case the harness samples `/proc/stat` for 3 s and pins the
  case to the CPU whose physical core (CPU and SMT sibling) was least busy,
  never CPU 0 or its sibling. Load (loadavg, sibling busy %, machine busy %)
  is recorded before and during every case; `uptime` and `mpstat -P ALL`
  before and after all timed runs.
- Re-run rule: if any variant's wall-time IQR is wider than 4% of its
  median, the whole case (all variants) is re-run on a freshly chosen core,
  at most twice more; the attempt with the smallest worst-variant IQR is
  reported, and every attempt is listed.
- `perf stat` instructions (and cycles) are reported for every cell as the
  noise-robust companion to wall time. They do not change the rule above
  (which is about wall time and RSS), but a wall-time win that instructions
  and cycles do not corroborate would be called out as suspect.

## Reading

**ordstack is *useful on this evidence*, and better than *competitive*
with bumpalo.** On both workloads and at both input sizes the ordstack
port beats `std` by far more than 10% in median wall time (IQRs disjoint)
and in peak RSS:

| input | ordstack wall vs std | ordstack RSS vs std | ordstack wall vs bumpalo | ordstack RSS vs bumpalo | instructions:u vs std | cycles vs std |
|---|---|---|---|---|---|---|
| treebuild 7 | −64.9% | −65.1% | −37.0% | −40.5% | −56.0% | −65.8% |
| treebuild 8 | −64.8% | −66.9% | −36.2% | −42.9% | −56.1% | −62.7% |
| serde_json_xform 120k | −30.4% | −41.6% | −3.6% | −33.8% | −19.6% | −29.2% |
| serde_json_xform 480k | −31.0% | −43.4% | −3.5% | −35.2% | −16.6% | −30.4% |

The wall-time wins are corroborated by the noise-robust counters: user-mode
instructions and cycles fall by similar amounts, so these are not
scheduling artefacts.

What that reading does *not* say, and should be read with it:

1. **Against bumpalo, the time win is in `treebuild` only.** On
   `serde_json_xform` ordstack is 3.5% faster than bumpalo (inside the 10%
   "competitive" band) and executes 2% *more* user instructions; its
   remaining edge there is cycles (fewer cache misses and faults). On
   `treebuild` it beats bumpalo by 36–37% in time and 41–43% in RSS. That
   gap is exactly the case the README advertises: bumpalo copies a
   children vector that has to grow while later allocations sit above it,
   and it never reuses the old bytes. bumpalo's arena holds 33.5 MB
   (depth 7) and 268 MB (depth 8) of chunks, against ordstack's 15.6 and
   81.2 MB of stack. Instruction counts are nearly equal (−1.3%); the
   difference is memory traffic (cycles −41%, minor faults 4,095 vs 7,106).
2. **On `serde_json_xform`, a drop-in fast allocator gets the same speed
   for no code.** `std-mimalloc` is −34.0% / −34.4% in wall time against
   ordstack's −30.4% / −31.0%; its parse and transform phases are faster
   than ordstack's. What ordstack adds over mimalloc there is memory: peak
   RSS −41.6% / −43.4% against mimalloc's −3.6% / −14.5%. On `treebuild`
   ordstack is clearly ahead of mimalloc too (33.6 vs 56.1 ms; 16 vs 52 MB).
3. **Part of the `treebuild` gain is the map substitute, not the
   allocator.** The ports replace each node's one-entry `HashMap` with a
   sorted vector of pairs (ordstack has no map). `std-control` (the
   original plus only that change) is 12–13% faster and 9% smaller than
   `std`; ordstack is still about −60% in time and −61% to −64% in
   RSS against `std-control`. On `serde_json_xform` the ports' only non-allocator change
   (no fresh heap `String` per bucket key) is worth 1–2% (`std-control`).
4. **The memory win has two sources that a reader may weigh differently.**
   Frozen handles are smaller than their std counterparts (`SStr` and
   `SSlice` are 16 bytes, `String` and `Vec` 24) and blocks carry no malloc
   header; and there is no free at teardown. The "rest" column of the phase
   timer (process start, teardown, exit) is 44 ms for `std` and 10 ms for
   ordstack at 120k records.
5. **Source cost is large where construction goes through serde.** The
   `serde_json_xform` port is 4.8x the original's code lines (56 → 269,
   +380%), almost all of it hand-written `DeserializeSeed`s replacing two
   derives. bumpalo needs the same machinery (235 lines, +320%), so this is
   the price of any arena under serde rather than of ordstack in particular,
   but it is part of the answer: under the brief's own caveat a 30% speedup
   bought with 4x the code is a qualified win, and mimalloc buys 34% with
   two lines. `treebuild` costs +83% code lines (47 → 86) against
   bumpalo's +43%.

## Setup

- Machine: AMD EPYC 7K62 (2 sockets × 48 cores, SMT off, 96 CPUs online),
  503 GB RAM, Linux 6.8.0-138-generic (Ubuntu 22.04), governor
  `schedutil`, THP `madvise`. The machine is shared: other users' jobs ran
  throughout (see "Load and re-runs").
- Toolchain on the benchmark machine: rustc 1.97.1 (LLVM 22.1.6), cargo
  1.97.1; glibc 2.35; perf 6.8.12 (`perf_event_paranoid` = 1); Python
  3.10.12. (The dev box only ran `cargo check`, with rustc 1.94.1.)
- Crates (`bench/Cargo.lock`): ordstack 0.1.0 (path, branch `bench`),
  bumpalo 3.20.3 (`collections`, `boxed`), mimalloc 0.1.52
  (libmimalloc-sys 0.1.49, default features), serde 1.0.229, serde_json
  1.0.154. `nm` confirms only the `std-mimalloc` binaries contain mimalloc.
- Profile: `release` (opt-level 3, no LTO, default codegen units), all
  variants, same compiler. Edition 2021 for the bench crates (as the
  originals).
- Inputs: `treebuild 7` (97,656 nodes) and `treebuild 8` (488,281 nodes);
  `serde_json_xform` on `big.json` (seed 1, 120,000 records, 14,677,152
  bytes, sha256 `4a156501…`) and `big4.json` (seed 1, 480,000 records,
  59,694,615 bytes, sha256 `3c091360…`), from `gen_json.py`.
- Method: per input, 3 warm-up rounds then 31 timed rounds; each round runs
  every variant once in an order rotated by one each round; every run of a
  case pinned to one CPU chosen as the least busy (never CPU 0). Wall time
  around spawn + wait4; user+sys and minor faults from rusage; peak RSS
  from `/usr/bin/time -f %M`, 5 rotated runs (`rss.py`; see Deviations
  for why not wait4); `perf stat -e instructions:u,instructions,cycles`, 7
  rotated runs. Every timed run's stdout was checked against `std`; every
  variant's output file was checked byte-identical once per input
  (14,986,734 bytes at 120k, 60,436,263 at 480k; the harness exits
  non-zero on a mismatch).
- Raw data: `results/results-release.json` (the reading),
  `results/results-release-lto.json`, `results/env-*.txt`, and the earlier
  runs in `results/prelim/` and `results/loaded/`.

## Load and re-runs

Other jobs were running on the machine for the whole session, so the
timed numbers come from the third full run:

| run | when (UTC) | loadavg | outcome |
|---|---|---|---|
| preliminary (`results/prelim/`) | 04:49–04:56 | 45–60 | fixed CPU 11 (not checked for idleness), before the owner's note; superseded |
| loaded (`results/loaded/`) | 04:59–05:13 | 45–68 | idle-CPU selection, but every case failed the 4% IQR bar three times; the culprit was `std-mimalloc` alone (e.g. treebuild 7: median 351 ms, range 152–2,141 ms, minor faults 10,518 against 333 when quiet); every other variant stayed within about 4%. Superseded |
| **reported** (`results/results-release.json`) | 05:24–05:31 | 12 → 5 | all cases passed: treebuild 7 on its second attempt (first 4.2%, second 2.1%), the others first time (1.4%, 1.5%, 0.9%); machine 4–8% busy during the cases |
| LTO sensitivity (`results/results-release-lto.json`) | 05:13–05:24 | 26 → 9 | treebuild 7 and 8 each needed a second attempt; final worst IQRs 3.3%, 3.2%, 0.9%, 1.6% |

The machine has SMT disabled, so there were no sibling threads to avoid.
Per-case load (loadavg before and after, machine busy %) is listed above
each table in `results/summary-*.md`. `uptime`, `free` and per-CPU `mpstat`
before and after the timed runs are in `results/env-release.txt`. The
conclusions of the superseded runs agree with the reported one for every
variant except `std-mimalloc`. I did not identify the cause of mimalloc's
slowdown under load. User+sys time equalled wall time, so the cost was CPU
work on mimalloc's behalf, not waiting, and its fault count rose 30-fold.

## Tables (release profile, the reading)

Wall time in ms over 31 runs; RSS in MB (`/usr/bin/time`, 5 runs; values
are identical across runs within 1 MB). `vs std` compares medians.

### treebuild 7

| variant | wall median | IQR | min–max | vs std | user+sys | peak RSS | vs std | minor faults | instructions:u (M) | cycles (M) |
|---|---|---|---|---|---|---|---|---|---|---|
| std | 95.8 | 95.4–96.2 | 94.3–97.6 | — | 94.6 | 47.2 | — | 12,095 | 380.8 | 286.5 |
| std-mimalloc | 56.1 | 55.9–57.1 | 55.4–58.5 | −41.4% | 55.0 | 51.7 | +9.4% | 333 | 283.3 | 173.8 |
| std-control | 83.9 | 83.4–84.6 | 82.2–86.8 | −12.4% | 82.7 | 42.8 | −9.5% | 10,951 | 348.1 | 246.6 |
| ordstack | 33.6 | 33.3–34.0 | 32.5–34.3 | −64.9% | 32.6 | 16.5 | −65.1% | 4,095 | 167.5 | 98.1 |
| bumpalo | 53.3 | 53.1–53.7 | 52.8–54.2 | −44.3% | 52.3 | 27.8 | −41.3% | 7,106 | 169.8 | 167.9 |

### treebuild 8

| variant | wall median | IQR | min–max | vs std | user+sys | peak RSS | vs std | minor faults | instructions:u (M) | cycles (M) |
|---|---|---|---|---|---|---|---|---|---|---|
| std | 468.0 | 466.3–470.0 | 462.9–475.0 | — | 466.7 | 237.8 | — | 60,922 | 1,908.2 | 1,436.0 |
| std-mimalloc | 267.9 | 267.3–268.3 | 260.0–276.9 | −42.8% | 266.8 | 235.7 | −0.9% | 430 | 1,415.0 | 862.3 |
| std-control | 407.1 | 403.8–409.3 | 400.5–416.1 | −13.0% | 406.0 | 216.0 | −9.1% | 55,200 | 1,744.5 | 1,242.5 |
| ordstack | 164.7 | 164.0–166.0 | 162.7–170.4 | −64.8% | 163.7 | 78.8 | −66.9% | 20,115 | 837.4 | 536.1 |
| bumpalo | 257.9 | 256.7–259.4 | 250.1–263.0 | −44.9% | 256.9 | 138.0 | −42.0% | 35,333 | 848.8 | 837.3 |

### serde_json_xform, 120,000 records

| variant | wall median | IQR | min–max | vs std | user+sys | peak RSS | vs std | minor faults | instructions:u (M) | cycles (M) |
|---|---|---|---|---|---|---|---|---|---|---|
| std | 293.6 | 292.2–296.6 | 287.6–301.3 | — | 292.4 | 57.8 | — | 14,792 | 1,632.2 | 934.5 |
| std-mimalloc | 193.9 | 193.1–195.9 | 191.5–198.6 | −34.0% | 192.8 | 55.7 | −3.6% | 339 | 1,331.4 | 626.3 |
| std-control | 291.2 | 289.8–292.1 | 287.0–300.7 | −0.8% | 290.1 | 57.8 | 0.0% | 14,792 | 1,600.0 | 930.8 |
| ordstack | 204.5 | 203.7–205.6 | 201.3–210.6 | −30.4% | 203.3 | 33.8 | −41.6% | 8,555 | 1,313.0 | 662.0 |
| bumpalo | 212.2 | 210.9–213.7 | 208.6–215.9 | −27.7% | 211.1 | 51.0 | −11.7% | 13,010 | 1,285.9 | 683.7 |

Phase timer (medians, ms; `rest` = wall minus the three phases, i.e.
process start, teardown and exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 149.3 | 58.4 | 42.4 | 44.1 |
| std-mimalloc | 113.0 | 32.1 | 39.3 | 9.8 |
| std-control | 147.8 | 55.3 | 42.9 | 44.9 |
| ordstack | 118.5 | 38.8 | 36.9 | 10.3 |
| bumpalo | 122.7 | 41.9 | 39.5 | 8.0 |

### serde_json_xform, 480,000 records

| variant | wall median | IQR | min–max | vs std | user+sys | peak RSS | vs std | minor faults | instructions:u (M) | cycles (M) |
|---|---|---|---|---|---|---|---|---|---|---|
| std | 1,164.6 | 1,162.1–1,170.3 | 1,152.9–1,174.9 | — | 1,162.2 | 226.5 | — | 58,019 | 6,322.4 | 3,795.9 |
| std-mimalloc | 763.7 | 759.1–766.0 | 754.4–769.5 | −34.4% | 762.0 | 193.7 | −14.5% | 412 | 5,349.2 | 2,513.8 |
| std-control | 1,150.5 | 1,145.9–1,153.1 | 1,139.9–1,161.3 | −1.2% | 1,148.6 | 226.5 | 0.0% | 58,019 | 6,193.1 | 3,727.2 |
| ordstack | 804.1 | 802.1–806.6 | 798.6–813.9 | −31.0% | 802.2 | 128.2 | −43.4% | 32,994 | 5,276.0 | 2,641.9 |
| bumpalo | 833.4 | 832.0–836.0 | 828.1–842.0 | −28.4% | 831.8 | 198.0 | −12.6% | 50,818 | 5,167.2 | 2,751.5 |

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 598.4 | 251.4 | 230.6 | 84.5 |
| std-mimalloc | 453.7 | 126.1 | 161.6 | 21.6 |
| std-control | 590.8 | 238.3 | 232.2 | 89.2 |
| ordstack | 471.6 | 153.9 | 151.0 | 27.8 |
| bumpalo | 489.4 | 165.6 | 161.3 | 17.7 |

Parse dominates (50–55% of wall). The allocation-heavy transform phase is
where std loses most (58 → 39 ms with ordstack). mimalloc is fastest
there (32 ms). ordstack's write phase is the fastest of all variants,
plausibly from locality: the records and their strings are contiguous.

All per-variant perf ranges (min–max over 7 runs), and the plain
`instructions` count (user + kernel), are in `results/summary-release.md`.

### LTO sensitivity check (`release-lto`: fat LTO, one codegen unit)

| input | std | ordstack | bumpalo | ordstack vs std | ordstack vs bumpalo |
|---|---|---|---|---|---|
| treebuild 7 | 93.0 | 35.3 | 51.0 | −62.0% | −30.8% |
| treebuild 8 | 468.1 | 171.0 | 249.9 | −63.5% | −31.6% |
| xform 120k | 295.0 | 184.4 | 196.4 | −37.5% | −6.1% |
| xform 480k | 1,140.0 | 719.0 | 768.0 | −36.9% | −6.4% |

(Median wall ms.) The reading is unchanged under LTO. LTO helps the
`serde_json_xform` ports by about 10% (ordstack's user instructions
−7.4%, bumpalo's −5.3%) and leaves std unchanged. So some of ordstack's
non-generic internals (`Stack::try_bump`, `reset_to`, `offset_of`, none
marked `#[inline]`) may be paying a cross-crate call cost, but the effect
is small and close to bumpalo's own LTO gain. `treebuild` does not
benefit. Full tables: `results/summary-release-lto.md`.

## Where ordstack differs from a plain arena

From `Stack::high_water` / `Stack::extent` (one run, `ORDSTACK_STATS=1`):

| input | stack | high water | held at teardown | note |
|---|---|---|---|---|
| treebuild 7 | tree | 15.59 MB | 15.59 MB | never reclaimed before teardown |
| treebuild 7 | scratch | 256 B | — | per-level children vectors, popped as they go |
| treebuild 8 | tree / scratch | 81.22 MB / 288 B | 81.22 MB | same shape |
| xform 120k | recs | 9.44 MB | 0 (reset) | record array; grew in place by doubling to 9.4 MB, no copy |
| xform 120k | data | 9.00 MB | 0 (reset) | parsed strings and arrays |
| xform 120k | out | 14.76 MB | 14.76 MB | output records |
| xform 480k | recs / data / out | 37.75 / 35.94 / 58.98 MB | 0 / 0 / 58.98 MB | same shape |

- `serde_json_xform`: 55% of the port's stack bytes (high-water sum: 18.4
  of 33.2 MB at 120k, 73.7 of 132.7 MB at 480k) were reclaimed before
  teardown, by the explicit `Stack::reset` after the transform. The largest
  single block, the record array (28% of the bytes), grew in place through
  every doubling without a copy. bumpalo's `recs` arena, set up the same
  way, held 33.5 MB of chunks for the same 9.4 MB array, because growth
  across a chunk boundary copies into a new chunk.
- `treebuild`: effectively 0% reclaimed before teardown. Except for 256
  bytes of scratch, ordstack behaves as an arena with exact-size blocks
  here. Its advantage over bumpalo on this program is that a growing
  container is never copied: the children vector grows on a second stack
  and is copied once, at exact size.

The library exposes no counters for in-place grows or bytes copied, so the
"grew in place" share is the record array's capacity, not a measured
total. The stackfit spike measured these with its instrumented core; its
numbers have the same shape.

## Source cost

`srcstats.py`. Lines marked `BENCH` (instrumentation, including the phase
timer that the serde original also carries) are excluded. Every file is
passed through rustfmt first, so the original and the ports are formatted
alike. "Code lines" are non-blank lines that are not comments.

| workload | file | lines | code lines | code lines added / removed | lifetime params declared (types / fns / impl headers) | lifetime tokens | PORT notes |
|---|---|---|---|---|---|---|---|
| treebuild | original | 65 | 47 | — | 0 / 0 / 0 | 0 | — |
| treebuild | std-control | 85 | 59 (+26%) | +19 / −7 | 0 / 0 / 0 | 0 | — |
| treebuild | **ordstack** | 134 | 86 (+83%) | +60 / −21 | 1 / 1 / 1 | 20 | 10 |
| treebuild | bumpalo | 97 | 67 (+43%) | +41 / −21 | 1 / 1 / 1 | 18 | 5 |
| serde_json_xform | original | 74 | 56 | — | 0 / 0 / 0 | 0 | — |
| serde_json_xform | std-control | 88 | 62 (+11%) | +8 / −2 | 0 / 0 / 0 | 0 | — |
| serde_json_xform | **ordstack** | 352 | 269 (+380%) | +239 / −26 | 19 / 0 / 22 | 141 | 13 |
| serde_json_xform | bumpalo | 305 | 235 (+320%) | +204 / −25 | 11 / 0 / 22 | 103 | 10 |

Stacks or arenas used: treebuild 2 stacks (ordstack) vs 1 bump; serde 3
stacks vs 3 bumps (the bumpalo port mirrors the ordstack layout on
purpose).

## API workarounds (feedback for 0.2)

Each is marked `PORT:` in the ports.

1. **No `format!` twin.** Each `format!` became `SString::new_in` +
   `write!` + `freeze` (3 lines for 1). `SString: fmt::Write` made this
   possible; an `sformat!(frame, ...)` returning `SStr` would remove it
   (treebuild ×2).
2. **No map.** `HashMap<String, Vec<i32>>` became a sorted `SVec` of
   pairs, frozen, plus a 6-line `insert_sorted` helper. Its cost and
   benefit were measured separately (`std-control`).
3. **Iteration gaps.** `&SSlice` is not `IntoIterator`
   (`for c in &node.children` must be `.iter()`), and `SVec` is not
   `IntoIterator` by value (`recs.into_iter()` became
   `recs.freeze().into_iter()`).
4. **No serde support.** Serializing needed two `serialize_with` helpers
   (`SStr`, `SSlice<SStr>`); a `serde` feature implementing `Serialize`
   for `SStr`, `SSlice`, `SBox` and `SFixedVec` would remove them and allow
   `to_writer(&out)`. Deserializing needs hand-written `DeserializeSeed`s
   (~170 lines), because `Deserialize` has no context parameter. That is
   inherent to serde, but a seed-generating derive or a small set of stock
   seeds (`StrSeed`, `SeqSeed<T>`) is the single biggest lever on source
   cost.
5. **No `*_in` twins for allocating std methods.** `r.tags.clone()` became
   an explicit `SFixedVec` + `alloc_str` loop (`alloc_slice_clone` cannot
   help: `SStr` is not `Clone`, since cloning it needs a frame);
   `to_uppercase()` became `extend(chars().flat_map(char::to_uppercase))`.
   A `Frame::alloc_slice_from_iter` for exact-size iterators would cover
   the first.
6. **Stack statistics are unreadable while anything on the stack is
   alive.** `Stack::frame` borrows the stack mutably for the whole brand, so
   `high_water()` / `extent()` can only be read after every block is gone
   (treebuild has to `drop(root)` first). A read-only accessor on `Frame`
   (e.g. `Frame::stack_high_water()`) would allow mid-run measurement. There
   are no counters for in-place grows or bytes copied.
7. **Lazy reclamation needs an explicit `Stack::reset`** to drop the parsed
   data before writing (2 lines). It lowers the bump pointer but does not
   return pages; `trim` would. Here reset did not lower peak RSS, because
   the `out` stack grows on its own pages.
8. **A growing child container needs a second stack and a copy** (treebuild's
   `children`: grows on `scratch`, then `freeze_into(tree)`). This is the
   design, not a gap, but it is the main structural change in the port.
9. **4 GiB of address space per `Stack::new()`.** Three stacks reserve
   12 GiB; this ran under `ulimit -v 32000000`, but a tighter address-space
   limit (e.g. 8 GB) would make the serde port fail at start-up.
   `with_reserve` exists but needs a size guess.
10. **No `#[inline]` on the non-generic hot paths** (`try_bump`, `reset_to`,
    `offset_of`, `ptr_at`). The LTO check suggests a small effect at most.

## Deviations from the brief, and caveats

- **`std` is not byte-for-byte the original for `serde_json_xform`.** The
  phase timer (four `// BENCH` lines, active only with `XFORM_PHASES` set)
  is in the shared `orig.rs`, so every variant carries the same timer.
  `treebuild`'s `orig.rs` is the unchanged original.
- **An extra variant, `std-control`**, isolates the ports' non-allocator
  changes (sorted pairs for the map in treebuild; a reused bucket-key buffer
  in serde), marked `// CONTROL`.
- **Pinning** uses `sched_setaffinity` in the child before `exec`
  (Python), not the `taskset` binary; the effect is the same.
- **Peak RSS is from `/usr/bin/time -f %M`, 5 runs**, not from the timed
  runs' `wait4`. `ru_maxrss` of a child that Python forks includes the
  interpreter's pages from before `exec`, which floored every reading at
  about 20 MB. That floor hid ordstack's 16 MB at treebuild 7 (reported as
  22.3 MB). The timed-run figures stay in the JSON; every other cell
  agrees within about 1 MB.
- **The bumpalo `serde_json_xform` port mirrors ordstack's three-arena
  layout** (and resets two of them), rather than using one arena. That is
  the fairest comparison of allocation strategy, but it is more structure
  than a typical bumpalo user would write.
- **`by_bucket` stays a heap `BTreeMap` in both arena ports**: its vectors
  grow round-robin, which a stack cannot express and which bumpalo would
  copy repeatedly.
- **Peak RSS includes the input file buffer** (`serde_json::from_reader`
  over a `BufReader` streams, so this is small) and the output buffer.
- **The two superseded runs** (`prelim/`, `loaded/`) were taken under heavy
  external load and are kept only as a record; the reading uses the clean
  run.
- **Not done:** a run on a dedicated, unshared machine, and
  hardware-counter breakdowns beyond instructions and cycles (cache
  misses, dTLB). Under `perf_event_paranoid` = 1 these were available, but
  were not pre-registered and are not needed for the reading.

## Addendum 2026-10-14: `#[inline]` on the hot paths (0.2a)

Rule, fixed before the runs: keep `#[inline]` if, on every case, the
ratio of ordstack's median wall time to std's (same run) rises by no more
than 2% and ordstack's user-mode instructions rise by no more than 2%.

Both runs: `RUNS=15 PERF_RUNS=3`. The `#[inline]` attributes went on
`Stack::{set_top, reset_to, try_bump, try_extend, ptr_at, offset_of}` and
`Region::{base, len}`.

| case | ratio before | ratio after | instructions:u before (M) | after (M) | load (before / after) |
|---|---|---|---|---|---|
| treebuild-d7 | 0.363 | 0.376 (+3.6%) | 167.5 | 170.0 (+1.5%) | 144 / 152 |
| treebuild-d8 | 0.359 | 0.365 (+1.7%) | 837.4 | 849.7 (+1.5%) | 144 / 152 |
| xform-120k | 0.629 | 0.690 (+9.7%) | 120.9 | 115.7 (−4.3%) | 144 / 152 |
| xform-480k | 0.679 | 0.673 (−0.9%) | 482.1 | 465.1 (−3.5%) | 144 / 152 |

Load is the 1-minute load average at the start of each run, on 96 cores
shared with other jobs. Every case except treebuild-d7 (after) exceeded
the harness's 4% wall-IQR noise bar on all three attempts; the reported
attempts' worst IQRs were 5.9 / 22.0 / 37.3 / 23.5% before and 3.6 / 15.4
/ 27.2 / 9.7% after. The wall-time differences are therefore within
noise. Instructions rose by about 1.5% on the tree-building cases and fell
by about 4% on the serde cases.

Decision: reverted. treebuild-d7 and xform-120k broke the wall-time rule,
and the instruction counts, the noise-robust measure, show no consistent
gain. Within the crate the compiler can already inline these small
non-generic functions, so the attribute buys little. Revisit on a quieter
machine if a profile shows the calls.
