Runs per (variant, input): 31 timed after 3 warm-up rounds, every run of a case pinned to one CPU. Re-run rule: worst variant's wall IQR > 4% of its median → the case is re-run on a fresh core.

### treebuild-d7 (`treebuild 7`)

- attempt 1: CPU 23; worst wall IQR 88.5% of median; loadavg 6.70/6.97/9.20 → 10.73/7.97/9.46; chosen core busy before {'23': 0.0}; SMT sibling busy during {}; machine busy before 17.0% / during 15.3% **(reported)**
- attempt 2: CPU 8; worst wall IQR 178.2% of median; loadavg 10.11/7.88/9.42 → 9.83/7.44/8.89; chosen core busy before {'8': 0.0}; SMT sibling busy during {}; machine busy before 3.6% / during 7.3%
- attempt 3: CPU 2; worst wall IQR 589.6% of median; loadavg 9.29/7.37/8.86 → 10.73/7.87/8.86; chosen core busy before {'2': 0.0}; SMT sibling busy during {}; machine busy before 12.1% / during 13.4%

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 100.8 | 95.6–110.3 | 91.1–143.4 | +0.0% | 99.8 | 47.2 | 47.2–47.2 | +0.0% | 12091 |
| std-mimalloc | 279.4 | 168.1–415.2 | 77.5–3531.3 | +177.3% | 277.8 | 51.7 | 51.7–51.7 | +9.4% | 329 |
| std-control | 85.6 | 83.4–93.5 | 78.4–110.1 | -15.0% | 84.9 | 42.8 | 42.8–42.8 | -9.5% | 10947 |
| ordstack | 34.0 | 32.7–34.8 | 32.1–38.8 | -66.2% | 33.1 | 16.5 | 15.8–16.5 | -65.1% | 4091 |
| ordstack-alloc | 91.4 | 85.7–97.3 | 80.1–110.9 | -9.3% | 90.7 | 51.0 | 51.0–51.0 | +7.9% | 13008 |
| bumpalo | 55.0 | 52.1–58.5 | 50.3–72.1 | -45.4% | 54.4 | 27.8 | 27.8–27.8 | -41.3% | 7102 |
| bumpalo-frozen | 36.5 | 35.2–37.0 | 34.0–45.9 | -63.8% | 35.5 | 18.8 | 18.8–18.8 | -60.3% | 4815 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 11; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 380.8 | 380.8–380.8 | +0.0% | 438.4 | 438.2–440.0 | +0.0% | 280.2 | 269.5–287.6 | +0.0% |
| std-mimalloc | 283.3 | 283.3–283.3 | -25.6% | 670.7 | 663.0–700.2 | +53.0% | 830.4 | 768.4–1179.4 | +196.3% |
| std-control | 348.1 | 348.1–348.1 | -8.6% | 400.2 | 399.9–401.0 | -8.7% | 245.4 | 236.9–262.2 | -12.4% |
| ordstack | 167.5 | 167.5–167.5 | -56.0% | 192.1 | 192.0–193.1 | -56.2% | 101.2 | 96.9–109.1 | -63.9% |
| ordstack-alloc | 248.6 | 248.6–248.6 | -34.7% | 328.3 | 327.3–330.6 | -25.1% | 280.8 | 262.3–301.4 | +0.2% |
| bumpalo | 169.8 | 169.8–169.8 | -55.4% | 212.7 | 212.5–214.2 | -51.5% | 163.3 | 156.3–179.3 | -41.7% |
| bumpalo-frozen | 152.0 | 152.0–152.0 | -60.1% | 181.0 | 180.9–181.9 | -58.7% | 106.1 | 101.6–108.3 | -62.1% |

### treebuild-d8 (`treebuild 8`)

- attempt 1: CPU 2; worst wall IQR 77.4% of median; loadavg 10.73/7.87/8.86 → 16.45/11.22/9.83; chosen core busy before {'2': 0.0}; SMT sibling busy during {}; machine busy before 17.1% / during 11.2%
- attempt 2: CPU 43; worst wall IQR 90.8% of median; loadavg 16.57/11.34/9.88 → 16.89/14.39/11.37; chosen core busy before {'43': 0.0}; SMT sibling busy during {}; machine busy before 16.4% / during 16.1%
- attempt 3: CPU 10; worst wall IQR 63.6% of median; loadavg 17.54/14.57/11.45 → 19.41/17.06/13.08; chosen core busy before {'10': 0.0}; SMT sibling busy during {}; machine busy before 21.8% / during 20.2% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 486.4 | 454.7–583.1 | 437.8–10104.7 | +0.0% | 485.0 | 237.8 | 237.8–237.8 | +0.0% | 60924 |
| std-mimalloc | 1519.4 | 1221.5–2187.3 | 592.8–15410.5 | +212.4% | 1517.4 | 235.7 | 235.6–235.7 | -0.9% | 433 |
| std-control | 421.3 | 409.4–466.6 | 388.3–8619.6 | -13.4% | 420.0 | 216.0 | 216.0–216.0 | -9.1% | 55202 |
| ordstack | 170.1 | 165.1–183.7 | 158.7–3425.8 | -65.0% | 169.2 | 78.8 | 78.8–78.8 | -66.9% | 20118 |
| ordstack-alloc | 445.3 | 416.6–511.3 | 397.8–7809.0 | -8.4% | 444.0 | 255.8 | 255.8–255.8 | +7.6% | 65503 |
| bumpalo | 265.9 | 249.6–305.3 | 241.9–5208.5 | -45.3% | 264.3 | 138.0 | 138.0–138.0 | -42.0% | 35336 |
| bumpalo-frozen | 174.8 | 166.8–192.5 | 160.5–3853.7 | -64.1% | 173.7 | 93.8 | 93.8–93.8 | -60.6% | 23893 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 3; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1908.2 | 1908.2–1908.2 | +0.0% | 2200.8 | 2199.4–2219.4 | +0.0% | 1769.8 | 1443.5–2244.8 | +0.0% |
| std-mimalloc | 1415.0 | 1415.0–1415.0 | -25.8% | 3531.9 | 2538.9–3708.7 | +60.5% | 4665.6 | 4054.7–4971.1 | +163.6% |
| std-control | 1744.5 | 1744.5–1744.5 | -8.6% | 2009.6 | 2007.6–2017.0 | -8.7% | 1279.0 | 1231.8–1884.6 | -27.7% |
| ordstack | 837.4 | 837.4–837.4 | -56.1% | 961.1 | 959.5–964.6 | -56.3% | 554.0 | 527.6–735.9 | -68.7% |
| ordstack-alloc | 1275.3 | 1275.3–1275.3 | -33.2% | 1682.5 | 1675.7–1691.1 | -23.6% | 1527.9 | 1373.0–1891.8 | -13.7% |
| bumpalo | 848.8 | 848.8–848.8 | -55.5% | 1068.7 | 1063.1–1072.9 | -51.4% | 999.0 | 802.8–1309.6 | -43.6% |
| bumpalo-frozen | 759.5 | 759.5–759.5 | -60.2% | 906.1 | 904.3–913.5 | -58.8% | 663.1 | 549.1–752.1 | -62.5% |

### xform-120k (`serde_json_xform big.json out_std.json`)

- attempt 1: CPU 3; worst wall IQR 107.7% of median; loadavg 19.86/17.19/13.15 → 18.39/17.49/13.61; chosen core busy before {'3': 0.0}; SMT sibling busy during {}; machine busy before 19.6% / during 16.8%
- attempt 2: CPU 5; worst wall IQR 162.6% of median; loadavg 17.64/17.35/13.59 → 14.60/16.38/13.56; chosen core busy before {'5': 0.0}; SMT sibling busy during {}; machine busy before 13.9% / during 12.8%
- attempt 3: CPU 6; worst wall IQR 39.5% of median; loadavg 14.00/16.23/13.52 → 13.04/15.49/13.48; chosen core busy before {'6': 0.0}; SMT sibling busy during {}; machine busy before 10.4% / during 13.1% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 307.7 | 300.1–317.4 | 292.2–352.6 | +0.0% | 306.8 | 57.8 | 57.8–57.8 | +0.0% | 14795 |
| std-mimalloc | 448.8 | 414.9–592.2 | 356.3–1732.9 | +45.9% | 447.8 | 55.7 | 55.7–55.7 | -3.6% | 343 |
| std-control | 306.9 | 299.4–322.6 | 294.4–378.0 | -0.3% | 305.6 | 57.8 | 57.8–57.8 | +0.0% | 14796 |
| ordstack | 209.6 | 208.3–212.8 | 205.5–236.6 | -31.9% | 208.6 | 33.8 | 33.8–33.8 | -41.6% | 8559 |
| ordstack-alloc | 270.5 | 267.9–277.6 | 265.1–301.9 | -12.1% | 269.5 | 81.8 | 81.8–81.8 | +41.6% | 20910 |
| bumpalo | 218.5 | 216.4–226.4 | 213.6–238.2 | -29.0% | 217.6 | 51.0 | 51.0–51.0 | -11.7% | 13014 |
| bumpalo-frozen | 218.5 | 217.0–222.3 | 213.7–246.0 | -29.0% | 217.6 | 51.0 | 51.0–51.0 | -11.7% | 13014 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 152.2 | 58.7 | 48.5 | 44.7 |
| std-mimalloc | 262.1 | 105.7 | 43.3 | 29.5 |
| std-control | 155.6 | 55.2 | 46.1 | 47.9 |
| ordstack | 120.9 | 39.3 | 38.6 | 10.9 |
| ordstack-alloc | 146.3 | 52.8 | 54.9 | 17.5 |
| bumpalo | 126.2 | 42.7 | 41.1 | 8.3 |
| bumpalo-frozen | 126.1 | 42.6 | 41.0 | 8.3 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 11; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1632.2 | 1632.2–1632.2 | +0.0% | 1749.0 | 1747.6–1781.0 | +0.0% | 1132.5 | 934.1–1503.0 | +0.0% |
| std-mimalloc | 1331.4 | 1331.4–1331.4 | -18.4% | 1799.1 | 1680.8–1932.8 | +2.9% | 1459.0 | 1349.7–1673.4 | +28.8% |
| std-control | 1600.0 | 1600.0–1600.0 | -2.0% | 1716.9 | 1715.7–1747.4 | -1.8% | 985.7 | 940.5–1409.2 | -13.0% |
| ordstack | 1313.0 | 1313.0–1313.0 | -19.6% | 1405.0 | 1402.2–1436.3 | -19.7% | 698.0 | 664.3–709.0 | -38.4% |
| ordstack-alloc | 1448.7 | 1448.7–1448.7 | -11.2% | 1618.5 | 1613.1–1647.2 | -7.5% | 894.8 | 863.7–913.0 | -21.0% |
| bumpalo | 1285.9 | 1285.9–1285.9 | -21.2% | 1405.6 | 1402.0–1435.8 | -19.6% | 716.4 | 696.9–737.2 | -36.7% |
| bumpalo-frozen | 1286.7 | 1286.7–1286.7 | -21.2% | 1406.2 | 1403.0–1437.1 | -19.6% | 700.7 | 684.4–738.3 | -38.1% |

### xform-480k (`serde_json_xform big4.json out_std.json`)

- attempt 1: CPU 6; worst wall IQR 84.1% of median; loadavg 13.04/15.49/13.48 → 28.04/21.03/16.08; chosen core busy before {'6': 0.0}; SMT sibling busy during {}; machine busy before 10.0% / during 15.7%
- attempt 2: CPU 7; worst wall IQR 66.6% of median; loadavg 26.76/20.88/16.06 → 16.38/18.14/16.45; chosen core busy before {'7': 0.0}; SMT sibling busy during {}; machine busy before 14.5% / during 17.4%
- attempt 3: CPU 2; worst wall IQR 42.1% of median; loadavg 16.38/18.14/16.45 → 16.14/17.45/16.86; chosen core busy before {'2': 0.0}; SMT sibling busy during {}; machine busy before 13.3% / during 17.1% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 1292.1 | 1193.8–1424.6 | 1129.3–9069.0 | +0.0% | 1290.8 | 226.5 | 226.5–226.5 | +0.0% | 58024 |
| std-mimalloc | 1783.6 | 1518.5–2269.2 | 829.0–20079.6 | +38.0% | 1781.2 | 193.7 | 185.4–193.7 | -14.5% | 963 |
| std-control | 1304.3 | 1210.7–1387.0 | 1128.3–1529.6 | +0.9% | 1300.8 | 226.5 | 226.5–226.5 | +0.0% | 58023 |
| ordstack | 839.8 | 823.7–847.2 | 807.1–11326.1 | -35.0% | 838.0 | 128.2 | 128.2–128.2 | -43.4% | 32998 |
| ordstack-alloc | 1082.3 | 1071.0–1117.0 | 1048.7–27517.0 | -16.2% | 1080.5 | 323.2 | 323.2–323.2 | +42.7% | 82691 |
| bumpalo | 882.1 | 855.0–905.0 | 837.5–22239.8 | -31.7% | 880.3 | 198.0 | 198.0–198.0 | -12.6% | 50822 |
| bumpalo-frozen | 870.7 | 857.1–882.8 | 833.8–21892.6 | -32.6% | 869.2 | 198.0 | 198.0–198.0 | -12.6% | 50823 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 621.7 | 265.3 | 278.4 | 110.5 |
| std-mimalloc | 1102.5 | 407.2 | 177.2 | 46.5 |
| std-control | 629.9 | 251.0 | 279.7 | 123.5 |
| ordstack | 488.8 | 159.1 | 158.5 | 29.3 |
| ordstack-alloc | 596.8 | 215.9 | 213.0 | 55.7 |
| bumpalo | 518.4 | 170.5 | 166.9 | 17.7 |
| bumpalo-frozen | 509.3 | 170.7 | 168.6 | 17.5 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 2; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 6322.4 | 6322.4–6322.4 | +0.0% | 6786.2 | 6763.6–6794.5 | +0.0% | 3845.3 | 3676.4–4535.8 | +0.0% |
| std-mimalloc | 5349.2 | 5349.2–5349.2 | -15.4% | 6999.4 | 6775.3–7496.6 | +3.1% | 5560.5 | 5181.3–6055.6 | +44.6% |
| std-control | 6193.1 | 6193.1–6193.1 | -2.0% | 6662.1 | 6627.1–6680.2 | -1.8% | 3977.0 | 3691.9–4809.1 | +3.4% |
| ordstack | 5276.0 | 5276.0–5276.0 | -16.6% | 5635.6 | 5600.3–5660.8 | -17.0% | 2709.3 | 2624.2–2772.9 | -29.5% |
| ordstack-alloc | 5822.0 | 5822.0–5822.0 | -7.9% | 6505.6 | 6464.3–6524.4 | -4.1% | 3568.3 | 3427.9–4080.1 | -7.2% |
| bumpalo | 5167.2 | 5167.2–5167.2 | -18.3% | 5637.6 | 5612.3–5655.1 | -16.9% | 2866.5 | 2742.5–3177.1 | -25.5% |
| bumpalo-frozen | 5167.2 | 5167.2–5167.2 | -18.3% | 5633.6 | 5608.4–5645.1 | -17.0% | 2822.5 | 2743.9–2968.8 | -26.6% |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | instructions:u vs std / bumpalo | cycles vs std / bumpalo | time vs bumpalo-frozen | RSS vs bumpalo-frozen |
|---|---|---|---|---|---|---|---|---|---|---|---|
| treebuild-d7 | -66.2% | yes | yes | -65.1% | yes | -38.2% | -40.5% | -56.0% / -1.3% | -63.9% / -38.1% | -6.8% | -12.0% |
| treebuild-d8 | -65.0% | yes | yes | -66.9% | yes | -36.0% | -42.9% | -56.1% / -1.3% | -68.7% / -44.5% | -2.7% | -16.0% |
| xform-120k | -31.9% | yes | yes | -41.6% | yes | -4.1% | -33.8% | -19.6% / +2.1% | -38.4% / -2.6% | -4.1% | -33.8% |
| xform-480k | -35.0% | yes | yes | -43.4% | yes | -4.8% | -35.2% | -16.6% / +2.1% | -29.5% / -5.5% | -3.5% | -35.2% |

- treebuild: time criterion at both sizes: met; RSS criterion at both sizes: met
- serde_json_xform: time criterion at both sizes: met; RSS criterion at both sizes: met

### Arena statistics (ORDSTACK_STATS=1, one run)

- treebuild-d7/ordstack: stack tree high_water=15592336 extent=15592336; stack scratch high_water=256 extent=64
- treebuild-d7/ordstack-alloc: stack tree high_water=52121928 extent=52121800
- treebuild-d7/bumpalo: bump allocated_bytes=33552896
- treebuild-d7/bumpalo-frozen: bump allocated_bytes=33552896
- treebuild-d8/ordstack: stack tree high_water=81217336 extent=81217336; stack scratch high_water=288 extent=64
- treebuild-d8/ordstack-alloc: stack tree high_water=267121928 extent=267121800
- treebuild-d8/bumpalo: bump allocated_bytes=268433728
- treebuild-d8/bumpalo-frozen: bump allocated_bytes=134216064
- xform-120k/ordstack: stack recs high_water=9437312 extent=8640000; stack data high_water=8997696 extent=8997684; recs_vec_capacity_bytes=9437184; stack out high_water=14757682 extent=14757679
- xform-120k/ordstack-alloc: stack data high_water=86588666 extent=86588657
- xform-120k/bumpalo: bump recs allocated_bytes=33552896; bump data allocated_bytes=33552896; bump out allocated_bytes=23052160
- xform-120k/bumpalo-frozen: bump recs allocated_bytes=33552896; bump data allocated_bytes=33552896; bump out allocated_bytes=23052160
- xform-480k/ordstack: stack recs high_water=37748864 extent=34560000; stack data high_water=35942616 extent=35942604; recs_vec_capacity_bytes=37748736; stack out high_water=58982603 extent=58982600
- xform-480k/ordstack-alloc: stack data high_water=346257107 extent=346257097
- xform-480k/bumpalo: bump recs allocated_bytes=134216064; bump data allocated_bytes=134216064; bump out allocated_bytes=92172160
- xform-480k/bumpalo-frozen: bump recs allocated_bytes=134216064; bump data allocated_bytes=134216064; bump out allocated_bytes=92172160

