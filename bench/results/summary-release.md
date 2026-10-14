Runs per (variant, input): 31 timed after 3 warm-up rounds, every run of a case pinned to one CPU. Re-run rule: worst variant's wall IQR > 4% of its median → the case is re-run on a fresh core.

### treebuild-d7 (`treebuild 7`)

- attempt 1: CPU 9; worst wall IQR 4.2% of median; loadavg 11.43/16.08/28.06 → 10.91/15.81/27.84; chosen core busy before {'9': 0.0}; SMT sibling busy during {}; machine busy before 7.2% / during 8.1%
- attempt 2: CPU 9; worst wall IQR 2.1% of median; loadavg 10.59/15.67/27.73 → 10.24/15.43/27.52; chosen core busy before {'9': 0.0}; SMT sibling busy during {}; machine busy before 6.8% / during 6.4% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 95.8 | 95.4–96.2 | 94.3–97.6 | +0.0% | 94.6 | 47.2 | 47.2–47.2 | +0.0% | 12095 |
| std-mimalloc | 56.1 | 55.9–57.1 | 55.4–58.5 | -41.4% | 55.0 | 51.7 | 51.7–51.7 | +9.4% | 333 |
| std-control | 83.9 | 83.4–84.6 | 82.2–86.8 | -12.4% | 82.7 | 42.8 | 42.8–42.8 | -9.5% | 10951 |
| ordstack | 33.6 | 33.3–34.0 | 32.5–34.3 | -64.9% | 32.6 | 16.5 | 15.8–16.5 | -65.1% | 4095 |
| bumpalo | 53.3 | 53.1–53.7 | 52.8–54.2 | -44.3% | 52.3 | 27.8 | 27.8–27.8 | -41.3% | 7106 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 4; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 380.8 | 380.8–380.8 | +0.0% | 438.1 | 437.9–438.7 | +0.0% | 286.5 | 285.7–290.4 | +0.0% |
| std-mimalloc | 283.3 | 283.3–283.3 | -25.6% | 287.9 | 287.8–288.0 | -34.3% | 173.8 | 173.0–175.2 | -39.3% |
| std-control | 348.1 | 348.1–348.1 | -8.6% | 399.9 | 399.9–400.0 | -8.7% | 246.6 | 244.1–250.4 | -13.9% |
| ordstack | 167.5 | 167.5–167.5 | -56.0% | 192.0 | 192.0–192.1 | -56.2% | 98.1 | 97.4–99.0 | -65.8% |
| bumpalo | 169.8 | 169.8–169.8 | -55.4% | 212.6 | 212.4–212.7 | -51.5% | 167.9 | 167.1–168.3 | -41.4% |

### treebuild-d8 (`treebuild 8`)

- attempt 1: CPU 3; worst wall IQR 1.4% of median; loadavg 9.74/15.24/27.40 → 6.83/13.60/26.20; chosen core busy before {'3': 0.0}; SMT sibling busy during {}; machine busy before 3.7% / during 4.8% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 468.0 | 466.3–470.0 | 462.9–475.0 | +0.0% | 466.7 | 237.8 | 237.8–237.8 | +0.0% | 60922 |
| std-mimalloc | 267.9 | 267.3–268.3 | 260.0–276.9 | -42.8% | 266.8 | 235.7 | 235.7–235.7 | -0.9% | 430 |
| std-control | 407.1 | 403.8–409.3 | 400.5–416.1 | -13.0% | 406.0 | 216.0 | 216.0–216.0 | -9.1% | 55200 |
| ordstack | 164.7 | 164.0–166.0 | 162.7–170.4 | -64.8% | 163.7 | 78.8 | 78.8–78.8 | -66.9% | 20115 |
| bumpalo | 257.9 | 256.7–259.4 | 250.1–263.0 | -44.9% | 256.9 | 138.0 | 138.0–138.0 | -42.0% | 35333 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 1; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1908.2 | 1908.2–1908.2 | +0.0% | 2198.6 | 2197.5–2200.8 | +0.0% | 1436.0 | 1415.0–1448.8 | +0.0% |
| std-mimalloc | 1415.0 | 1415.0–1415.0 | -25.8% | 1430.5 | 1430.4–1430.9 | -34.9% | 862.3 | 856.1–868.9 | -40.0% |
| std-control | 1744.5 | 1744.5–1744.5 | -8.6% | 2006.6 | 2006.4–2007.3 | -8.7% | 1242.5 | 1223.1–1318.2 | -13.5% |
| ordstack | 837.4 | 837.4–837.4 | -56.1% | 959.3 | 959.2–959.5 | -56.4% | 536.1 | 529.9–543.5 | -62.7% |
| bumpalo | 848.8 | 848.8–848.8 | -55.5% | 1063.3 | 1063.0–1064.4 | -51.6% | 837.3 | 829.9–868.1 | -41.7% |

### xform-120k (`serde_json_xform big.json out_std.json`)

- attempt 1: CPU 1; worst wall IQR 1.5% of median; loadavg 6.53/13.42/26.08 → 5.63/12.32/25.17; chosen core busy before {'1': 0.0}; SMT sibling busy during {}; machine busy before 3.9% / during 4.8% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 293.6 | 292.2–296.6 | 287.6–301.3 | +0.0% | 292.4 | 57.8 | 57.8–57.8 | +0.0% | 14792 |
| std-mimalloc | 193.9 | 193.1–195.9 | 191.5–198.6 | -34.0% | 192.8 | 55.7 | 55.7–55.7 | -3.6% | 339 |
| std-control | 291.2 | 289.8–292.1 | 287.0–300.7 | -0.8% | 290.1 | 57.8 | 57.8–57.8 | +0.0% | 14792 |
| ordstack | 204.5 | 203.7–205.6 | 201.3–210.6 | -30.4% | 203.3 | 33.8 | 33.8–33.8 | -41.6% | 8555 |
| bumpalo | 212.2 | 210.9–213.7 | 208.6–215.9 | -27.7% | 211.1 | 51.0 | 51.0–51.0 | -11.7% | 13010 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 149.3 | 58.4 | 42.4 | 44.1 |
| std-mimalloc | 113.0 | 32.1 | 39.3 | 9.8 |
| std-control | 147.8 | 55.3 | 42.9 | 44.9 |
| ordstack | 118.5 | 38.8 | 36.9 | 10.3 |
| bumpalo | 122.7 | 41.9 | 39.5 | 8.0 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 2; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1632.2 | 1632.2–1632.2 | +0.0% | 1748.1 | 1747.8–1780.1 | +0.0% | 934.5 | 927.5–960.1 | +0.0% |
| std-mimalloc | 1331.4 | 1331.4–1331.4 | -18.4% | 1374.7 | 1374.4–1405.7 | -21.4% | 626.3 | 621.2–656.2 | -33.0% |
| std-control | 1600.0 | 1600.0–1600.0 | -2.0% | 1715.9 | 1715.7–1747.7 | -1.8% | 930.8 | 924.6–971.3 | -0.4% |
| ordstack | 1313.0 | 1313.0–1313.0 | -19.6% | 1402.6 | 1402.5–1436.9 | -19.8% | 662.0 | 654.7–696.0 | -29.2% |
| bumpalo | 1285.9 | 1285.9–1285.9 | -21.2% | 1402.4 | 1402.3–1436.0 | -19.8% | 683.7 | 678.3–724.9 | -26.8% |

### xform-480k (`serde_json_xform big4.json out_std.json`)

- attempt 1: CPU 2; worst wall IQR 0.9% of median; loadavg 5.58/12.20/25.06 → 5.05/9.18/21.87; chosen core busy before {'2': 0.0}; SMT sibling busy during {}; machine busy before 3.7% / during 5.1% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 1164.6 | 1162.1–1170.3 | 1152.9–1174.9 | +0.0% | 1162.2 | 226.5 | 226.5–226.5 | +0.0% | 58019 |
| std-mimalloc | 763.7 | 759.1–766.0 | 754.4–769.5 | -34.4% | 762.0 | 193.7 | 193.7–193.7 | -14.5% | 412 |
| std-control | 1150.5 | 1145.9–1153.1 | 1139.9–1161.3 | -1.2% | 1148.6 | 226.5 | 226.5–226.5 | +0.0% | 58019 |
| ordstack | 804.1 | 802.1–806.6 | 798.6–813.9 | -31.0% | 802.2 | 128.2 | 128.2–128.2 | -43.4% | 32994 |
| bumpalo | 833.4 | 832.0–836.0 | 828.1–842.0 | -28.4% | 831.8 | 198.0 | 198.0–198.0 | -12.6% | 50818 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 598.4 | 251.4 | 230.6 | 84.5 |
| std-mimalloc | 453.7 | 126.1 | 161.6 | 21.6 |
| std-control | 590.8 | 238.3 | 232.2 | 89.2 |
| ordstack | 471.6 | 153.9 | 151.0 | 27.8 |
| bumpalo | 489.4 | 165.6 | 161.3 | 17.7 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 1; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 6322.4 | 6322.4–6322.4 | +0.0% | 6791.7 | 6755.8–6816.8 | +0.0% | 3795.9 | 3703.4–3824.6 | +0.0% |
| std-mimalloc | 5349.2 | 5349.2–5349.2 | -15.4% | 5519.3 | 5486.2–5536.0 | -18.7% | 2513.8 | 2452.4–2562.6 | -33.8% |
| std-control | 6193.1 | 6193.1–6193.1 | -2.0% | 6662.1 | 6625.4–6679.9 | -1.9% | 3727.2 | 3677.6–3761.3 | -1.8% |
| ordstack | 5276.0 | 5276.0–5276.0 | -16.6% | 5635.9 | 5602.1–5663.0 | -17.0% | 2641.9 | 2616.0–2714.3 | -30.4% |
| bumpalo | 5167.2 | 5167.2–5167.2 | -18.3% | 5648.8 | 5599.9–5663.3 | -16.8% | 2751.5 | 2714.2–2795.4 | -27.5% |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | instructions:u vs std / bumpalo | cycles vs std / bumpalo |
|---|---|---|---|---|---|---|---|---|---|
| treebuild-d7 | -64.9% | yes | yes | -65.1% | yes | -37.0% | -40.5% | -56.0% / -1.3% | -65.8% / -41.6% |
| treebuild-d8 | -64.8% | yes | yes | -66.9% | yes | -36.2% | -42.9% | -56.1% / -1.3% | -62.7% / -36.0% |
| xform-120k | -30.4% | yes | yes | -41.6% | yes | -3.6% | -33.8% | -19.6% / +2.1% | -29.2% / -3.2% |
| xform-480k | -31.0% | yes | yes | -43.4% | yes | -3.5% | -35.2% | -16.6% / +2.1% | -30.4% / -4.0% |

- treebuild: time criterion at both sizes: met; RSS criterion at both sizes: met
- serde_json_xform: time criterion at both sizes: met; RSS criterion at both sizes: met

### Arena statistics (ORDSTACK_STATS=1, one run)

- treebuild-d7/ordstack: stack tree high_water=15592336 extent=15592336; stack scratch high_water=256 extent=64
- treebuild-d7/bumpalo: bump allocated_bytes=33552896
- treebuild-d8/ordstack: stack tree high_water=81217336 extent=81217336; stack scratch high_water=288 extent=64
- treebuild-d8/bumpalo: bump allocated_bytes=268433728
- xform-120k/ordstack: stack recs high_water=9437312 extent=8640000; stack data high_water=8997696 extent=8997684; recs_vec_capacity_bytes=9437184; stack out high_water=14757682 extent=14757679
- xform-120k/bumpalo: bump recs allocated_bytes=33552896; bump data allocated_bytes=33552896; bump out allocated_bytes=23052160
- xform-480k/ordstack: stack recs high_water=37748864 extent=34560000; stack data high_water=35942616 extent=35942604; recs_vec_capacity_bytes=37748736; stack out high_water=58982603 extent=58982600
- xform-480k/bumpalo: bump recs allocated_bytes=134216064; bump data allocated_bytes=134216064; bump out allocated_bytes=92172160

