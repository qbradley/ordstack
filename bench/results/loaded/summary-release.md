Runs per (variant, input): 31 timed after 3 warm-up rounds, every run of a case pinned to one CPU. Re-run rule: worst variant's wall IQR > 4% of its median → the case is re-run on a fresh core.

### treebuild-d7 (`treebuild 7`)

- attempt 1: CPU 92; worst wall IQR 69.3% of median; loadavg 68.55/59.95/43.06 → 67.36/60.24/43.51; chosen core busy before {'92': 0.3}; SMT sibling busy during {}; machine busy before 29.4% / during 30.5%
- attempt 2: CPU 64; worst wall IQR 60.9% of median; loadavg 67.25/60.34/43.63 → 67.67/60.98/44.29; chosen core busy before {'64': 0.7}; SMT sibling busy during {}; machine busy before 29.1% / during 29.8%
- attempt 3: CPU 61; worst wall IQR 42.2% of median; loadavg 67.45/61.05/44.41 → 67.42/61.60/45.03; chosen core busy before {'61': 0.7}; SMT sibling busy during {}; machine busy before 28.3% / during 30.8% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 93.0 | 91.8–94.5 | 90.5–96.1 | +0.0% | 91.8 | 47.2 | 47.2–47.2 | +0.0% | 12085 |
| std-mimalloc | 351.1 | 282.8–431.1 | 152.5–2140.7 | +277.7% | 349.9 | 49.0 | 47.1–51.7 | +3.7% | 10518 |
| std-control | 82.2 | 81.6–83.9 | 80.2–86.3 | -11.6% | 81.1 | 42.8 | 42.8–42.8 | -9.5% | 10941 |
| ordstack | 33.7 | 33.0–34.3 | 31.6–45.4 | -63.7% | 32.5 | 22.4 | 22.4–22.5 | -52.5% | 4085 |
| bumpalo | 51.5 | 51.2–52.6 | 50.1–57.8 | -44.6% | 50.4 | 27.8 | 27.8–27.8 | -41.3% | 7096 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 2; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 380.8 | 380.8–380.8 | +0.0% | 438.0 | 437.9–438.6 | +0.0% | 287.0 | 285.7–293.8 | +0.0% |
| std-mimalloc | 283.3 | 283.3–283.3 | -25.6% | 287.8 | 287.8–287.9 | -34.3% | 173.2 | 171.6–174.5 | -39.7% |
| std-control | 348.1 | 348.1–348.1 | -8.6% | 399.8 | 399.7–399.9 | -8.7% | 249.6 | 246.3–256.1 | -13.0% |
| ordstack | 167.5 | 167.5–167.5 | -56.0% | 192.0 | 192.0–192.0 | -56.2% | 99.2 | 98.7–101.9 | -65.4% |
| bumpalo | 169.8 | 169.8–169.8 | -55.4% | 212.5 | 212.5–212.7 | -51.5% | 166.4 | 165.1–166.6 | -42.0% |

### treebuild-d8 (`treebuild 8`)

- attempt 1: CPU 67; worst wall IQR 53.6% of median; loadavg 67.42/61.60/45.03 → 63.77/62.60/47.51; chosen core busy before {'67': 1.3}; SMT sibling busy during {}; machine busy before 32.0% / during 29.6%
- attempt 2: CPU 56; worst wall IQR 30.9% of median; loadavg 63.39/62.54/47.58 → 60.08/61.47/48.79; chosen core busy before {'56': 0.3}; SMT sibling busy during {}; machine busy before 29.3% / during 25.4% **(reported)**
- attempt 3: CPU 56; worst wall IQR 31.5% of median; loadavg 59.35/61.30/48.80 → 57.95/60.89/50.03; chosen core busy before {'56': 0.7}; SMT sibling busy during {}; machine busy before 26.6% / during 24.0%

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 459.1 | 450.7–466.2 | 436.8–535.3 | +0.0% | 457.8 | 237.8 | 237.8–237.8 | +0.0% | 60915 |
| std-mimalloc | 1709.4 | 1522.5–2051.1 | 1272.1–3587.5 | +272.4% | 1706.7 | 231.8 | 231.3–233.4 | -2.5% | 59267 |
| std-control | 407.4 | 400.4–414.0 | 385.9–439.3 | -11.3% | 405.7 | 216.0 | 216.0–216.0 | -9.1% | 55193 |
| ordstack | 163.8 | 162.3–166.2 | 157.7–187.0 | -64.3% | 162.3 | 78.8 | 78.8–78.8 | -66.9% | 20109 |
| bumpalo | 248.4 | 246.0–252.2 | 235.0–295.8 | -45.9% | 246.5 | 138.0 | 138.0–138.0 | -42.0% | 35327 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 8; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1908.2 | 1908.2–1908.2 | +0.0% | 2197.1 | 2196.4–2200.3 | +0.0% | 1423.4 | 1409.1–1441.5 | +0.0% |
| std-mimalloc | 1415.0 | 1415.0–1415.0 | -25.8% | 1429.8 | 1429.8–1430.3 | -34.9% | 880.7 | 876.4–889.9 | -38.1% |
| std-control | 1744.5 | 1744.5–1744.5 | -8.6% | 2005.9 | 2005.5–2008.0 | -8.7% | 1236.3 | 1221.7–1241.1 | -13.1% |
| ordstack | 837.4 | 837.4–837.4 | -56.1% | 959.1 | 958.9–962.1 | -56.3% | 531.2 | 518.0–538.8 | -62.7% |
| bumpalo | 848.8 | 848.8–848.8 | -55.5% | 1062.7 | 1062.4–1066.8 | -51.6% | 822.6 | 813.4–835.7 | -42.2% |

### xform-120k (`serde_json_xform big.json out_std.json`)

- attempt 1: CPU 42; worst wall IQR 45.0% of median; loadavg 57.95/60.89/50.03 → 53.12/59.02/50.01; chosen core busy before {'42': 0.0}; SMT sibling busy during {}; machine busy before 16.2% / during 17.5% **(reported)**
- attempt 2: CPU 95; worst wall IQR 69.4% of median; loadavg 53.12/59.02/50.01 → 45.93/56.27/49.61; chosen core busy before {'95': 0.3}; SMT sibling busy during {}; machine busy before 16.5% / during 18.3%
- attempt 3: CPU 30; worst wall IQR 137.2% of median; loadavg 44.02/55.71/49.46 → 31.45/50.54/48.03; chosen core busy before {'30': 0.0}; SMT sibling busy during {}; machine busy before 17.6% / during 19.5%

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 294.8 | 292.1–298.6 | 288.2–304.4 | +0.0% | 293.0 | 57.8 | 57.8–57.8 | +0.0% | 14786 |
| std-mimalloc | 451.1 | 332.0–534.9 | 250.9–1509.5 | +53.0% | 449.5 | 48.1 | 45.9–55.4 | -16.7% | 8240 |
| std-control | 290.2 | 287.8–291.5 | 284.5–300.0 | -1.6% | 288.5 | 57.8 | 57.8–57.8 | +0.0% | 14785 |
| ordstack | 206.9 | 205.7–208.0 | 202.9–213.0 | -29.8% | 205.2 | 33.8 | 33.8–33.8 | -41.6% | 8549 |
| bumpalo | 213.0 | 212.2–215.7 | 210.0–233.2 | -27.7% | 211.3 | 51.0 | 51.0–51.0 | -11.7% | 13004 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 149.2 | 58.3 | 43.0 | 43.9 |
| std-mimalloc | 236.8 | 52.7 | 40.4 | 29.0 |
| std-control | 147.4 | 55.2 | 42.3 | 44.6 |
| ordstack | 118.4 | 39.5 | 37.6 | 10.9 |
| bumpalo | 122.7 | 41.4 | 40.7 | 8.6 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 2; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 1632.2 | 1632.2–1632.2 | +0.0% | 1747.9 | 1747.5–1780.4 | +0.0% | 965.3 | 962.5–985.7 | +0.0% |
| std-mimalloc | 1331.4 | 1331.4–1331.4 | -18.4% | 1374.2 | 1373.9–1407.4 | -21.4% | 626.7 | 619.6–663.8 | -35.1% |
| std-control | 1600.0 | 1600.0–1600.0 | -2.0% | 1715.6 | 1715.3–1747.0 | -1.8% | 957.7 | 947.4–973.0 | -0.8% |
| ordstack | 1313.0 | 1313.0–1313.0 | -19.6% | 1402.3 | 1401.9–1435.9 | -19.8% | 661.5 | 657.2–703.4 | -31.5% |
| bumpalo | 1285.9 | 1285.9–1285.9 | -21.2% | 1402.3 | 1401.7–1436.0 | -19.8% | 692.5 | 684.0–723.5 | -28.3% |

### xform-480k (`serde_json_xform big4.json out_std.json`)

- attempt 1: CPU 12; worst wall IQR 8.5% of median; loadavg 31.45/50.54/48.03 → 17.85/36.29/42.98; chosen core busy before {'12': 0.0}; SMT sibling busy during {}; machine busy before 18.9% / during 17.1%
- attempt 2: CPU 6; worst wall IQR 1.0% of median; loadavg 17.62/35.94/42.83 → 14.76/26.93/38.25; chosen core busy before {'6': 0.0}; SMT sibling busy during {}; machine busy before 15.8% / during 16.2% **(reported)**

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 1178.5 | 1174.4–1182.9 | 1169.3–1191.3 | +0.0% | 1176.4 | 226.5 | 226.5–226.5 | +0.0% | 58013 |
| std-mimalloc | 771.9 | 770.1–775.2 | 760.9–779.6 | -34.5% | 770.2 | 193.7 | 193.7–193.7 | -14.5% | 405 |
| std-control | 1160.1 | 1154.7–1166.0 | 1142.7–1517.1 | -1.6% | 1158.3 | 226.5 | 226.5–226.5 | +0.0% | 58013 |
| ordstack | 816.5 | 811.2–819.3 | 804.5–991.9 | -30.7% | 814.6 | 128.2 | 128.2–128.2 | -43.4% | 32988 |
| bumpalo | 845.3 | 842.7–847.8 | 838.0–868.7 | -28.3% | 843.5 | 198.0 | 198.0–198.0 | -12.6% | 50812 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 604.4 | 253.4 | 232.7 | 85.1 |
| std-mimalloc | 457.2 | 127.1 | 164.2 | 23.4 |
| std-control | 595.4 | 239.4 | 235.5 | 89.2 |
| ordstack | 479.9 | 155.7 | 153.0 | 28.8 |
| bumpalo | 494.0 | 168.4 | 163.3 | 19.5 |

perf stat (`instructions:u,instructions,cycles`, 7 rotated runs per variant on CPU 2; median, min–max):

| variant | instructions:u (M) | min–max | vs std | instructions (M) | min–max | vs std | cycles (M) | min–max | vs std |
|---|---|---|---|---|---|---|---|---|---|
| std | 6322.4 | 6322.4–6322.4 | +0.0% | 6783.8 | 6753.5–6787.1 | +0.0% | 3792.0 | 3749.8–4645.6 | +0.0% |
| std-mimalloc | 5349.2 | 5349.2–5349.2 | -15.4% | 5517.9 | 5484.4–5522.6 | -18.7% | 2523.6 | 2464.4–2951.2 | -33.4% |
| std-control | 6193.1 | 6193.1–6193.1 | -2.0% | 6654.6 | 6626.3–6665.5 | -1.9% | 3739.1 | 3697.2–3772.9 | -1.4% |
| ordstack | 5276.0 | 5276.0–5276.0 | -16.6% | 5632.3 | 5600.4–5646.0 | -17.0% | 2648.7 | 2624.5–2675.7 | -30.1% |
| bumpalo | 5167.2 | 5167.2–5167.2 | -18.3% | 5631.8 | 5601.0–5639.7 | -17.0% | 2759.1 | 2739.4–2813.3 | -27.2% |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo | instructions:u vs std / bumpalo | cycles vs std / bumpalo |
|---|---|---|---|---|---|---|---|---|---|
| treebuild-d7 | -63.7% | yes | yes | -52.5% | yes | -34.6% | -19.1% | -56.0% / -1.3% | -65.4% / -40.4% |
| treebuild-d8 | -64.3% | yes | yes | -66.9% | yes | -34.1% | -42.9% | -56.1% / -1.3% | -62.7% / -35.4% |
| xform-120k | -29.8% | yes | yes | -41.6% | yes | -2.9% | -33.8% | -19.6% / +2.1% | -31.5% / -4.5% |
| xform-480k | -30.7% | yes | yes | -43.4% | yes | -3.4% | -35.2% | -16.6% / +2.1% | -30.1% / -4.0% |

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

