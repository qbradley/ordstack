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

(Numbers follow below once measured.)
