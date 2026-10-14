//! `SMap` and `SFrozenMap`.

use ordstack::map::Entry;
use ordstack::{AllocErrorKind, SMap, Stack};
use std::cell::{Cell, RefCell};
use std::hash::{BuildHasher, Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

/// Pushes its id to a shared log when dropped.
struct D(u32, Rc<RefCell<Vec<u32>>>);
impl Drop for D {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
    }
}
fn log() -> Rc<RefCell<Vec<u32>>> {
    Rc::new(RefCell::new(Vec::new()))
}

/// Hashes every key to `key % 4`: long probe runs, many collisions.
#[derive(Clone, Copy, Default)]
struct Collide;
struct CollideHasher(u64);
impl Hasher for CollideHasher {
    fn finish(&self) -> u64 {
        self.0 % 4
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = self.0.wrapping_mul(31).wrapping_add(*b as u64);
        }
    }
}
impl BuildHasher for Collide {
    type Hasher = CollideHasher;
    fn build_hasher(&self) -> CollideHasher {
        CollideHasher(0)
    }
}

/// `size_of` of one entry of an `SMap<u64, u64>`: hash, key, value.
const E64: usize = 24;

#[test]
fn insert_get_and_insertion_order() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<&str, u32> = SMap::new_in(&mut f);
    assert!(m.is_empty());
    assert_eq!(m.get("x"), None);
    for (i, w) in ["pear", "fig", "apple", "kiwi"].into_iter().enumerate() {
        assert_eq!(m.insert(w, i as u32), None);
    }
    assert_eq!(m.len(), 4);
    assert_eq!(m.get("apple"), Some(&2));
    assert_eq!(m.get_index_of("kiwi"), Some(3));
    assert_eq!(m.get_index(1), Some((&"fig", &1)));
    assert_eq!(m.get_index(4), None);
    assert!(m.contains_key("pear") && !m.contains_key("plum"));
    assert_eq!(
        m.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        [("pear", 0), ("fig", 1), ("apple", 2), ("kiwi", 3)]
    );
    assert_eq!(
        format!("{m:?}"),
        r#"{"pear": 0, "fig": 1, "apple": 2, "kiwi": 3}"#
    );
}

#[test]
fn insert_of_a_present_key_replaces_the_value_in_place() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m = SMap::new_in(&mut f);
    m.insert(1u32, D(10, l.clone()));
    m.insert(2, D(20, l.clone()));
    let old = m.insert(1, D(11, l.clone())).expect("present");
    assert_eq!(old.0, 10);
    assert!(l.borrow().is_empty()); // the map returned it; it did not drop it
    drop(old);
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), [1, 2]);
    assert_eq!(m.get(&1).map(|d| d.0), Some(11));
}

#[test]
fn borrowed_lookups() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<String, u32> = SMap::new_in(&mut f);
    m.insert("one".to_string(), 1);
    assert_eq!(m.get("one"), Some(&1));
    *m.get_mut("one").unwrap() += 1;
    assert_eq!(m.get("one"), Some(&2));
    *m.get_index_mut(0).unwrap().1 += 1;
    assert_eq!(m.get("one"), Some(&3));
}

#[test]
fn growth_extends_the_block_in_place() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let _kept = f.alloc(1u64); // 0..8
    let mut m: SMap<u64, u64> = SMap::new_in(&mut f);
    assert_eq!(m.capacity(), 0);
    for i in 0..7 {
        m.insert(i, i * 10);
    }
    assert_eq!(m.capacity(), 7);
    let first = m.get(&0).unwrap() as *const u64;
    // 7 entries and an 8-slot index.
    assert_eq!(m.above().stack().extent(), 8 + 7 * E64 + 8 * 4);
    m.insert(7, 70); // grows: 14 entries and a 16-slot index
    assert_eq!(m.capacity(), 14);
    assert_eq!(m.above().stack().extent(), 8 + 14 * E64 + 16 * 4);
    for i in 8..1000 {
        m.insert(i, i * 10);
    }
    assert_eq!(m.get(&0).unwrap() as *const u64, first); // never moved
    assert!((0..1000).all(|i| m.get(&i) == Some(&(i * 10))));
    assert!(m.keys().copied().eq(0..1000));
}

#[test]
fn with_capacity_and_reserve() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<u64, u64> = SMap::with_capacity_in(10, &mut f);
    assert_eq!(m.capacity(), 14);
    m.reserve(14);
    assert_eq!(m.capacity(), 14);
    m.reserve(15);
    assert_eq!(m.capacity(), 28);
    let e = m.try_reserve(usize::MAX).unwrap_err();
    assert_eq!(e.kind(), AllocErrorKind::CapacityOverflow);
    assert_eq!(m.capacity(), 28);
}

#[test]
fn an_exhausted_stack_leaves_the_map_unchanged() {
    let mut st = Stack::with_reserve(4096);
    let mut f = st.frame();
    let mut m: SMap<u64, u64> = SMap::new_in(&mut f);
    let mut n = 0;
    while m.try_reserve(1).is_ok() {
        m.insert(n, n);
        n += 1;
    }
    let e = m.try_reserve(1).unwrap_err();
    assert_eq!(e.kind(), AllocErrorKind::Exhausted);
    assert!((0..n).all(|i| m.get(&i) == Some(&i)));
    let r = catch_unwind(AssertUnwindSafe(|| m.insert(n, n)));
    let msg = *r.unwrap_err().downcast::<String>().unwrap();
    assert!(msg.starts_with("ordstack: "), "{msg}");
    assert!((0..n).all(|i| m.get(&i) == Some(&i)));
}

#[test]
fn swap_remove_moves_the_last_entry_into_the_hole() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<u32, u32> = SMap::new_in(&mut f);
    m.extend((0..5).map(|i| (i, i * 10)));
    assert_eq!(m.swap_remove(&1), Some(10));
    assert_eq!(m.swap_remove(&1), None);
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), [0, 4, 2, 3]);
    assert_eq!(m.swap_remove_index(3), Some((3, 30)));
    assert_eq!(m.swap_remove_index(3), None);
    assert!([0, 4, 2].iter().all(|k| m.get(k) == Some(&(k * 10))));
}

#[test]
fn shift_remove_keeps_the_order() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<u32, u32> = SMap::new_in(&mut f);
    m.extend((0..5).map(|i| (i, i * 10)));
    assert_eq!(m.shift_remove(&1), Some(10));
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), [0, 2, 3, 4]);
    assert_eq!(m.shift_remove_index(0), Some((0, 0)));
    assert_eq!(m.keys().copied().collect::<Vec<_>>(), [2, 3, 4]);
    assert_eq!(m.get_index_of(&4), Some(2));
    assert!([2, 3, 4].iter().all(|k| m.get(k) == Some(&(k * 10))));
}

/// Random inserts and removals with a colliding hasher, against a `Vec`
/// model of the insertion order.
#[test]
fn heavy_collisions_match_a_model() {
    let mut st = Stack::with_reserve(1 << 20);
    let mut f = st.frame();
    let mut m: SMap<u32, u32, Collide> = SMap::with_hasher_in(Collide, &mut f);
    let mut model: Vec<(u32, u32)> = Vec::new();
    let mut seed = 0x9E37_79B9u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let ops = if cfg!(miri) { 150 } else { 5000 };
    for step in 0..ops {
        let k = rnd() % 64;
        let pos = model.iter().position(|e| e.0 == k);
        match rnd() % 4 {
            0 | 1 => {
                let old = m.insert(k, step);
                match pos {
                    Some(p) => assert_eq!(old, Some(std::mem::replace(&mut model[p].1, step))),
                    None => {
                        assert_eq!(old, None);
                        model.push((k, step));
                    }
                }
            }
            2 => assert_eq!(m.swap_remove(&k), pos.map(|p| model.swap_remove(p).1)),
            _ => assert_eq!(m.shift_remove(&k), pos.map(|p| model.remove(p).1)),
        }
        assert!(m.iter().map(|(k, v)| (*k, *v)).eq(model.iter().copied()));
        assert!((0..64).all(|k| m.get(&k) == model.iter().find(|e| e.0 == k).map(|e| &e.1)));
    }
}

#[test]
fn entries_drop_back_to_front_and_clear_keeps_capacity() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    {
        let mut m = SMap::new_in(&mut f);
        for i in 0..4 {
            m.insert(i, D(i, l.clone()));
        }
        let cap = m.capacity();
        m.clear();
        assert_eq!(*l.borrow(), [3, 2, 1, 0]);
        assert_eq!((m.len(), m.capacity(), m.get(&0).is_none()), (0, cap, true));
        m.insert(9, D(9, l.clone()));
        m.insert(8, D(8, l.clone()));
    }
    assert_eq!(*l.borrow(), [3, 2, 1, 0, 8, 9]);
}

#[test]
fn iterators() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<u32, u32> = SMap::new_in(&mut f);
    m.extend([(1, 10), (2, 20), (3, 30)]);
    for (_, v) in &mut m {
        *v += 1;
    }
    for v in m.values_mut() {
        *v *= 2;
    }
    let mut seen = Vec::new();
    for (k, v) in &m {
        seen.push((*k, *v));
    }
    assert_eq!(seen, [(1, 22), (2, 42), (3, 62)]);
    assert_eq!(m.values().rev().copied().collect::<Vec<_>>(), [62, 42, 22]);
    assert_eq!(m.keys().len(), 3);
    assert_eq!(m.iter_mut().next_back().map(|(k, _)| *k), Some(3));
}

#[test]
fn order_does_not_depend_on_the_hasher() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut a: SMap<u32, ()> = SMap::new_in(&mut f);
    a.extend((0..50).rev().map(|k| (k, ())));
    let ka: Vec<u32> = a.keys().copied().collect();
    let mut up = a.above();
    let mut b: SMap<u32, (), Collide> = SMap::with_hasher_in(Collide, &mut up);
    b.extend((0..50).rev().map(|k| (k, ())));
    assert!(b.keys().copied().eq(ka.iter().copied()));
}

#[test]
fn zero_size_keys_and_values() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<(), ()> = SMap::new_in(&mut f);
    assert_eq!(m.insert((), ()), None);
    assert_eq!(m.insert((), ()), Some(()));
    assert_eq!(m.len(), 1);
    assert_eq!(m.swap_remove(&()), Some(()));
    assert!(m.is_empty());
}

#[test]
fn allocating_above_the_map_and_growing_again() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<u64, u64> = SMap::new_in(&mut f);
    m.insert(1, 1);
    let end = {
        let mut up = m.above();
        let b = up.alloc_slice_copy(&[7u8; 300]);
        assert_eq!(b[299], 7);
        up.end_offset()
    };
    assert_eq!(end, 7 * E64 + 8 * 4 + 300);
    for i in 2..100 {
        m.insert(i, i); // grows over the reclaimed 300 bytes
    }
    assert!((1..100).all(|i| m.get(&i) == Some(&i)));
}

/// A key whose `Hash` or `Eq` panics on demand.
struct Touchy {
    k: u32,
    panic_hash: bool,
}
impl Hash for Touchy {
    fn hash<H: Hasher>(&self, h: &mut H) {
        assert!(!self.panic_hash, "hash panicked");
        self.k.hash(h);
    }
}
impl PartialEq for Touchy {
    fn eq(&self, o: &Touchy) -> bool {
        assert!(self.k != 99 && o.k != 99, "eq panicked");
        self.k == o.k
    }
}
impl Eq for Touchy {}

#[test]
fn a_panicking_hash_or_eq_leaves_the_map_unchanged() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<Touchy, u32, Collide> = SMap::with_hasher_in(Collide, &mut f);
    for k in 0..6 {
        m.insert(
            Touchy {
                k,
                panic_hash: false,
            },
            k,
        );
    }
    let key = |k, panic_hash| Touchy { k, panic_hash };
    assert!(catch_unwind(AssertUnwindSafe(|| m.insert(key(6, true), 6))).is_err());
    // 99 collides with keys already present, so `eq` runs and panics.
    assert!(catch_unwind(AssertUnwindSafe(|| m.insert(key(99, false), 99))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| m.swap_remove(&key(99, false)))).is_err());
    assert_eq!(m.len(), 6);
    assert!((0..6).all(|k| m.get(&key(k, false)) == Some(&k)));
    m.insert(key(6, false), 6);
    assert_eq!(m.get_index_of(&key(6, false)), Some(6));
}

/// Panics when dropped if its id is 1.
struct Bomb(u32, Rc<RefCell<Vec<u32>>>);
impl Drop for Bomb {
    fn drop(&mut self) {
        self.1.borrow_mut().push(self.0);
        assert!(self.0 != 1, "bomb");
    }
}

#[test]
fn a_panicking_destructor_in_clear_drops_the_rest_once() {
    let l = log();
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m = SMap::new_in(&mut f);
    for i in 0..4 {
        m.insert(i, Bomb(i, l.clone()));
    }
    assert!(catch_unwind(AssertUnwindSafe(|| m.clear())).is_err());
    assert_eq!(*l.borrow(), [3, 2, 1, 0]);
    assert!(m.is_empty() && m.get(&0).is_none());
    m.insert(5, Bomb(5, l.clone()));
    assert_eq!(m.len(), 1);
}

#[test]
fn entry_api() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<&str, u32> = SMap::new_in(&mut f);
    *m.entry("a").or_insert(1) += 10;
    assert_eq!(m.get("a"), Some(&11));
    let calls = Cell::new(0);
    m.entry("a").or_insert_with(|| {
        calls.set(calls.get() + 1);
        0
    });
    assert_eq!(calls.get(), 0); // not called for a present key
    *m.entry("b").or_default() += 2;
    assert_eq!(m.entry("b").key(), &"b");
    match m.entry("a") {
        Entry::Occupied(mut e) => {
            assert_eq!((e.key(), e.index(), *e.get()), (&"a", 0, 11));
            *e.get_mut() = 12;
        }
        Entry::Vacant(_) => unreachable!(),
    }
    match m.entry("c") {
        Entry::Vacant(e) => {
            assert_eq!(e.key(), &"c");
            *e.insert(5) += 1;
        }
        Entry::Occupied(_) => unreachable!(),
    }
    assert_eq!(
        m.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
        [("a", 12), ("b", 2), ("c", 6)]
    );
}
