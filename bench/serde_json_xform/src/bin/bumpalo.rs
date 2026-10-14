// Port of serde_json_xform (src/orig.rs) onto bumpalo 3 (feature
// `collections`). It reuses the ordstack port's structure: the same
// hand-written `DeserializeSeed`s and the same three arenas (`out`, `recs`,
// `data`). No `unsafe`; serde_json is untouched. Comments marked PORT say
// what changed; lines marked BENCH are instrumentation and are not counted
// as source cost.
//
// Mirrors run_workloads.sh's py_json workload: load big.json (120,000
// records), do a nontrivial transform pass over every record, and
// serialize a new structure back out. Uses serde_json's DOM (serde_json::Value)
// for the transform step, not a typed struct, so the shape of allocation
// traffic -- lots of small heap Strings/Vecs/Maps built and torn down per
// record -- resembles the untyped-JSON style the C/perl/Python counterparts
// exercise, rather than a single big monomorphic Vec<Record>.
#![forbid(unsafe_code)]
use bumpalo::collections::{String as BString, Vec as BVec};
use bumpalo::Bump;
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter};

// PORT: the record types borrow blocks of the `data` arena.
struct Nested<'d> {
    a: i64,
    b: &'d [i64],
}

struct Rec<'d> {
    id: i64,
    x: f64,
    name: &'d str,
    tags: &'d [&'d str],
    nested: Nested<'d>,
}

#[derive(Serialize)]
struct OutRec<'o> {
    id: i64,
    x_scaled: f64,
    name_upper: &'o str,
    tag_count: usize,
    tags_sorted: &'o [&'o str],
    nested_sum: i64,
}

// ---------------------------------------------------------------------------
// PORT: everything from here to `main` replaces two `#[derive(Deserialize)]`.
// `Deserialize::deserialize` has no parameter through which an arena could
// be passed, so the parsed values can only be placed in arenas by
// hand-written `DeserializeSeed` impls that carry them. (Unlike the derive,
// these do not reject duplicate keys.)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum RecField {
    Id,
    X,
    Name,
    Tags,
    Nested,
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum NestedField {
    A,
    B,
    #[serde(other)]
    Other,
}

/// A JSON string -> `&str` in `data`.
struct StrSeed<'d>(&'d Bump);

impl<'de, 'd> DeserializeSeed<'de> for StrSeed<'d> {
    type Value = &'d str;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<&'d str, D::Error> {
        d.deserialize_str(self)
    }
}
impl<'de, 'd> Visitor<'de> for StrSeed<'d> {
    type Value = &'d str;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string")
    }
    fn visit_str<E: de::Error>(self, s: &str) -> Result<&'d str, E> {
        Ok(self.0.alloc_str(s))
    }
}

/// A JSON array of integers -> a vector in `data`.
struct IntsSeed<'d>(&'d Bump);

impl<'de, 'd> DeserializeSeed<'de> for IntsSeed<'d> {
    type Value = &'d [i64];
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'd> Visitor<'de> for IntsSeed<'d> {
    type Value = &'d [i64];
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of integers")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut v = BVec::new_in(self.0);
        while let Some(x) = seq.next_element()? {
            v.push(x);
        }
        Ok(v.into_bump_slice())
    }
}

/// A JSON array of strings -> a vector in `data`. Its strings are allocated
/// after it in the same arena, so when it grows it is copied.
struct TagsSeed<'d>(&'d Bump);

impl<'de, 'd> DeserializeSeed<'de> for TagsSeed<'d> {
    type Value = &'d [&'d str];
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'd> Visitor<'de> for TagsSeed<'d> {
    type Value = &'d [&'d str];
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of strings")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut v = BVec::new_in(self.0);
        while let Some(s) = seq.next_element_seed(StrSeed(self.0))? {
            v.push(s);
        }
        Ok(v.into_bump_slice())
    }
}

struct NestedSeed<'d>(&'d Bump);

impl<'de, 'd> DeserializeSeed<'de> for NestedSeed<'d> {
    type Value = Nested<'d>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'd> Visitor<'de> for NestedSeed<'d> {
    type Value = Nested<'d>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Nested")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let (mut a, mut b) = (None, None);
        while let Some(k) = map.next_key()? {
            match k {
                NestedField::A => a = Some(map.next_value()?),
                NestedField::B => b = Some(map.next_value_seed(IntsSeed(self.0))?),
                NestedField::Other => drop(map.next_value::<IgnoredAny>()?),
            }
        }
        Ok(Nested {
            a: a.ok_or_else(|| de::Error::missing_field("a"))?,
            b: b.ok_or_else(|| de::Error::missing_field("b"))?,
        })
    }
}

struct RecSeed<'d>(&'d Bump);

impl<'de, 'd> DeserializeSeed<'de> for RecSeed<'d> {
    type Value = Rec<'d>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'd> Visitor<'de> for RecSeed<'d> {
    type Value = Rec<'d>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Rec")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let (mut id, mut x, mut name, mut tags, mut nested) = (None, None, None, None, None);
        while let Some(k) = map.next_key()? {
            match k {
                RecField::Id => id = Some(map.next_value()?),
                RecField::X => x = Some(map.next_value()?),
                RecField::Name => name = Some(map.next_value_seed(StrSeed(self.0))?),
                RecField::Tags => tags = Some(map.next_value_seed(TagsSeed(self.0))?),
                RecField::Nested => nested = Some(map.next_value_seed(NestedSeed(self.0))?),
                RecField::Other => drop(map.next_value::<IgnoredAny>()?),
            }
        }
        Ok(Rec {
            id: id.ok_or_else(|| de::Error::missing_field("id"))?,
            x: x.ok_or_else(|| de::Error::missing_field("x"))?,
            name: name.ok_or_else(|| de::Error::missing_field("name"))?,
            tags: tags.ok_or_else(|| de::Error::missing_field("tags"))?,
            nested: nested.ok_or_else(|| de::Error::missing_field("nested"))?,
        })
    }
}

/// The top-level array, pushed into `recs`, which has an arena of its own
/// so that it is always the last allocation there and grows in place.
struct RecsSeed<'a, 'r, 'd> {
    recs: &'a mut BVec<'r, Rec<'d>>,
    data: &'d Bump,
}

impl<'de> DeserializeSeed<'de> for RecsSeed<'_, '_, '_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for RecsSeed<'_, '_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of records")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(rec) = seq.next_element_seed(RecSeed(self.data))? {
            self.recs.push(rec);
        }
        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let in_path = args.get(1).expect("usage: serde_json_xform IN OUT");
    let out_path = args.get(2).expect("usage: serde_json_xform IN OUT");

    // PORT: three arenas, as in the ordstack port. `out` holds what lives to
    // the end; `recs` the parsed record array; `data` what the parsed records
    // own (and the per-record bucket key).
    let out_bump = Bump::new();
    let mut recs_bump = Bump::new();
    let mut data_bump = Bump::new();

    let t0 = std::time::Instant::now(); // BENCH
    let f = File::open(in_path).expect("open input");
    let reader = BufReader::new(f);
    // PORT: was `let recs: Vec<Rec> = serde_json::from_reader(reader)`.
    let mut recs: BVec<Rec> = BVec::new_in(&recs_bump);
    let mut de = serde_json::Deserializer::from_reader(reader);
    RecsSeed {
        recs: &mut recs,
        data: &data_bump,
    }
    .deserialize(&mut de)
    .expect("parse json");
    de.end().expect("parse json");
    let t1 = std::time::Instant::now(); // BENCH

    // Transform: build a new record per input record (fresh Strings/Vecs),
    // plus a side aggregation keyed by name-bucket (BTreeMap<String, Vec<i64>>)
    // so there is grouping/re-owning allocation traffic beyond a flat map.
    //
    // PORT: `by_bucket` stays on the heap, as in the ordstack port, so that
    // the two ports differ only in the arena.
    let mut by_bucket: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut out: BVec<OutRec> = BVec::with_capacity_in(recs.len(), &out_bump);
    for r in recs.into_iter() {
        // PORT: was `r.tags.clone()`; the strings are copied into `out`.
        let mut tags_sorted =
            BVec::from_iter_in(r.tags.iter().map(|t| &*out_bump.alloc_str(t)), &out_bump);
        tags_sorted.sort();
        // PORT: was `by_bucket.entry(format!(..)).or_insert_with(Vec::new)`.
        // The key is formatted in `data` (reset after the loop); a heap
        // `String` is made only for a new bucket, as in the ordstack port.
        let bucket = bumpalo::format!(in &data_bump, "b{}", r.nested.a);
        match by_bucket.get_mut(bucket.as_str()) {
            Some(ids) => ids.push(r.id),
            None => drop(by_bucket.insert(String::from(bucket.as_str()), vec![r.id])),
        }
        // PORT: was `r.name.to_uppercase()`.
        let name_upper =
            BString::from_iter_in(r.name.chars().flat_map(char::to_uppercase), &out_bump);
        out.push(OutRec {
            id: r.id,
            x_scaled: r.x * 2.0,
            name_upper: name_upper.into_bump_str(),
            tag_count: tags_sorted.len(),
            tags_sorted: tags_sorted.into_bump_slice(),
            nested_sum: r.nested.b.iter().sum(),
        });
    }
    // BENCH{ arena statistics before the reset below.
    let stats = env::var_os("ORDSTACK_STATS").is_some();
    if stats {
        for (name, b) in [("recs", &recs_bump), ("data", &data_bump)] {
            eprintln!("bump {} allocated_bytes={}", name, b.allocated_bytes());
        }
    }
    // BENCH}
    // PORT: give the parsed data's chunks back, as the ordstack port does.
    recs_bump.reset();
    data_bump.reset();

    let t2 = std::time::Instant::now(); // BENCH
    let f_out = File::create(out_path).expect("create output");
    let writer = BufWriter::new(f_out);
    // PORT: was `&out`; bumpalo's `Vec` is `Serialize` only with its `serde`
    // feature, its slice always is.
    serde_json::to_writer(writer, &*out).expect("write json");
    phases(t0, t1, t2); // BENCH

    let total_bucket_rows: usize = by_bucket.values().map(|v| v.len()).sum();
    println!("{} {} {}", out.len(), by_bucket.len(), total_bucket_rows);
    // BENCH{
    if stats {
        eprintln!("bump out allocated_bytes={}", out_bump.allocated_bytes());
    }
    // BENCH}
}

// BENCH{ phase timer, identical in every variant; prints to stderr only.
fn phases(t0: std::time::Instant, t1: std::time::Instant, t2: std::time::Instant) {
    let t3 = std::time::Instant::now();
    if std::env::var_os("XFORM_PHASES").is_some() {
        let us = |a: std::time::Instant, b: std::time::Instant| (b - a).as_micros();
        eprintln!(
            "phases_us parse={} transform={} write={}",
            us(t0, t1),
            us(t1, t2),
            us(t2, t3)
        );
    }
}
// BENCH}
