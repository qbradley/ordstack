// Port of serde_json_xform (src/orig.rs) onto ordstack 0.1. Adapted from the
// stackfit spike's port. No `unsafe`, no Rc, no leaking; serde_json is
// untouched. Comments marked PORT say what the API forced; lines marked
// BENCH are instrumentation and are not counted as source cost.
//
// Mirrors run_workloads.sh's py_json workload: load big.json (120,000
// records), do a nontrivial transform pass over every record, and
// serialize a new structure back out. Uses serde_json's DOM (serde_json::Value)
// for the transform step, not a typed struct, so the shape of allocation
// traffic -- lots of small heap Strings/Vecs/Maps built and torn down per
// record -- resembles the untyped-JSON style the C/perl/Python counterparts
// exercise, rather than a single big monomorphic Vec<Record>.
#![forbid(unsafe_code)]
use ordstack::{Frame, SFixedVec, SSlice, SStr, SString, SVec, Stack};
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
use std::env;
use std::fmt::{self, Write as _};
use std::fs::File;
use std::io::{BufReader, BufWriter};

// PORT: the record types hold frozen blocks of the `data` stack.
struct Nested<'d> {
    a: i64,
    b: SSlice<'d, i64>,
}

struct Rec<'d> {
    id: i64,
    x: f64,
    name: SStr<'d>,
    tags: SSlice<'d, SStr<'d>>,
    nested: Nested<'d>,
}

#[derive(Serialize)]
struct OutRec<'o> {
    id: i64,
    x_scaled: f64,
    // PORT: ordstack's handles do not implement `Serialize`; two
    // `serialize_with` helpers (below) stand in.
    #[serde(serialize_with = "ser_str")]
    name_upper: SStr<'o>,
    tag_count: usize,
    #[serde(serialize_with = "ser_strs")]
    tags_sorted: SSlice<'o, SStr<'o>>,
    nested_sum: i64,
}

fn ser_str<S: Serializer>(s: &SStr<'_>, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(s)
}

fn ser_strs<S: Serializer>(v: &SSlice<'_, SStr<'_>>, ser: S) -> Result<S::Ok, S::Error> {
    ser.collect_seq(v.iter().map(|s| s.as_str()))
}

// ---------------------------------------------------------------------------
// PORT: everything from here to `main` replaces two `#[derive(Deserialize)]`.
// `Deserialize::deserialize` has no parameter through which a stack could be
// passed, so the parsed values can only be placed on stacks by hand-written
// `DeserializeSeed` impls that carry the frames. (Unlike the derive, these do
// not reject duplicate keys.)
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

/// A JSON string -> frozen `SStr` on `data`.
struct StrSeed<'a, 'd>(&'a mut Frame<'d>);

impl<'de, 'd> DeserializeSeed<'de> for StrSeed<'_, 'd> {
    type Value = SStr<'d>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<SStr<'d>, D::Error> {
        d.deserialize_str(self)
    }
}
impl<'de, 'd> Visitor<'de> for StrSeed<'_, 'd> {
    type Value = SStr<'d>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string")
    }
    fn visit_str<E: de::Error>(self, s: &str) -> Result<SStr<'d>, E> {
        Ok(self.0.alloc_str(s))
    }
}

/// A JSON array of integers -> grown in place on top of `data`, then frozen
/// where it is.
struct IntsSeed<'a, 'd>(&'a mut Frame<'d>);

impl<'de, 'd> DeserializeSeed<'de> for IntsSeed<'_, 'd> {
    type Value = SSlice<'d, i64>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'd> Visitor<'de> for IntsSeed<'_, 'd> {
    type Value = SSlice<'d, i64>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of integers")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut v = SVec::new_in(self.0);
        while let Some(x) = seq.next_element()? {
            v.push(x);
        }
        Ok(v.freeze())
    }
}

/// A JSON array of strings. The vector of handles grows on `scratch` while
/// each string is frozen on `data`; when complete it is copied to `data`.
struct TagsSeed<'a, 's, 'd> {
    scratch: &'a mut Frame<'s>,
    data: &'a mut Frame<'d>,
}

impl<'de, 'd> DeserializeSeed<'de> for TagsSeed<'_, '_, 'd> {
    type Value = SSlice<'d, SStr<'d>>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de, 'd> Visitor<'de> for TagsSeed<'_, '_, 'd> {
    type Value = SSlice<'d, SStr<'d>>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of strings")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut v = SVec::new_in(self.scratch);
        while let Some(s) = seq.next_element_seed(StrSeed(&mut *self.data))? {
            v.push(s);
        }
        Ok(v.freeze_into(self.data))
    }
}

struct NestedSeed<'a, 'd>(&'a mut Frame<'d>);

impl<'de, 'd> DeserializeSeed<'de> for NestedSeed<'_, 'd> {
    type Value = Nested<'d>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'd> Visitor<'de> for NestedSeed<'_, 'd> {
    type Value = Nested<'d>;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("struct Nested")
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let (mut a, mut b) = (None, None);
        while let Some(k) = map.next_key()? {
            match k {
                NestedField::A => a = Some(map.next_value()?),
                NestedField::B => b = Some(map.next_value_seed(IntsSeed(&mut *self.0))?),
                NestedField::Other => drop(map.next_value::<IgnoredAny>()?),
            }
        }
        Ok(Nested {
            a: a.ok_or_else(|| de::Error::missing_field("a"))?,
            b: b.ok_or_else(|| de::Error::missing_field("b"))?,
        })
    }
}

struct RecSeed<'a, 's, 'd> {
    scratch: &'a mut Frame<'s>,
    data: &'a mut Frame<'d>,
}

impl<'de, 'd> DeserializeSeed<'de> for RecSeed<'_, '_, 'd> {
    type Value = Rec<'d>;
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
        d.deserialize_map(self)
    }
}
impl<'de, 'd> Visitor<'de> for RecSeed<'_, '_, 'd> {
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
                RecField::Name => name = Some(map.next_value_seed(StrSeed(&mut *self.data))?),
                RecField::Tags => {
                    let seed = TagsSeed {
                        scratch: &mut *self.scratch,
                        data: &mut *self.data,
                    };
                    tags = Some(map.next_value_seed(seed)?)
                }
                RecField::Nested => {
                    nested = Some(map.next_value_seed(NestedSeed(&mut *self.data))?)
                }
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

/// The top-level array, pushed into `recs` (which grows in place: it is the
/// top of its stack between records, and frozen under the per-record scratch
/// frame while a record is being parsed).
struct RecsSeed<'a, 'v, 'r, 'd> {
    recs: &'a mut SVec<'v, 'r, Rec<'d>>,
    data: &'a mut Frame<'d>,
}

impl<'de> DeserializeSeed<'de> for RecsSeed<'_, '_, '_, '_> {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<(), D::Error> {
        d.deserialize_seq(self)
    }
}
impl<'de> Visitor<'de> for RecsSeed<'_, '_, '_, '_> {
    type Value = ();
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of records")
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        loop {
            let seed = RecSeed {
                scratch: &mut self.recs.above(),
                data: &mut *self.data,
            };
            match seq.next_element_seed(seed)? {
                Some(rec) => self.recs.push(rec),
                None => return Ok(()),
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let in_path = args.get(1).expect("usage: serde_json_xform IN OUT");
    let out_path = args.get(2).expect("usage: serde_json_xform IN OUT");

    // PORT: three stacks. `out` holds what lives to the end; `recs` the
    // parsed record array (plus per-record scratch above it); `data` what the
    // parsed records own (plus a per-iteration scratch string above it).
    let mut out_stack = Stack::new();
    let mut recs_stack = Stack::new();
    let mut data_stack = Stack::new();
    let mut out_frame = out_stack.frame();
    let mut recs_frame = recs_stack.frame();
    let mut data = data_stack.frame();

    let t0 = std::time::Instant::now(); // BENCH
    let f = File::open(in_path).expect("open input");
    let reader = BufReader::new(f);
    // PORT: was `let recs: Vec<Rec> = serde_json::from_reader(reader)`.
    let mut recs: SVec<Rec> = SVec::new_in(&mut recs_frame);
    let mut de = serde_json::Deserializer::from_reader(reader);
    RecsSeed {
        recs: &mut recs,
        data: &mut data,
    }
    .deserialize(&mut de)
    .expect("parse json");
    de.end().expect("parse json");
    let t1 = std::time::Instant::now(); // BENCH
    let recs_bytes = recs.capacity() * std::mem::size_of::<Rec>(); // BENCH

    // Transform: build a new record per input record (fresh Strings/Vecs),
    // plus a side aggregation keyed by name-bucket (BTreeMap<String, Vec<i64>>)
    // so there is grouping/re-owning allocation traffic beyond a flat map.
    //
    // PORT (not expressible): `by_bucket` stays on the heap. Its seven
    // `Vec<i64>` values grow round-robin, so none of them is the top of any
    // stack when it needs to grow.
    let mut by_bucket: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    // PORT: `out` never grew in the original (`Vec::with_capacity(recs.len())`),
    // so it is a fixed-capacity frozen block; that lets the blocks its
    // elements own be allocated above it on the same stack.
    let mut out: SFixedVec<OutRec> = SFixedVec::with_capacity_in(recs.len(), &mut out_frame);
    // PORT: was `recs.into_iter()`; an `SVec` is not `IntoIterator`, its
    // frozen slice is.
    for r in recs.freeze().into_iter() {
        // PORT: was `r.tags.clone()`. `Clone` cannot be given a frame, so the
        // deep copy is spelled out (exact size known: no scratch needed).
        let mut tags_sorted = SFixedVec::with_capacity_in(r.tags.len(), &mut out_frame);
        for t in r.tags.iter() {
            tags_sorted.push(out_frame.alloc_str(t));
        }
        let mut tags_sorted = tags_sorted.into_slice();
        tags_sorted.sort();
        // PORT: was `by_bucket.entry(format!(..)).or_insert_with(Vec::new)`.
        // The key is formatted on top of `data` and popped at the end of the
        // iteration; a heap `String` is made only for a new bucket.
        let mut bucket = SString::new_in(&mut data);
        write!(bucket, "b{}", r.nested.a).unwrap();
        match by_bucket.get_mut(&*bucket) {
            Some(ids) => ids.push(r.id),
            None => drop(by_bucket.insert(String::from(&*bucket), vec![r.id])),
        }
        // PORT: was `r.name.to_uppercase()`; grows in place on top of `out`.
        let mut name_upper = SString::new_in(&mut out_frame);
        name_upper.extend(r.name.chars().flat_map(char::to_uppercase));
        out.push(OutRec {
            id: r.id,
            x_scaled: r.x * 2.0,
            name_upper: name_upper.freeze(),
            tag_count: tags_sorted.len(),
            tags_sorted,
            nested_sum: r.nested.b.iter().sum(),
        });
    }
    // BENCH{ stack statistics before the explicit reset below.
    let stats = env::var_os("ORDSTACK_STATS").is_some();
    if stats {
        for (name, st) in [("recs", &recs_stack), ("data", &data_stack)] {
            eprintln!(
                "stack {} high_water={} extent={}",
                name,
                st.high_water(),
                st.extent()
            );
        }
        eprintln!("recs_vec_capacity_bytes={}", recs_bytes);
    }
    // BENCH}
    // PORT: the parsed records are gone (the loop consumed `recs`); popping
    // what they owned has to be asked for, because a frame's blocks are only
    // reclaimed when the thing beneath it is next used. These two lines do
    // not compile if anything parsed is still referenced.
    recs_stack.reset();
    data_stack.reset();

    let t2 = std::time::Instant::now(); // BENCH
    let f_out = File::create(out_path).expect("create output");
    let writer = BufWriter::new(f_out);
    // PORT: was `&out`; an `SFixedVec` is not `Serialize`, its slice is.
    serde_json::to_writer(writer, &*out).expect("write json");
    phases(t0, t1, t2); // BENCH

    let total_bucket_rows: usize = by_bucket.values().map(|v| v.len()).sum();
    println!("{} {} {}", out.len(), by_bucket.len(), total_bucket_rows);
    // BENCH{
    if stats {
        drop(out);
        let st = &out_stack;
        eprintln!(
            "stack out high_water={} extent={}",
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
