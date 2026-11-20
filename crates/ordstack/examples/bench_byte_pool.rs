//! Run with `cargo run --release -p ordstack --example bench_byte_pool -- 9`.

use ordstack::{ByteRun, SBytePool, Stack};
use std::{hint::black_box, time::Instant};

const CHURN: usize = 1024;
const SIZES: [usize; 4] = [32, 64, 96, 128];

fn ns_per_op(start: Instant, count: usize) -> f64 {
    start.elapsed().as_secs_f64() * 1e9 / count as f64
}

fn churn(pool: &mut SBytePool<'_, '_, '_, '_>, mut run: ByteRun, size: usize) -> (ByteRun, f64) {
    let start = Instant::now();
    for _ in 0..CHURN {
        assert!(pool.free(black_box(run)));
        run = black_box(pool.try_alloc(black_box(size)).unwrap());
    }
    (run, ns_per_op(start, CHURN))
}

fn measure(count: usize, rounds: usize) {
    let mut data = Stack::with_reserve(count * 256);
    let mut metadata = Stack::with_reserve(count * 256);
    let mut handles = Vec::with_capacity(count);
    let mut samples = [(); 6].map(|()| Vec::with_capacity(rounds));
    let mut extents = (0, 0);

    for _ in 0..rounds {
        handles.clear();
        let (mut bytes, mut runs) = (data.frame(), metadata.frame());
        let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
        let start = Instant::now();
        for i in 0..count {
            handles.push(black_box(pool.try_alloc(black_box(SIZES[i % 4])).unwrap()));
        }
        let allocate = ns_per_op(start, count);

        let start = Instant::now();
        let mut total = 0;
        // An odd stride visits each index once for these power-of-two sizes.
        for i in 0..count {
            let run = handles[i.wrapping_mul(4051) % count];
            total += black_box(pool.get(black_box(run)).unwrap()).len();
        }
        let lookup = ns_per_op(start, count);
        assert_eq!(total, count * 80);

        let (run, front) = churn(&mut pool, handles[0], SIZES[0]);
        handles[0] = run;
        let (run, tail) = churn(&mut pool, handles[count - 1], SIZES[3]);
        handles[count - 1] = run;

        let start = Instant::now();
        let mut run = run;
        for _ in 0..CHURN {
            run = black_box(pool.try_resize(black_box(run), 256).unwrap().unwrap());
            run = black_box(pool.try_resize(black_box(run), 128).unwrap().unwrap());
        }
        let resize = ns_per_op(start, 2 * CHURN);
        handles[count - 1] = run;

        let start = Instant::now();
        while let Some(run) = handles.pop() {
            assert!(pool.free(black_box(run)));
        }
        let free = ns_per_op(start, count);
        drop(pool);
        extents = (bytes.stack().extent(), runs.stack().extent());

        for (samples, time) in samples
            .iter_mut()
            .zip([allocate, lookup, free, front, tail, resize])
        {
            samples.push(time);
        }
    }

    print!("{count:>5}");
    for samples in &mut samples {
        samples.sort_unstable_by(f64::total_cmp);
        print!(" {:>10.1}", samples[samples.len() / 2]);
    }
    println!(" {:>10} {:>10}", extents.0, extents.1);
}

fn measure_fragmented(count: usize, rounds: usize) {
    let mut data = Stack::with_reserve(count * 256);
    let mut metadata = Stack::with_reserve(count * 256);
    let mut handles = Vec::with_capacity(count);
    let mut frees = Vec::with_capacity(rounds);
    let mut fills = Vec::with_capacity(rounds);
    let mut extents = (0, 0);

    for _ in 0..rounds {
        handles.clear();
        let (mut bytes, mut runs) = (data.frame(), metadata.frame());
        let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
        handles.extend((0..count).map(|i| pool.try_alloc(SIZES[i % 4]).unwrap()));
        let start = Instant::now();
        for &run in handles.iter().step_by(2) {
            assert!(pool.free(black_box(run)));
        }
        frees.push(ns_per_op(start, count / 2));
        // Skip 32-byte holes and partially fill every 96-byte hole.
        let start = Instant::now();
        for _ in 0..count / 4 {
            black_box(pool.try_alloc(black_box(64)).unwrap());
        }
        fills.push(ns_per_op(start, count / 4));
        drop(pool);
        extents = (bytes.stack().extent(), runs.stack().extent());
    }

    frees.sort_unstable_by(f64::total_cmp);
    fills.sort_unstable_by(f64::total_cmp);
    println!(
        "{count:>5} {:>10.1} {:>10.1} {:>10} {:>10}",
        frees[rounds / 2],
        fills[rounds / 2],
        extents.0,
        extents.1
    );
}

fn main() {
    let rounds = std::env::args().nth(1).map_or(9, |n| n.parse().unwrap());
    assert!(rounds > 0);
    println!("Median ns/op (front/tail count a free + alloc pair). Extents in bytes.");
    println!(
        " runs      alloc        get       free      front       tail     resize       data   metadata"
    );
    for count in [64, 256, 1024, 4096, 16384] {
        measure(count, rounds);
    }
    println!("\nFragmented: free alternating runs, then partially fill the larger holes.");
    println!(" runs       free       fill       data   metadata");
    for count in [64, 256, 1024, 4096, 16384] {
        measure_fragmented(count, rounds);
    }
}
