Runs per (variant, input): 31 timed after 3 warm-up rounds, every run of a case pinned to one CPU. Re-run rule: worst variant's wall IQR > 4% of its median → the case is re-run on a fresh core.

### treebuild-d7 (`treebuild 7`)

- attempt 1: CPU 7; worst wall IQR 175.9% of median; loadavg 24.31/28.57/27.63 → 56.00/37.13/30.66; chosen core busy before {'7': 19.2}; SMT sibling busy during {}; machine busy before 57.5% / during 41.2%
- attempt 2: CPU 57; worst wall IQR 5.2% of median; loadavg 53.44/36.91/30.63 → 46.45/36.43/30.61; chosen core busy before {'57': 0.3}; SMT sibling busy during {}; machine busy before 28.9% / during 25.9% **(reported)**
- attempt 3: CPU 42; worst wall IQR 28.5% of median; loadavg 46.45/36.43/30.61 → 35.44/34.67/30.20; chosen core busy before {'42': 0.0}; SMT sibling busy during {}; machine busy before 19.7% / during 16.2%

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MiB (`/usr/bin/time %M`, 5 runs) | RSS min–max MiB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 93.8 | 93.2–94.4 | 91.5–109.1 | +0.0% | 92.3 | 47.2 | 47.2–47.2 | +0.0% | 12096 |
| std-mimalloc | 56.8 | 56.4–57.4 | 54.7–60.3 | -39.4% | 55.3 | 47.2 | 47.2–48.9 | +0.0% | 335 |
| std-control | 83.5 | 82.3–84.3 | 81.6–85.6 | -11.0% | 82.0 | 42.8 | 42.8–42.8 | -9.5% | 10952 |
| ordstack | 35.2 | 34.3–36.1 | 33.6–37.0 | -62.5% | 33.8 | 15.8 | 15.8–16.5 | -66.7% | 4096 |
| ordstack-alloc | 86.0 | 85.1–86.6 | 82.6–89.1 | -8.4% | 84.5 | 52.5 | 52.5–52.5 | +11.1% | 13330 |
| alloc-control | 105.5 | 104.6–106.1 | 103.4–109.0 | +12.4% | 104.1 | 56.2 | 56.2–56.2 | +19.0% | 14309 |
| bumpalo | 53.3 | 53.0–53.6 | 51.9–58.4 | -43.2% | 51.9 | 27.8 | 27.8–27.8 | -41.3% | 7107 |
| bumpalo-frozen | 36.9 | 36.2–37.2 | 35.4–37.9 | -60.7% | 35.3 | 18.8 | 18.8–18.8 | -60.3% | 4821 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 7; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 380.8 | 380.8–380.8 | +0.0% | 439.8 | 438.0–447.4 | +0.0% | 394.4 | 287.4–727.1 | +0.0% |
| std-mimalloc | 283.3 | 283.3–283.3 | -25.6% | 985.2 | 288.2–2096.4 | +124.0% | 2389.2 | 217.8–8188.9 | +505.8% |
| std-control | 348.1 | 348.1–348.1 | -8.6% | 400.7 | 399.7–412.5 | -8.9% | 297.6 | 241.8–695.8 | -24.5% |
| ordstack | 167.5 | 167.5–167.5 | -56.0% | 192.3 | 192.0–196.3 | -56.3% | 113.2 | 99.5–271.2 | -71.3% |
| ordstack-alloc | 242.0 | 242.0–242.0 | -36.5% | 324.7 | 322.5–334.3 | -26.2% | 393.6 | 283.6–560.0 | -0.2% |
| alloc-control | 403.9 | 403.9–403.9 | +6.1% | 472.8 | 471.6–493.0 | +7.5% | 484.5 | 312.7–834.9 | +22.9% |
| bumpalo | 169.8 | 169.8–169.8 | -55.4% | 213.4 | 212.5–219.7 | -51.5% | 225.9 | 164.9–415.6 | -42.7% |
| bumpalo-frozen | 152.0 | 152.0–152.0 | -60.1% | 181.9 | 180.8–188.3 | -58.6% | 132.7 | 106.5–326.3 | -66.4% |

### treebuild-d8 (`treebuild 8`)

- attempt 1: CPU 35; worst wall IQR 44.3% of median; loadavg 35.44/34.67/30.20 → 17.51/26.68/27.89; chosen core busy before {'35': 0.0}; SMT sibling busy during {}; machine busy before 21.3% / during 16.8% **(reported)**
- attempt 2: CPU 35; worst wall IQR 64.0% of median; loadavg 18.51/26.74/27.90 → 15.75/22.59/26.18; chosen core busy before {'35': 0.0}; SMT sibling busy during {}; machine busy before 17.5% / during 15.3%
- attempt 3: CPU 1; worst wall IQR 95.4% of median; loadavg 15.21/22.36/26.09 → 18.38/21.24/24.93; chosen core busy before {'1': 0.0}; SMT sibling busy during {}; machine busy before 13.4% / during 20.5%

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MiB (`/usr/bin/time %M`, 5 runs) | RSS min–max MiB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 500.0 | 475.7–583.0 | 460.6–6420.6 | +0.0% | 498.6 | 237.8 | 237.8–237.8 | +0.0% | 60928 |
| std-mimalloc | 1143.9 | 1034.2–1235.0 | 843.2–1788.5 | +128.8% | 1142.1 | 231.8 | 231.8–233.2 | -2.5% | 436 |
| std-control | 455.8 | 418.5–542.4 | 403.0–7564.1 | -8.8% | 454.5 | 216.0 | 216.0–216.0 | -9.1% | 55205 |
| ordstack | 181.3 | 168.8–199.9 | 165.0–387.0 | -63.7% | 179.7 | 78.8 | 78.8–78.8 | -66.9% | 20121 |
| ordstack-alloc | 551.0 | 434.7–678.9 | 416.2–8873.2 | +10.2% | 549.2 | 257.2 | 257.2–257.2 | +8.2% | 65825 |
| alloc-control | 588.1 | 548.7–788.1 | 521.5–10328.1 | +17.6% | 586.7 | 281.2 | 281.2–281.2 | +18.3% | 71991 |
| bumpalo | 287.7 | 261.2–321.0 | 249.6–4721.1 | -42.5% | 286.3 | 138.0 | 138.0–138.0 | -42.0% | 35340 |
| bumpalo-frozen | 180.8 | 171.6–197.2 | 166.1–3673.9 | -63.8% | 179.5 | 93.8 | 93.8–93.8 | -60.6% | 23896 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 2; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1908.2 | 1908.2–1908.2 | +0.0% | 2206.6 | 2198.9–2215.4 | +0.0% | 1640.0 | 1437.7–2347.1 | +0.0% |
| std-mimalloc | 1415.1 | 1415.0–1415.3 | -25.8% | 6236.0 | 4503.8–7164.8 | +182.6% | 21312.4 | 6496.4–28267.2 | +1199.6% |
| std-control | 1744.5 | 1744.5–1744.5 | -8.6% | 2014.5 | 2010.4–2026.4 | -8.7% | 1580.9 | 1333.2–1917.1 | -3.6% |
| ordstack | 837.4 | 837.4–837.4 | -56.1% | 964.5 | 960.1–968.2 | -56.3% | 577.5 | 545.5–817.5 | -64.8% |
| ordstack-alloc | 1242.1 | 1242.1–1242.1 | -34.9% | 1658.9 | 1645.0–1669.8 | -24.8% | 1687.1 | 1530.6–2046.7 | +2.9% |
| alloc-control | 2051.7 | 2051.7–2051.7 | +7.5% | 2408.0 | 2398.9–2413.9 | +9.1% | 2166.5 | 1683.3–2803.3 | +32.1% |
| bumpalo | 848.8 | 848.8–848.8 | -55.5% | 1075.9 | 1063.1–1079.8 | -51.2% | 1001.6 | 825.7–1259.9 | -38.9% |
| bumpalo-frozen | 759.5 | 759.5–759.5 | -60.2% | 909.1 | 904.2–917.2 | -58.8% | 634.8 | 527.0–911.0 | -61.3% |

### xform-120k (`serde_json_xform big.json out_std.json`)

- attempt 1: CPU 10; worst wall IQR 31.2% of median; loadavg 17.95/21.10/24.86 → 36.34/27.74/26.96; chosen core busy before {'10': 0.0}; SMT sibling busy during {}; machine busy before 11.0% / during 36.5%
- attempt 2: CPU 1; worst wall IQR 28.9% of median; loadavg 35.43/27.69/26.94 → 19.02/24.19/25.77; chosen core busy before {'1': 0.0}; SMT sibling busy during {}; machine busy before 7.4% / during 14.7%
- attempt 3: CPU 1; worst wall IQR 22.5% of median; loadavg 18.14/23.92/25.67 → 18.76/22.68/25.08; chosen core busy before {'1': 0.0}; SMT sibling busy during {}; machine busy before 8.0% / during 17.9% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MiB (`/usr/bin/time %M`, 5 runs) | RSS min–max MiB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 308.9 | 299.0–359.8 | 292.1–492.4 | +0.0% | 307.1 | 57.8 | 57.8–57.8 | +0.0% | 14799 |
| std-mimalloc | 387.6 | 381.6–408.3 | 258.3–464.5 | +25.5% | 385.9 | 46.5 | 44.0–48.4 | -19.5% | 346 |
| std-control | 351.2 | 310.2–389.3 | 294.2–484.3 | +13.7% | 349.8 | 57.8 | 57.8–57.8 | +0.0% | 14799 |
| ordstack | 213.1 | 209.8–214.9 | 206.6–228.0 | -31.0% | 211.7 | 33.8 | 33.8–33.8 | -41.6% | 8562 |
| ordstack-alloc | 263.3 | 256.1–266.2 | 247.4–273.5 | -14.8% | 261.4 | 81.8 | 81.8–81.8 | +41.6% | 20915 |
| alloc-control | 335.1 | 304.6–365.9 | 297.9–458.8 | +8.5% | 333.4 | 66.8 | 66.8–66.8 | +15.6% | 17096 |
| bumpalo | 220.7 | 219.4–227.3 | 214.6–242.4 | -28.5% | 219.4 | 51.0 | 51.0–51.0 | -11.7% | 13017 |
| bumpalo-frozen | 220.5 | 218.1–227.4 | 215.3–239.8 | -28.6% | 218.8 | 51.0 | 51.0–51.0 | -11.7% | 13017 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 153.8 | 60.3 | 47.2 | 49.8 |
| std-mimalloc | 230.7 | 86.6 | 44.8 | 25.5 |
| std-control | 157.7 | 59.2 | 56.6 | 69.4 |
| ordstack | 122.0 | 40.8 | 38.4 | 11.9 |
| ordstack-alloc | 149.1 | 46.6 | 44.6 | 19.6 |
| alloc-control | 161.5 | 58.6 | 60.6 | 55.5 |
| bumpalo | 127.3 | 43.5 | 41.8 | 9.1 |
| bumpalo-frozen | 127.3 | 43.3 | 41.6 | 8.9 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 7; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1632.2 | 1632.2–1632.2 | +0.0% | 1754.3 | 1750.2–1783.4 | +0.0% | 1093.4 | 1026.3–1249.6 | +0.0% |
| std-mimalloc | 1331.4 | 1331.4–1331.5 | -18.4% | 2234.4 | 1944.9–2616.5 | +27.4% | 4141.2 | 2594.4–8494.1 | +278.7% |
| std-control | 1600.0 | 1600.0–1600.0 | -2.0% | 1722.4 | 1719.0–1764.1 | -1.8% | 1118.8 | 1030.7–1519.0 | +2.3% |
| ordstack | 1313.0 | 1313.0–1313.0 | -19.6% | 1406.9 | 1403.9–1438.9 | -19.8% | 726.4 | 674.7–802.1 | -33.6% |
| ordstack-alloc | 1352.5 | 1352.5–1352.5 | -17.1% | 1526.1 | 1520.2–1562.1 | -13.0% | 886.2 | 829.4–1156.4 | -18.9% |
| alloc-control | 1692.0 | 1692.0–1692.0 | +3.7% | 1827.6 | 1824.5–1858.0 | +4.2% | 1024.1 | 991.8–1249.0 | -6.3% |
| bumpalo | 1285.9 | 1285.9–1285.9 | -21.2% | 1408.5 | 1407.6–1443.6 | -19.7% | 762.6 | 711.4–810.0 | -30.3% |
| bumpalo-frozen | 1286.7 | 1286.7–1286.7 | -21.2% | 1409.3 | 1406.6–1445.2 | -19.7% | 757.6 | 724.8–812.5 | -30.7% |

### xform-480k (`serde_json_xform big4.json out_std.json`)

- attempt 1: CPU 1; worst wall IQR 66.0% of median; loadavg 18.76/22.68/25.08 → 30.01/21.70/23.06; chosen core busy before {'1': 0.0}; SMT sibling busy during {}; machine busy before 11.6% / during 17.3%
- attempt 2: CPU 22; worst wall IQR 47.8% of median; loadavg 30.01/21.70/23.06 → 29.73/24.54/23.65; chosen core busy before {'22': 0.3}; SMT sibling busy during {}; machine busy before 19.7% / during 23.8% **(reported)**
- attempt 3: CPU 61; worst wall IQR 122.1% of median; loadavg 28.95/24.47/23.63 → 22.38/23.34/23.42; chosen core busy before {'61': 0.3}; SMT sibling busy during {}; machine busy before 22.3% / during 22.0%

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MiB (`/usr/bin/time %M`, 5 runs) | RSS min–max MiB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 1338.9 | 1173.3–1497.9 | 1150.4–27823.7 | +0.0% | 1337.3 | 226.5 | 226.5–226.5 | +0.0% | 58026 |
| std-mimalloc | 3260.9 | 2377.1–3936.8 | 1132.9–29529.6 | +143.5% | 3258.9 | 169.5 | 167.4–185.2 | -25.2% | 1833 |
| std-control | 1338.4 | 1263.0–1429.6 | 1176.1–27294.4 | -0.0% | 1331.1 | 226.5 | 226.5–226.5 | +0.0% | 58027 |
| ordstack | 836.9 | 822.0–858.6 | 805.4–21137.7 | -37.5% | 834.8 | 128.2 | 128.2–128.2 | -43.4% | 33001 |
| ordstack-alloc | 1029.7 | 999.5–1045.7 | 980.5–19113.0 | -23.1% | 1028.1 | 323.2 | 323.2–323.2 | +42.7% | 82693 |
| alloc-control | 1261.6 | 1160.0–1359.2 | 1116.3–1847.9 | -5.8% | 1259.8 | 262.5 | 262.5–262.5 | +15.9% | 67227 |
| bumpalo | 866.2 | 852.5–898.8 | 832.1–4289.8 | -35.3% | 864.4 | 198.0 | 198.0–198.0 | -12.6% | 50825 |
| bumpalo-frozen | 861.5 | 852.5–890.7 | 841.2–23483.5 | -35.7% | 859.8 | 198.0 | 198.0–198.0 | -12.6% | 50826 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 618.2 | 273.7 | 328.7 | 123.6 |
| std-mimalloc | 2307.8 | 541.6 | 182.5 | 86.6 |
| std-control | 636.0 | 265.0 | 290.5 | 143.3 |
| ordstack | 487.4 | 158.7 | 160.4 | 29.4 |
| ordstack-alloc | 601.1 | 181.4 | 179.4 | 57.7 |
| alloc-control | 649.5 | 229.3 | 258.6 | 94.0 |
| bumpalo | 505.3 | 170.9 | 168.6 | 18.5 |
| bumpalo-frozen | 507.7 | 169.1 | 168.4 | 18.8 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 11; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 6322.4 | 6322.4–6322.4 | +0.0% | 6817.0 | 6755.4–6849.2 | +0.0% | 7254.9 | 3777.0–12300.4 | +0.0% |
| std-mimalloc | 5349.5 | 5349.2–5349.8 | -15.4% | 9853.7 | 7047.3–14737.7 | +44.5% | 28856.1 | 6493.3–323570.9 | +297.7% |
| std-control | 6193.1 | 6193.1–6193.1 | -2.0% | 6685.1 | 6657.4–6714.5 | -1.9% | 6663.7 | 3787.0–10183.2 | -8.1% |
| ordstack | 5276.0 | 5276.0–5276.0 | -16.6% | 5645.5 | 5628.4–5669.5 | -17.2% | 4157.7 | 2726.9–5964.1 | -42.7% |
| ordstack-alloc | 5436.0 | 5436.0–5436.0 | -14.0% | 6123.5 | 6105.4–6150.4 | -10.2% | 4974.7 | 3356.6–7057.3 | -31.4% |
| alloc-control | 6576.9 | 6576.9–6576.9 | +4.0% | 7127.8 | 7090.7–7137.9 | +4.6% | 5015.2 | 3944.8–8054.9 | -30.9% |
| bumpalo | 5167.2 | 5167.2–5167.2 | -18.3% | 5652.6 | 5603.3–5663.8 | -17.1% | 4702.3 | 2937.1–5927.7 | -35.2% |
| bumpalo-frozen | 5167.2 | 5167.2–5167.2 | -18.3% | 5653.5 | 5600.8–5680.4 | -17.1% | 4825.4 | 2979.8–8544.4 | -33.5% |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | instructions:u vs std / bumpalo | cycles vs std / bumpalo | time vs bumpalo-frozen | RSS vs bumpalo-frozen |
|---|---|---|---|---|---|---|---|---|---|---|---|
| treebuild-d7 | -62.5% | yes | yes | -66.7% | yes | -33.9% | -43.2% | -56.0% / -1.3% | -71.3% / -49.9% | -4.5% | -16.0% |
| treebuild-d8 | -63.7% | yes | yes | -66.9% | yes | -37.0% | -42.9% | -56.1% / -1.3% | -64.8% / -42.3% | +0.3% | -16.0% |
| xform-120k | -31.0% | yes | yes | -41.6% | yes | -3.5% | -33.8% | -19.6% / +2.1% | -33.6% / -4.8% | -3.4% | -33.8% |
| xform-480k | -37.5% | yes | yes | -43.4% | yes | -3.4% | -35.2% | -16.6% / +2.1% | -42.7% / -11.6% | -2.9% | -35.2% |

- treebuild: time criterion at both sizes: met; RSS criterion at both sizes: met
- serde_json_xform: time criterion at both sizes: met; RSS criterion at both sizes: met

### Arena statistics (ORDSTACK_STATS=1, one run)

- treebuild-d7/ordstack: stack tree high_water=15592336 extent=15592336; stack scratch high_water=256 extent=64
- treebuild-d7/ordstack-alloc: stack tree high_water=53423992 extent=53423864
- treebuild-d7/bumpalo: bump allocated_bytes=33552896
- treebuild-d7/bumpalo-frozen: bump allocated_bytes=33552896
- treebuild-d8/ordstack: stack tree high_water=81217336 extent=81217336; stack scratch high_water=288 extent=64
- treebuild-d8/ordstack-alloc: stack tree high_water=268423992 extent=268423864
- treebuild-d8/bumpalo: bump allocated_bytes=268433728
- treebuild-d8/bumpalo-frozen: bump allocated_bytes=134216064
- xform-120k/ordstack: stack recs high_water=9437312 extent=8640000; stack data high_water=8997696 extent=8997684; recs_vec_capacity_bytes=9437184; stack out high_water=14757682 extent=14757679
- xform-120k/ordstack-alloc: stack data high_water=86588607 extent=86588601
- xform-120k/bumpalo: bump recs allocated_bytes=33552896; bump data allocated_bytes=33552896; bump out allocated_bytes=23052160
- xform-120k/bumpalo-frozen: bump recs allocated_bytes=33552896; bump data allocated_bytes=33552896; bump out allocated_bytes=23052160
- xform-480k/ordstack: stack recs high_water=37748864 extent=34560000; stack data high_water=35942616 extent=35942604; recs_vec_capacity_bytes=37748736; stack out high_water=58982603 extent=58982600
- xform-480k/ordstack-alloc: stack data high_water=346257048 extent=346257041
- xform-480k/bumpalo: bump recs allocated_bytes=134216064; bump data allocated_bytes=134216064; bump out allocated_bytes=92172160
- xform-480k/bumpalo-frozen: bump recs allocated_bytes=134216064; bump data allocated_bytes=134216064; bump out allocated_bytes=92172160

