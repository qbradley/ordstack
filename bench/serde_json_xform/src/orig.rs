// Mirrors run_workloads.sh's py_json workload: load big.json (120,000
// records), do a nontrivial transform pass over every record, and
// serialize a new structure back out. Uses serde_json's DOM (serde_json::Value)
// for the transform step, not a typed struct, so the shape of allocation
// traffic -- lots of small heap Strings/Vecs/Maps built and torn down per
// record -- resembles the untyped-JSON style the C/perl/Python counterparts
// exercise, rather than a single big monomorphic Vec<Record>.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(Deserialize)]
struct Nested {
    a: i64,
    b: Vec<i64>,
}

#[derive(Deserialize)]
struct Rec {
    id: i64,
    x: f64,
    name: String,
    tags: Vec<String>,
    nested: Nested,
}

#[derive(Serialize)]
struct OutRec {
    id: i64,
    x_scaled: f64,
    name_upper: String,
    tag_count: usize,
    tags_sorted: Vec<String>,
    nested_sum: i64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let in_path = args.get(1).expect("usage: serde_json_xform IN OUT");
    let out_path = args.get(2).expect("usage: serde_json_xform IN OUT");

    let t0 = std::time::Instant::now(); // BENCH
    let f = File::open(in_path).expect("open input");
    let reader = BufReader::new(f);
    let recs: Vec<Rec> = serde_json::from_reader(reader).expect("parse json");
    let t1 = std::time::Instant::now(); // BENCH

    // Transform: build a new record per input record (fresh Strings/Vecs),
    // plus a side aggregation keyed by name-bucket (BTreeMap<String, Vec<i64>>)
    // so there is grouping/re-owning allocation traffic beyond a flat map.
    let mut by_bucket: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut out: Vec<OutRec> = Vec::with_capacity(recs.len());
    for r in recs.into_iter() {
        let mut tags_sorted = r.tags.clone();
        tags_sorted.sort();
        let bucket = format!("b{}", r.nested.a);
        by_bucket.entry(bucket).or_insert_with(Vec::new).push(r.id);
        out.push(OutRec {
            id: r.id,
            x_scaled: r.x * 2.0,
            name_upper: r.name.to_uppercase(),
            tag_count: tags_sorted.len(),
            tags_sorted,
            nested_sum: r.nested.b.iter().sum(),
        });
    }

    let t2 = std::time::Instant::now(); // BENCH
    let f_out = File::create(out_path).expect("create output");
    let writer = BufWriter::new(f_out);
    serde_json::to_writer(writer, &out).expect("write json");
    phases(t0, t1, t2); // BENCH

    let total_bucket_rows: usize = by_bucket.values().map(|v| v.len()).sum();
    println!("{} {} {}", out.len(), by_bucket.len(), total_bucket_rows);
}

// BENCH{ phase timer, identical in every variant; prints to stderr only.
fn phases(t0: std::time::Instant, t1: std::time::Instant, t2: std::time::Instant) {
    let t3 = std::time::Instant::now();
    if std::env::var_os("XFORM_PHASES").is_some() {
        let us = |a: std::time::Instant, b: std::time::Instant| (b - a).as_micros();
        eprintln!("phases_us parse={} transform={} write={}", us(t0, t1), us(t1, t2), us(t2, t3));
    }
}
// BENCH}
