Runs per (variant, input): 31 timed after 3 warm-up rounds, every run of a case pinned to one CPU. Re-run rule: worst variant's wall IQR > 4% of its median → the case is re-run on a fresh core.

### treebuild-d7 (`treebuild 7`)

- attempt 1: CPU 71; worst wall IQR 9.2% of median; loadavg 19.00/24.45/36.18 → 21.32/24.77/36.15; chosen core busy before {'71': 0.0}; SMT sibling busy during {}; machine busy before 32.9% / during 33.1%
- attempt 2: CPU 38; worst wall IQR 3.3% of median; loadavg 21.78/24.81/36.11 → 23.13/24.99/36.04; chosen core busy before {'38': 0.3}; SMT sibling busy during {}; machine busy before 32.7% / during 32.9% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 93.0 | 92.2–94.1 | 91.2–97.2 | +0.0% | 91.5 | 47.2 | 47.2–47.2 | +0.0% | 12090 |
| std-mimalloc | 55.5 | 55.0–56.1 | 53.5–62.1 | -40.3% | 53.9 | 51.7 | 51.7–51.7 | +9.4% | 326 |
| std-control | 81.7 | 81.1–82.9 | 79.5–85.9 | -12.1% | 80.1 | 42.8 | 42.8–42.8 | -9.5% | 10945 |
| ordstack | 35.3 | 35.0–36.1 | 34.1–38.6 | -62.0% | 33.7 | 15.8 | 15.8–15.8 | -66.7% | 4087 |
| bumpalo | 51.0 | 50.6–51.5 | 49.7–56.1 | -45.1% | 49.4 | 27.8 | 27.8–27.8 | -41.3% | 7100 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 18; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 364.2 | 364.2–364.2 | +0.0% | 421.6 | 421.3–422.4 | +0.0% | 267.0 | 266.3–269.6 | +0.0% |
| std-mimalloc | 276.7 | 276.7–276.7 | -24.0% | 281.2 | 281.1–281.4 | -33.3% | 165.9 | 164.3–171.2 | -37.8% |
| std-control | 332.3 | 332.3–332.3 | -8.8% | 384.1 | 384.0–385.9 | -8.9% | 233.8 | 232.1–237.0 | -12.4% |
| ordstack | 164.0 | 164.0–164.0 | -55.0% | 188.4 | 188.4–188.9 | -55.3% | 97.5 | 95.7–101.9 | -63.5% |
| bumpalo | 162.7 | 162.7–162.7 | -55.3% | 205.4 | 205.2–206.0 | -51.3% | 149.8 | 146.2–151.4 | -43.9% |

### treebuild-d8 (`treebuild 8`)

- attempt 1: CPU 3; worst wall IQR 4.8% of median; loadavg 23.52/25.04/36.00 → 26.04/25.46/35.51; chosen core busy before {'3': 0.0}; SMT sibling busy during {}; machine busy before 30.4% / during 28.6%
- attempt 2: CPU 4; worst wall IQR 3.2% of median; loadavg 26.04/25.46/35.51 → 21.49/24.37/34.56; chosen core busy before {'4': 0.0}; SMT sibling busy during {}; machine busy before 30.3% / during 19.4% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 468.1 | 462.8–471.9 | 458.6–492.8 | +0.0% | 466.2 | 237.8 | 237.8–237.8 | +0.0% | 60920 |
| std-mimalloc | 268.5 | 264.0–272.6 | 261.7–299.8 | -42.6% | 266.7 | 235.7 | 235.7–235.7 | -0.9% | 426 |
| std-control | 404.3 | 402.2–407.4 | 397.4–437.7 | -13.6% | 402.5 | 216.0 | 216.0–216.0 | -9.1% | 55198 |
| ordstack | 171.0 | 169.8–171.9 | 166.8–199.5 | -63.5% | 169.1 | 78.8 | 78.8–78.8 | -66.9% | 20112 |
| bumpalo | 249.9 | 248.6–252.2 | 242.4–256.5 | -46.6% | 248.0 | 138.0 | 138.0–138.0 | -42.0% | 35332 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 7; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1825.0 | 1825.0–1825.0 | +0.0% | 2115.4 | 2114.9–2120.3 | +0.0% | 1328.8 | 1317.7–1355.0 | +0.0% |
| std-mimalloc | 1382.1 | 1382.1–1382.1 | -24.3% | 1397.8 | 1397.2–1398.7 | -33.9% | 838.2 | 818.7–849.6 | -36.9% |
| std-control | 1665.6 | 1665.6–1665.6 | -8.7% | 1927.6 | 1927.4–1929.0 | -8.9% | 1149.1 | 1135.0–1159.8 | -13.5% |
| ordstack | 819.5 | 819.5–819.5 | -55.1% | 941.7 | 941.3–944.1 | -55.5% | 519.1 | 511.6–520.4 | -60.9% |
| bumpalo | 813.2 | 813.2–813.2 | -55.4% | 1027.3 | 1026.9–1029.0 | -51.4% | 734.7 | 731.8–743.2 | -44.7% |

### xform-120k (`serde_json_xform big.json out_std.json`)

- attempt 1: CPU 4; worst wall IQR 0.9% of median; loadavg 20.73/24.17/34.44 → 17.36/22.85/33.56; chosen core busy before {'4': 0.0}; SMT sibling busy during {}; machine busy before 15.4% / during 16.2% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 295.0 | 293.9–296.7 | 291.7–324.1 | +0.0% | 293.2 | 57.8 | 57.8–57.8 | +0.0% | 14787 |
| std-mimalloc | 189.6 | 188.8–190.2 | 187.6–192.1 | -35.7% | 187.7 | 55.7 | 55.7–55.7 | -3.6% | 337 |
| std-control | 296.3 | 295.4–297.5 | 293.4–300.2 | +0.4% | 294.5 | 57.8 | 57.8–57.8 | +0.0% | 14789 |
| ordstack | 184.4 | 183.9–184.9 | 183.0–234.3 | -37.5% | 182.6 | 33.8 | 33.8–33.8 | -41.6% | 8551 |
| bumpalo | 196.4 | 195.8–196.9 | 194.8–246.2 | -33.4% | 194.5 | 51.0 | 51.0–51.0 | -11.7% | 13006 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 147.2 | 57.2 | 43.3 | 47.3 |
| std-mimalloc | 109.1 | 29.7 | 39.6 | 11.0 |
| std-control | 147.6 | 54.1 | 45.7 | 48.4 |
| ordstack | 105.3 | 32.1 | 35.4 | 11.3 |
| bumpalo | 114.8 | 35.0 | 37.7 | 8.9 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 10; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1594.3 | 1594.3–1594.3 | +0.0% | 1711.2 | 1709.3–1741.6 | +0.0% | 1005.7 | 940.3–1070.0 | +0.0% |
| std-mimalloc | 1312.4 | 1312.4–1312.4 | -17.7% | 1355.1 | 1354.8–1389.0 | -20.8% | 614.5 | 609.8–669.0 | -38.9% |
| std-control | 1567.5 | 1567.5–1567.5 | -1.7% | 1683.5 | 1682.8–1714.7 | -1.6% | 949.5 | 936.1–1193.1 | -5.6% |
| ordstack | 1216.1 | 1216.1–1216.1 | -23.7% | 1306.6 | 1304.9–1338.1 | -23.6% | 606.7 | 586.9–632.9 | -39.7% |
| bumpalo | 1218.4 | 1218.4–1218.4 | -23.6% | 1336.8 | 1334.3–1367.8 | -21.9% | 639.2 | 623.4–688.8 | -36.4% |

### xform-480k (`serde_json_xform big4.json out_std.json`)

- attempt 1: CPU 17; worst wall IQR 1.6% of median; loadavg 17.36/22.85/33.56 → 9.46/17.88/30.08; chosen core busy before {'17': 0.0}; SMT sibling busy during {}; machine busy before 15.0% / during 11.6% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB (`/usr/bin/time %M`, 5 runs) | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 1140.0 | 1131.3–1144.7 | 1119.5–1180.8 | +0.0% | 1138.4 | 226.5 | 226.5–226.5 | +0.0% | 58014 |
| std-mimalloc | 740.4 | 736.3–745.0 | 731.2–761.5 | -35.0% | 738.9 | 193.7 | 193.7–193.7 | -14.5% | 410 |
| std-control | 1138.7 | 1127.2–1145.3 | 1115.1–1186.2 | -0.1% | 1137.1 | 226.5 | 226.5–226.5 | +0.0% | 58015 |
| ordstack | 719.0 | 716.3–724.0 | 711.5–749.8 | -36.9% | 717.1 | 128.2 | 128.2–128.2 | -43.4% | 32990 |
| bumpalo | 768.0 | 764.8–773.0 | 756.3–804.7 | -32.6% | 766.3 | 198.0 | 198.0–198.0 | -12.6% | 50814 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 587.3 | 238.7 | 224.1 | 84.6 |
| std-mimalloc | 439.9 | 117.7 | 159.0 | 23.3 |
| std-control | 592.4 | 223.6 | 233.3 | 86.8 |
| ordstack | 422.1 | 126.1 | 141.4 | 29.2 |
| bumpalo | 462.7 | 137.6 | 150.0 | 18.5 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 11; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 6169.8 | 6169.8–6169.8 | +0.0% | 6630.3 | 6602.9–6632.6 | +0.0% | 3724.5 | 3637.8–3886.0 | +0.0% |
| std-mimalloc | 5272.3 | 5272.3–5272.3 | -14.5% | 5438.5 | 5407.7–5441.5 | -18.0% | 2439.6 | 2418.2–2461.5 | -34.5% |
| std-control | 6062.2 | 6062.2–6062.2 | -1.7% | 6523.1 | 6492.3–6526.8 | -1.6% | 3737.3 | 3657.7–3804.7 | +0.3% |
| ordstack | 4884.4 | 4884.4–4884.4 | -20.8% | 5238.3 | 5206.1–5241.4 | -21.0% | 2333.3 | 2284.2–2355.1 | -37.4% |
| bumpalo | 4892.8 | 4892.8–4892.8 | -20.7% | 5356.0 | 5323.7–5357.5 | -19.2% | 2496.2 | 2433.7–2512.5 | -33.0% |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | instructions:u vs std / bumpalo | cycles vs std / bumpalo |
|---|---|---|---|---|---|---|---|---|---|
| treebuild-d7 | -62.0% | yes | yes | -66.7% | yes | -30.8% | -43.2% | -55.0% / +0.8% | -63.5% / -34.9% |
| treebuild-d8 | -63.5% | yes | yes | -66.9% | yes | -31.6% | -42.9% | -55.1% / +0.8% | -60.9% / -29.3% |
| xform-120k | -37.5% | yes | yes | -41.6% | yes | -6.1% | -33.8% | -23.7% / -0.2% | -39.7% / -5.1% |
| xform-480k | -36.9% | yes | yes | -43.4% | yes | -6.4% | -35.2% | -20.8% / -0.2% | -37.4% / -6.5% |

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

