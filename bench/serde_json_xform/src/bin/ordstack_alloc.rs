// The original program (src/orig.rs) with its containers on one ordstack
// frame through `FrameAlloc` (feature `allocator-api2`): allocator-api2's
// `Vec` on `&FrameAlloc` wherever the original has a `Vec` or a `String`.
// No `unsafe`; serde_json is untouched. Comments marked PORT say what
// changed and why; lines marked BENCH are instrumentation and are not
// counted as source cost.
//
// Mirrors run_workloads.sh's py_json workload: load big.json (120,000
// records), do a nontrivial transform pass over every record, and
// serialize a new structure back out. Uses serde_json's DOM (serde_json::Value)
// for the transform step, not a typed struct, so the shape of allocation
// traffic -- lots of small heap Strings/Vecs/Maps built and torn down per
// record -- resembles the untyped-JSON style the C/perl/Python counterparts
// exercise, rather than a single big monomorphic Vec<Record>.
#![forbid(unsafe_code)]
use allocator_api2::alloc::Allocator;
use allocator_api2::vec::Vec;
use ordstack::Stack;
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{ser, Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write as _};

// PORT: allocator-api2 has no `String`; a string is its UTF-8 bytes.
struct Nested<A: Allocator> {
    a: i64,
    b: Vec<i64, A>,
}

struct Rec<A: Allocator> {
    id: i64,
    x: f64,
    name: Vec<u8, A>,
    tags: Vec<Vec<u8, A>, A>,
    nested: Nested<A>,
}

#[derive(Serialize)]
#[serde(bound = "")]
struct OutRec<A: Allocator> {
    id: i64,
    x_scaled: f64,
    // PORT: bytes are serialized as strings by two helpers (below).
    #[serde(serialize_with = "ser_str")]
    name_upper: Vec<u8, A>,
    tag_count: usize,
    #[serde(serialize_with = "ser_strs")]
    tags_sorted: Vec<Vec<u8, A>, A>,
    nested_sum: i64,
}

fn utf8<E: ser::Error>(b: &[u8]) -> Result<&str, E> {
    std::str::from_utf8(b).map_err(E::custom)
}

fn ser_str<A: Allocator, S: Serializer>(s: &Vec<u8, A>, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(utf8(s)?)
}

fn ser_strs<A: Allocator, S: Serializer>(
    v: &Vec<Vec<u8, A>, A>,
    ser: S,
) -> Result<S::Ok, S::Error> {
    let strs: Result<std::vec::Vec<&str>, S::Error> = v.iter().map(|s| utf8(s)).collect();
    ser.collect_seq(strs?)
}

// ---------------------------------------------------------------------------
// PORT: everything from here to `main` replaces two `#[derive(Deserialize)]`.
// `Deserialize::deserialize` has no parameter through which an allocator
// could be passed, so the parsed values can only be placed on the frame by
// hand-written `DeserializeSeed` impls that carry it. (Unlike the derive,
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

/// A JSON string -> its bytes, exactly sized, as `String`'s `visit_str` does.
struct StrSeed<A>(A);

impl<'de, A: Allocator + Copy> DeserializeSeed<'de> for StrSeed<A> {
    type Value = Vec<u8, A>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_str(self)
    }
}
impl<'de, A: Allocator + Copy> Visitor<'de> for StrSeed<A> {
    type Value = Vec<u8, A>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string")
    }
    fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
        let mut v = Vec::with_capacity_in(s.len(), self.0);
        v.extend_from_slice(s.as_bytes());
        Ok(v)
    }
}

/// A JSON array, element by element, as `Vec`'s `Deserialize` does.
struct SeqSeed<A, S>(A, S);

impl<'de, A: Allocator + Copy, S: DeserializeSeed<'de> + Copy> DeserializeSeed<'de>
    for SeqSeed<A, S>
{
    type Value = Vec<S::Value, A>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, A: Allocator + Copy, S: DeserializeSeed<'de> + Copy> Visitor<'de> for SeqSeed<A, S> {
    type Value = Vec<S::Value, A>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array")
    }
    fn visit_seq<Q: SeqAccess<'de>>(self, mut seq: Q) -> Result<Self::Value, Q::Error> {
        let mut v = Vec::new_in(self.0);
        while let Some(x) = seq.next_element_seed(self.1)? {
            v.push(x);
        }
        Ok(v)
    }
}

/// A plain value (an `i64` here), for `SeqSeed`.
struct Plain<T>(std::marker::PhantomData<T>);
impl<T> Clone for Plain<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Plain<T> {}
impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for Plain<T> {
    type Value = T;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<T, D::Error> {
        T::deserialize(d)
    }
}

impl<A: Copy> Clone for StrSeed<A> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A: Copy> Copy for StrSeed<A> {}

#[derive(Clone, Copy)]
struct NestedSeed<A>(A);

impl<'de, A: Allocator + Copy> DeserializeSeed<'de> for NestedSeed<A> {
    type Value = Nested<A>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, A: Allocator + Copy> Visitor<'de> for NestedSeed<A> {
    type Value = Nested<A>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Nested")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let (mut a, mut b) = (None, None);
        while let Some(k) = map.next_key()? {
            match k {
                NestedField::A => a = Some(map.next_value()?),
                NestedField::B => {
                    b = Some(map.next_value_seed(SeqSeed(self.0, Plain(Default::default())))?)
                }
                NestedField::Other => drop(map.next_value::<IgnoredAny>()?),
            }
        }
        Ok(Nested {
            a: a.ok_or_else(|| de::Error::missing_field("a"))?,
            b: b.ok_or_else(|| de::Error::missing_field("b"))?,
        })
    }
}

#[derive(Clone, Copy)]
struct RecSeed<A>(A);

impl<'de, A: Allocator + Copy> DeserializeSeed<'de> for RecSeed<A> {
    type Value = Rec<A>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, A: Allocator + Copy> Visitor<'de> for RecSeed<A> {
    type Value = Rec<A>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Rec")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let a = self.0;
        let (mut id, mut x, mut name, mut tags, mut nested) = (None, None, None, None, None);
        while let Some(k) = map.next_key()? {
            match k {
                RecField::Id => id = Some(map.next_value()?),
                RecField::X => x = Some(map.next_value()?),
                RecField::Name => name = Some(map.next_value_seed(StrSeed(a))?),
                RecField::Tags => tags = Some(map.next_value_seed(SeqSeed(a, StrSeed(a)))?),
                RecField::Nested => nested = Some(map.next_value_seed(NestedSeed(a))?),
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

fn main() {
    let args: std::vec::Vec<String> = env::args().collect();
    let in_path = args.get(1).expect("usage: serde_json_xform IN OUT");
    let out_path = args.get(2).expect("usage: serde_json_xform IN OUT");

    // PORT: a stack, a frame and its allocator, declared before anything on
    // them. Everything the original put in a `Vec` or `String` goes here.
    let mut st = Stack::new();
    let mut frame = st.frame();
    let alloc = frame.allocator();
    let a = &alloc;

    let t0 = std::time::Instant::now(); // BENCH
    let f = File::open(in_path).expect("open input");
    let reader = BufReader::new(f);
    // PORT: was `let recs: Vec<Rec> = serde_json::from_reader(reader)`.
    let mut de = serde_json::Deserializer::from_reader(reader);
    let recs = SeqSeed(a, RecSeed(a))
        .deserialize(&mut de)
        .expect("parse json");
    de.end().expect("parse json");
    let t1 = std::time::Instant::now(); // BENCH

    // Transform: build a new record per input record (fresh Strings/Vecs),
    // plus a side aggregation keyed by name-bucket (BTreeMap<String, Vec<i64>>)
    // so there is grouping/re-owning allocation traffic beyond a flat map.
    //
    // PORT: std's `BTreeMap` takes no allocator, so its nodes stay on the
    // heap; its keys and values are on the frame.
    let mut by_bucket: BTreeMap<Vec<u8, _>, Vec<i64, _>> = BTreeMap::new();
    let mut out: Vec<OutRec<_>, _> = Vec::with_capacity_in(recs.len(), a);
    for r in recs.into_iter() {
        let mut tags_sorted = r.tags.clone();
        tags_sorted.sort();
        // PORT: was `format!(..)`; `write!` into a byte vector.
        let mut bucket = Vec::new_in(a);
        write!(bucket, "b{}", r.nested.a).unwrap();
        by_bucket
            .entry(bucket)
            .or_insert_with(|| Vec::new_in(a))
            .push(r.id);
        // PORT: was `r.name.to_uppercase()`, which this spells out on bytes.
        let mut name_upper = Vec::with_capacity_in(r.name.len(), a);
        let name = std::str::from_utf8(&r.name).unwrap();
        for c in name.chars().flat_map(char::to_uppercase) {
            name_upper.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
        }
        out.push(OutRec {
            id: r.id,
            x_scaled: r.x * 2.0,
            name_upper,
            tag_count: tags_sorted.len(),
            tags_sorted,
            nested_sum: r.nested.b.iter().sum(),
        });
    }

    let t2 = std::time::Instant::now(); // BENCH
    let f_out = File::create(out_path).expect("create output");
    let writer = BufWriter::new(f_out);
    // PORT: was `&out`; allocator-api2's `Vec` is not `Serialize`, its
    // slice is.
    serde_json::to_writer(writer, &out[..]).expect("write json");
    phases(t0, t1, t2); // BENCH

    let total_bucket_rows: usize = by_bucket.values().map(|v| v.len()).sum();
    println!("{} {} {}", out.len(), by_bucket.len(), total_bucket_rows);
    // BENCH{ stack statistics, read once nothing on the stack is alive.
    if env::var_os("ORDSTACK_STATS").is_some() {
        drop(out);
        drop(by_bucket);
        eprintln!(
            "stack data high_water={} extent={}",
            st.high_water(),
            st.extent()
        );
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
