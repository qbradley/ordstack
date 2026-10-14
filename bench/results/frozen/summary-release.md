Runs per (variant, input): 25 timed after 3 warm-up rounds, every run of a case pinned to one CPU. Re-run rule: worst variant's wall IQR > 4% of its median → the case is re-run on a fresh core.

### treebuild-d7 (`treebuild 7`)

- attempt 1: CPU 64; worst wall IQR 3.6% of median; loadavg 33.28/18.63/11.40 → 32.92/19.04/11.61; chosen core busy before {'64': 0.0}; SMT sibling busy during {}; machine busy before 20.8% / during 27.4% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 96.3 | 96.0–97.2 | 95.0–100.4 | +0.0% | 95.2 | 47.2 | 47.2–47.2 | +0.0% | 12087 |
| std-mimalloc | 88.3 | 86.4–89.6 | 76.7–91.7 | -8.3% | 87.1 | 51.7 | 51.7–51.7 | +9.4% | 326 |
| std-control | 84.4 | 83.9–85.2 | 83.1–89.2 | -12.3% | 83.3 | 42.8 | 42.8–42.8 | -9.5% | 10943 |
| ordstack | 34.5 | 34.2–34.8 | 33.7–35.4 | -64.2% | 33.5 | 15.8 | 15.8–16.5 | -66.7% | 4087 |
| bumpalo | 54.5 | 54.2–54.7 | 53.5–55.4 | -43.4% | 53.4 | 27.8 | 27.8–27.8 | -41.3% | 7098 |
| bumpalo-frozen | 36.1 | 35.6–36.6 | 35.4–37.5 | -62.5% | 35.1 | 18.8 | 18.8–18.8 | -60.3% | 4811 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 67; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 380.8 | 380.8–380.8 | +0.0% | 437.9 | 437.9–438.5 | +0.0% | 289.9 | 286.3–298.5 | +0.0% |
| std-mimalloc | 283.3 | 283.3–283.3 | -25.6% | 375.9 | 328.8–387.1 | -14.2% | 273.5 | 240.5–306.7 | -5.7% |
| std-control | 348.1 | 348.1–348.1 | -8.6% | 399.8 | 399.7–400.3 | -8.7% | 255.0 | 245.6–264.1 | -12.0% |
| ordstack | 167.5 | 167.5–167.5 | -56.0% | 192.0 | 192.0–192.1 | -56.2% | 100.9 | 100.1–102.8 | -65.2% |
| bumpalo | 169.8 | 169.8–169.8 | -55.4% | 212.5 | 212.5–212.8 | -51.5% | 170.9 | 167.0–174.0 | -41.0% |
| bumpalo-frozen | 152.0 | 152.0–152.0 | -60.1% | 180.8 | 180.8–180.8 | -58.7% | 107.4 | 104.6–109.0 | -62.9% |

### treebuild-d8 (`treebuild 8`)

- attempt 1: CPU 86; worst wall IQR 2.2% of median; loadavg 32.93/19.27/11.72 → 36.59/22.52/13.27; chosen core busy before {'86': 1.7}; SMT sibling busy during {}; machine busy before 32.1% / during 34.8% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 481.4 | 480.5–484.2 | 476.1–490.3 | +0.0% | 480.4 | 237.8 | 237.8–237.8 | +0.0% | 60921 |
| std-mimalloc | 590.3 | 580.7–594.0 | 531.8–599.9 | +22.6% | 588.5 | 235.7 | 235.7–235.7 | -0.9% | 428 |
| std-control | 414.1 | 411.3–416.9 | 407.5–424.7 | -14.0% | 413.1 | 216.0 | 216.0–216.0 | -9.1% | 55198 |
| ordstack | 168.5 | 167.4–168.9 | 165.3–171.9 | -65.0% | 167.5 | 78.8 | 78.8–78.8 | -66.9% | 20114 |
| bumpalo | 265.1 | 263.8–266.8 | 257.9–270.3 | -44.9% | 264.0 | 138.0 | 138.0–138.0 | -42.0% | 35332 |
| bumpalo-frozen | 171.8 | 170.2–173.0 | 167.0–179.3 | -64.3% | 170.7 | 93.8 | 93.8–93.8 | -60.6% | 23889 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 83; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1908.2 | 1908.2–1908.2 | +0.0% | 2196.7 | 2196.4–2199.5 | +0.0% | 1499.7 | 1478.5–1529.1 | +0.0% |
| std-mimalloc | 1415.0 | 1415.0–1415.0 | -25.8% | 2530.5 | 2529.9–2541.0 | +15.2% | 1892.4 | 1869.8–1920.7 | +26.2% |
| std-control | 1744.5 | 1744.5–1744.5 | -8.6% | 2005.2 | 2005.1–2005.6 | -8.7% | 1284.0 | 1272.3–1297.3 | -14.4% |
| ordstack | 837.4 | 837.4–837.4 | -56.1% | 958.8 | 958.7–960.5 | -56.4% | 545.6 | 542.8–550.6 | -63.6% |
| bumpalo | 848.8 | 848.8–848.8 | -55.5% | 1062.3 | 1062.2–1063.8 | -51.6% | 871.3 | 867.0–881.0 | -41.9% |
| bumpalo-frozen | 759.5 | 759.5–759.5 | -60.2% | 903.8 | 903.8–905.1 | -58.9% | 555.2 | 546.8–564.3 | -63.0% |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | instructions:u vs std / bumpalo | cycles vs std / bumpalo | time vs bumpalo-frozen | RSS vs bumpalo-frozen |
|---|---|---|---|---|---|---|---|---|---|---|---|
| treebuild-d7 | -64.2% | yes | yes | -66.7% | yes | -36.7% | -43.2% | -56.0% / -1.3% | -65.2% / -41.0% | -4.5% | -16.0% |
| treebuild-d8 | -65.0% | yes | yes | -66.9% | yes | -36.4% | -42.9% | -56.1% / -1.3% | -63.6% / -37.4% | -1.9% | -16.0% |

- treebuild: time criterion at both sizes: met; RSS criterion at both sizes: met

### Arena statistics (ORDSTACK_STATS=1, one run)

- treebuild-d7/ordstack: stack tree high_water=15592336 extent=15592336; stack scratch high_water=256 extent=64
- treebuild-d7/bumpalo: bump allocated_bytes=33552896
- treebuild-d7/bumpalo-frozen: bump allocated_bytes=33552896
- treebuild-d8/ordstack: stack tree high_water=81217336 extent=81217336; stack scratch high_water=288 extent=64
- treebuild-d8/bumpalo: bump allocated_bytes=268433728
- treebuild-d8/bumpalo-frozen: bump allocated_bytes=134216064

