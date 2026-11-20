//! Run with `cargo run --release -p ordstack --example bench_bplus -- 32768 12`.

use ordstack::{SBPlusTree, Stack};
use std::{collections::BTreeMap, hint::black_box, time::Instant};

fn measure(name: &str, keys: &[u64], rounds: usize, mut build: impl FnMut(&[u64]) -> [f64; 5]) {
    let mut samples = [(); 5].map(|()| Vec::with_capacity(rounds));
    for _ in 0..rounds {
        for (samples, time) in samples.iter_mut().zip(build(keys)) {
            samples.push(time * 1e9 / keys.len() as f64);
        }
    }
    print!("{name}");
    for (label, times) in ["insert", "hit", "miss", "scan", "remove"]
        .into_iter()
        .zip(&mut samples)
    {
        times.sort_unstable_by(f64::total_cmp);
        print!(" {label}={:.2}", times[times.len() / 2]);
    }
    println!(" ns/key");
}

fn operations<T>(
    tree: &mut T,
    keys: &[u64],
    mut insert: impl FnMut(&mut T, u64),
    mut lookup: impl FnMut(&T, u64) -> Option<u64>,
    mut scan: impl FnMut(&T) -> u64,
    mut remove: impl FnMut(&mut T, u64) -> u64,
) -> [f64; 5] {
    let start = Instant::now();
    for &key in keys {
        insert(tree, black_box(key));
    }
    let inserted = start.elapsed().as_secs_f64();
    let expected = keys.iter().fold(0u64, |sum, &key| sum.wrapping_add(key));
    let start = Instant::now();
    let mut sum = 0u64;
    for &key in keys.iter().rev() {
        sum = sum.wrapping_add(black_box(lookup(tree, black_box(key))).unwrap());
    }
    let looked_up = start.elapsed().as_secs_f64();
    assert_eq!(sum, expected);
    let start = Instant::now();
    let mut unexpected_hits = 0;
    for &key in keys.iter().rev() {
        unexpected_hits += usize::from(black_box(lookup(tree, black_box(key - 1))).is_some());
    }
    let missed = start.elapsed().as_secs_f64();
    assert_eq!(unexpected_hits, 0);
    let start = Instant::now();
    let sum = black_box(scan(tree));
    let scanned = start.elapsed().as_secs_f64();
    assert_eq!(sum, expected);
    let start = Instant::now();
    let mut sum = 0u64;
    for &key in keys.iter().rev() {
        sum = sum.wrapping_add(black_box(remove(tree, black_box(key))));
    }
    let removed = start.elapsed().as_secs_f64();
    assert_eq!(sum, expected);
    [inserted, looked_up, missed, scanned, removed]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let count = args.next().map_or(32768, |n| n.parse::<usize>().unwrap());
    let rounds = args.next().map_or(12, |n| n.parse::<usize>().unwrap());
    let stack_only = args.next().as_deref() == Some("stack");
    assert!(count > 0 && rounds > 0);
    // Put one absent odd key before every even key in the tree.
    let mut keys: Vec<u64> = (1..=count as u64).map(|i| 2 * i).collect();
    let mut seed = 0x1234_5678_9abc_def0u64;
    for i in (1..keys.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        keys.swap(i, seed as usize % (i + 1));
    }
    let mut stack = Stack::with_reserve(count.checked_mul(512).unwrap());
    for (order, keys) in [
        ("random", keys.as_slice()),
        ("sorted", &{
            let mut sorted = keys.clone();
            sorted.sort_unstable();
            sorted
        }),
    ] {
        measure(&format!("{order} stack 8/11"), keys, rounds, |keys| {
            let mut frame = stack.frame();
            let mut tree = SBPlusTree::<_, _, 8, 11>::with_fanout_in(&mut frame);
            operations(
                &mut tree,
                keys,
                |tree, key| {
                    tree.try_insert(key, (key, !key)).unwrap();
                },
                |tree, key| tree.get(key).map(|value| value.0),
                |tree| tree.iter().fold(0u64, |sum, (_, v)| sum.wrapping_add(v.0)),
                |tree, key| tree.remove(key).unwrap().0,
            )
        });
        if stack_only {
            continue;
        }
        measure(&format!("{order} std"), keys, rounds, |keys| {
            operations(
                &mut BTreeMap::new(),
                keys,
                |tree, key| {
                    tree.insert(key, (key, !key));
                },
                |tree, key| tree.get(&key).map(|value| value.0),
                |tree| tree.values().fold(0u64, |sum, v| sum.wrapping_add(v.0)),
                |tree, key| tree.remove(&key).unwrap().0,
            )
        });
    }
}
