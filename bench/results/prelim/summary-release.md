Runs per (variant, input): 31 timed after 3 warm-up rounds, pinned to CPU 11.

### treebuild-d7 (`treebuild 7`)

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 99.2 | 98.0–102.3 | 95.4–121.6 | +0.0% | 97.6 | 47.2 | 47.2–47.2 | +0.0% | 12090 |
| std-mimalloc | 61.0 | 59.9–62.5 | 58.8–72.0 | -38.5% | 59.5 | 51.7 | 51.7–51.7 | +9.4% | 328 |
| std-control | 85.7 | 84.7–87.4 | 83.1–92.3 | -13.6% | 84.1 | 42.8 | 42.8–42.8 | -9.5% | 10946 |
| ordstack | 34.4 | 33.7–35.1 | 32.2–38.7 | -65.4% | 32.9 | 21.6 | 21.6–21.7 | -54.2% | 4090 |
| bumpalo | 55.9 | 55.0–57.4 | 53.6–62.7 | -43.7% | 54.4 | 27.8 | 27.8–27.8 | -41.3% | 7101 |

perf stat (`instructions,cycles`, median of 5 runs):

| variant | instructions (M) | cycles (M) | IPC |
|---|---|---|---|
| std | 437.9 | 295.1 | 1.48 |
| std-mimalloc | 752.3 | 1043.9 | 0.72 |
| std-control | 399.7 | 242.7 | 1.65 |
| ordstack | 192.0 | 96.5 | 1.99 |
| bumpalo | 212.5 | 158.9 | 1.34 |

### treebuild-d8 (`treebuild 8`)

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 468.8 | 461.3–476.3 | 453.0–491.2 | +0.0% | 467.3 | 237.8 | 237.8–237.8 | +0.0% | 60917 |
| std-mimalloc | 292.5 | 286.3–296.5 | 275.5–311.0 | -37.6% | 291.0 | 235.7 | 235.7–235.7 | -0.9% | 425 |
| std-control | 403.2 | 395.8–415.2 | 388.4–554.4 | -14.0% | 401.3 | 216.0 | 216.0–216.0 | -9.1% | 55195 |
| ordstack | 163.3 | 160.5–165.0 | 157.9–169.4 | -65.2% | 161.9 | 78.8 | 78.8–78.8 | -66.9% | 20111 |
| bumpalo | 255.4 | 252.1–256.8 | 244.1–309.1 | -45.5% | 253.8 | 138.0 | 138.0–138.0 | -42.0% | 35329 |

perf stat (`instructions,cycles`, median of 5 runs):

| variant | instructions (M) | cycles (M) | IPC |
|---|---|---|---|
| std | 2197.1 | 1367.6 | 1.61 |
| std-mimalloc | 4162.9 | 6242.4 | 0.67 |
| std-control | 2006.3 | 1291.1 | 1.55 |
| ordstack | 959.1 | 524.1 | 1.83 |
| bumpalo | 1064.0 | 828.0 | 1.28 |

### xform-120k (`serde_json_xform big.json out_std.json`)

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 297.5 | 291.4–300.8 | 286.4–367.6 | +0.0% | 295.9 | 57.8 | 57.8–57.8 | +0.0% | 14790 |
| std-mimalloc | 440.8 | 229.8–1132.2 | 192.7–1478.5 | +48.1% | 439.4 | 48.2 | 45.9–55.7 | -16.6% | 7863 |
| std-control | 295.5 | 291.1–299.4 | 282.6–376.1 | -0.7% | 293.8 | 57.8 | 57.8–57.8 | +0.0% | 14790 |
| ordstack | 209.0 | 206.9–209.6 | 204.6–210.5 | -29.7% | 207.4 | 33.8 | 33.8–33.8 | -41.6% | 8553 |
| bumpalo | 213.6 | 212.2–214.4 | 209.7–215.8 | -28.2% | 212.1 | 51.0 | 51.0–51.0 | -11.7% | 13008 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 149.3 | 59.1 | 44.2 | 44.1 |
| std-mimalloc | 199.7 | 69.8 | 41.7 | 14.6 |
| std-control | 148.2 | 55.8 | 44.8 | 46.3 |
| ordstack | 119.1 | 39.9 | 38.3 | 10.9 |
| bumpalo | 123.0 | 41.3 | 40.6 | 8.3 |

perf stat (`instructions,cycles`, median of 5 runs):

| variant | instructions (M) | cycles (M) | IPC |
|---|---|---|---|
| std | 1747.8 | 976.3 | 1.79 |
| std-mimalloc | 1746.0 | 1375.0 | 1.27 |
| std-control | 1716.8 | 941.9 | 1.82 |
| ordstack | 1402.2 | 668.9 | 2.10 |
| bumpalo | 1402.1 | 690.3 | 2.03 |

### xform-480k (`serde_json_xform big4.json out_std.json`)

| variant | wall median ms | wall IQR ms | wall min–max ms | vs std | user+sys median ms | peak RSS median MB | RSS min–max MB | vs std | minor faults |
|---|---|---|---|---|---|---|---|---|---|
| std | 1158.7 | 1143.2–1180.5 | 1124.7–1247.1 | +0.0% | 1157.1 | 226.5 | 226.5–226.5 | +0.0% | 58016 |
| std-mimalloc | 3276.9 | 2859.1–3675.2 | 2342.1–5734.2 | +182.8% | 3274.7 | 181.8 | 164.1–187.0 | -19.8% | 49618 |
| std-control | 1148.4 | 1133.6–1163.6 | 1113.7–1267.2 | -0.9% | 1146.8 | 226.5 | 226.5–226.5 | +0.0% | 58016 |
| ordstack | 812.2 | 806.7–816.0 | 798.8–842.7 | -29.9% | 810.7 | 128.2 | 128.2–128.2 | -43.4% | 32990 |
| bumpalo | 837.2 | 830.8–840.5 | 823.7–862.7 | -27.7% | 835.1 | 198.0 | 198.0–198.0 | -12.6% | 50815 |

In-program phase timer (medians, ms; `rest` = wall − the three phases: process start, teardown, exit):

| variant | parse | transform | write | rest |
|---|---|---|---|---|
| std | 599.1 | 251.7 | 227.9 | 80.5 |
| std-mimalloc | 2102.3 | 829.7 | 165.1 | 47.5 |
| std-control | 591.8 | 238.9 | 230.3 | 87.1 |
| ordstack | 478.1 | 155.7 | 152.0 | 26.2 |
| bumpalo | 490.8 | 163.5 | 164.2 | 16.4 |

perf stat (`instructions,cycles`, median of 5 runs):

| variant | instructions (M) | cycles (M) | IPC |
|---|---|---|---|
| std | 6783.1 | 3725.6 | 1.82 |
| std-mimalloc | 7603.8 | 6490.3 | 1.17 |
| std-control | 6653.7 | 3754.1 | 1.77 |
| ordstack | 5631.0 | 2659.6 | 2.12 |
| bumpalo | 5629.9 | 2709.5 | 2.08 |

### Reading rule, applied mechanically

| input | ordstack time vs std | IQRs disjoint | time ≥10% better | ordstack RSS vs std | RSS ≥10% better | time vs bumpalo | RSS vs bumpalo |
|---|---|---|---|---|---|---|---|
| treebuild-d7 | -65.4% | yes | yes | -54.2% | yes | -38.5% | -22.0% |
| treebuild-d8 | -65.2% | yes | yes | -66.9% | yes | -36.1% | -42.9% |
| xform-120k | -29.7% | yes | yes | -41.6% | yes | -2.1% | -33.8% |
| xform-480k | -29.9% | yes | yes | -43.4% | yes | -3.0% | -35.2% |

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

