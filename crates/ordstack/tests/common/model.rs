//! Operation interpreter shared by the property test and the fuzz target.
//!
//! Bytes are decoded into operations on a vector; "descend" recurses into a
//! frame above the current vector, so the borrow chain is the call stack.
//! Each level also keeps frozen slices and boxes side by side in its frame.
//!
//! Byte decoding (the operation is `byte % 25`; an argument byte, written
//! `arg`, is the next byte, or 0 once the input is exhausted):
//!
//! | op        | operation                                                    |
//! |-----------|--------------------------------------------------------------|
//! | 0..=5, 21..=22 | push a fresh value (and 23 without the `allocator-api2` feature) |
//! | 6         | pop                                                          |
//! | 7         | truncate to `arg % (len + 1)`                                |
//! | 8         | insert a fresh value at `arg % (len + 1)`                    |
//! | 9         | remove at `arg % len` (nothing if empty)                     |
//! | 10        | retain by a mask derived from `arg`                          |
//! | 11        | descend: `split_above` and run the rest in the new frame     |
//! | 12        | depth > 0: leave this level. Depth 0: drop the vector, start a fresh one |
//! | 13        | freeze the vector in place, allocate a box, run a `sub_scope` on the level's frame |
//! | 14        | reserve `arg` more                                           |
//! | 15        | shrink_to_fit                                                |
//! | 16        | swap_remove at `arg % len`; the argument is read only if the vector is non-empty |
//! | 17        | clear                                                        |
//! | 18        | extend with `arg % 8` fresh values; if `arg & 8`, from an iterator with no exact size hint |
//! | 19        | move the elements (reversed) into a vector on a second stack, `freeze_into` this level's frame, then allocate a box; the source vector has `arg % 4` spare slots |
//! | 20        | `split_above`, then `arg % 4 + 1` rounds in `sub` scopes (see `sub_scope`), `reset_above`, allocate again |
//! | 23        | with the `allocator-api2` feature: `split_above`, then two allocator-api2 vectors on one `FrameAlloc` in the new frame (see `alloc_scope`) |
//! | 24        | `split_above`, then map operations on a map in the new frame until the script ends the map (see `map_scope`) |

use ordstack::{Frame, SBox, SMap, SSlice, SVec, Stack};
use std::cell::{Cell, RefCell};

/// Hands out ids and records drops.
pub struct Cx {
    next: Cell<u32>,
    dropped: RefCell<Vec<u32>>,
    expected: RefCell<Vec<u32>>,
}

pub struct Tracked<'c> {
    id: u32,
    cx: &'c Cx,
}

impl Drop for Tracked<'_> {
    fn drop(&mut self) {
        self.cx.dropped.borrow_mut().push(self.id);
    }
}

impl Cx {
    fn new() -> Cx {
        Cx {
            next: Cell::new(0),
            dropped: RefCell::new(Vec::new()),
            expected: RefCell::new(Vec::new()),
        }
    }

    fn make(&self) -> Tracked<'_> {
        let id = self.next.get();
        self.next.set(id + 1);
        Tracked { id, cx: self }
    }

    /// Records that these ids must be dropped next, in this order.
    fn expect(&self, ids: impl IntoIterator<Item = u32>) {
        self.expected.borrow_mut().extend(ids);
    }

    fn finish(&self) {
        let dropped = self.dropped.borrow();
        assert_eq!(
            *dropped,
            *self.expected.borrow(),
            "drop order differs from the model"
        );
        let mut seen = dropped.clone();
        seen.sort_unstable();
        assert!(
            seen.iter().copied().eq(0..self.next.get()),
            "a value was dropped twice or never"
        );
    }
}

const MAX_DEPTH: u32 = 6;

/// Scripts longer than this are cut off: longer ones can exhaust the fixed
/// stack and the per-operation model comparison makes them quadratic.
pub const MAX_LEN: usize = 4096;

fn ids(xs: &[Tracked<'_>]) -> Vec<u32> {
    xs.iter().map(|t| t.id).collect()
}

fn arg(input: &mut dyn Iterator<Item = u8>) -> usize {
    input.next().unwrap_or(0) as usize
}

fn level<'f, 's, 'c>(
    frame: &mut Frame<'f>,
    scratch: &mut Frame<'s>,
    input: &mut dyn Iterator<Item = u8>,
    depth: u32,
    cx: &'c Cx,
) {
    enum Next {
        End,
        Restart,
        Freeze,
        FreezeViaScratch,
    }
    let mut frozen: Vec<(SSlice<'f, Tracked<'c>>, Vec<u32>)> = Vec::new();
    let mut boxes: Vec<(SBox<'f, Tracked<'c>>, u32)> = Vec::new();
    loop {
        let mut v: SVec<'_, 'f, Tracked<'c>> = SVec::new_in(frame);
        let mut m: Vec<u32> = Vec::new();
        let mut next = Next::End;
        let mut spare = 0;
        while let Some(op) = input.next() {
            match op % 25 {
                #[cfg(feature = "allocator-api2")]
                23 => {
                    let (elems, mut up) = v.split_above();
                    alloc_scope(&mut up, input, cx);
                    assert_eq!(ids(elems), m, "a lower vector changed under an allocator");
                }
                0..=5 | 21..=23 => {
                    let t = cx.make();
                    m.push(t.id);
                    v.push(t);
                }
                6 => match v.pop() {
                    Some(t) => {
                        assert_eq!(m.pop(), Some(t.id));
                        cx.expect([t.id]);
                    }
                    None => assert!(m.is_empty()),
                },
                7 => {
                    let n = arg(input) % (m.len() + 1);
                    cx.expect(m.drain(n..).rev());
                    v.truncate(n);
                }
                8 => {
                    let i = arg(input) % (m.len() + 1);
                    let t = cx.make();
                    m.insert(i, t.id);
                    v.insert(i, t);
                }
                9 => {
                    if !m.is_empty() {
                        let i = arg(input) % m.len();
                        let t = v.remove(i);
                        assert_eq!(m.remove(i), t.id);
                        cx.expect([t.id]);
                    }
                }
                10 => {
                    let mask = arg(input) as u32 | 1;
                    let keep = |id: u32| id.wrapping_mul(mask) & 2 == 0;
                    cx.expect(m.iter().copied().filter(|&id| !keep(id)));
                    m.retain(|&id| keep(id));
                    v.retain(|t| keep(t.id));
                }
                11 => {
                    if depth < MAX_DEPTH {
                        let (elems, mut up) = v.split_above();
                        level(&mut up, scratch, input, depth + 1, cx);
                        assert_eq!(ids(elems), m, "a lower vector changed while frozen");
                    }
                }
                12 => {
                    if depth == 0 {
                        next = Next::Restart;
                    }
                    break;
                }
                13 => {
                    next = Next::Freeze;
                    break;
                }
                14 => {
                    let n = arg(input);
                    v.reserve(n);
                    assert!(v.capacity() >= m.len() + n);
                }
                15 => v.shrink_to_fit(),
                16 => {
                    if !m.is_empty() {
                        let i = arg(input) % m.len();
                        let t = v.swap_remove(i);
                        assert_eq!(m.swap_remove(i), t.id);
                        cx.expect([t.id]);
                    }
                }
                17 => {
                    cx.expect(m.iter().rev().copied());
                    m.clear();
                    v.clear();
                }
                18 => {
                    let a = arg(input);
                    let n = (a % 8) as u32;
                    let first = cx.next.get();
                    m.extend(first..first + n);
                    let fresh = (0..n).map(|_| cx.make());
                    if a & 8 == 0 {
                        v.extend(fresh);
                    } else {
                        // No exact size hint: the lower bound is 0.
                        v.extend(fresh.filter(|_| true));
                    }
                    assert_eq!(cx.next.get(), first + n);
                }
                19 => {
                    spare = arg(input) % 4;
                    next = Next::FreezeViaScratch;
                    break;
                }
                24 => {
                    let (elems, mut up) = v.split_above();
                    map_scope(&mut up, scratch, input, cx);
                    assert_eq!(ids(elems), m, "a lower vector changed under a map");
                }
                _ => {
                    let rounds = arg(input) % 4 + 1;
                    let (elems, mut up) = v.split_above();
                    sub_scope(&mut up, rounds, cx);
                    assert_eq!(ids(elems), m, "a lower vector changed under a sub-frame");
                }
            }
            assert_eq!(ids(&v), m);
        }
        let s = match next {
            Next::End => {
                cx.expect(m.iter().rev().copied());
                drop(v);
                break;
            }
            Next::Restart => {
                cx.expect(m.iter().rev().copied());
                drop(v);
                sub_scope(frame, (frozen.len() + boxes.len()) % 3 + 1, cx);
                recheck(&frozen, &boxes);
                continue;
            }
            Next::Freeze => v.freeze(),
            Next::FreezeViaScratch => {
                // Move the elements one by one, so the copy is reversed.
                let tmp: SVec<'_, 's, Tracked<'c>> =
                    SVec::with_capacity_in(m.len() + spare, scratch);
                while let Some(t) = v.pop() {
                    tmp.push(t);
                }
                drop(v);
                m.reverse();
                assert_eq!(ids(&tmp), m);
                tmp.freeze_into(frame)
            }
        };
        frozen.push((s, m));
        let t = cx.make();
        let id = t.id;
        boxes.push((frame.alloc(t), id));
        recheck(&frozen, &boxes);
        sub_scope(frame, (frozen.len() + boxes.len()) % 3 + 1, cx);
        recheck(&frozen, &boxes);
    }
    for (b, id) in boxes {
        cx.expect([id]);
        drop(b);
    }
    for (s, m) in frozen {
        cx.expect(m.iter().rev().copied());
        drop(s);
    }
}

fn recheck<'f, 'c>(
    frozen: &[(SSlice<'f, Tracked<'c>>, Vec<u32>)],
    boxes: &[(SBox<'f, Tracked<'c>>, u32)],
) {
    for (s, m) in frozen {
        assert_eq!(ids(s), *m, "a frozen slice changed");
    }
    for (b, id) in boxes {
        assert_eq!(b.id, *id, "a frozen box changed");
    }
}

/// A box and a frozen slice of tracked values, with their model.
struct Blocks<'f, 'c> {
    b: SBox<'f, Tracked<'c>>,
    bid: u32,
    s: SSlice<'f, Tracked<'c>>,
    sm: Vec<u32>,
}

impl<'f, 'c> Blocks<'f, 'c> {
    fn new(f: &mut Frame<'f>, n: usize, cx: &'c Cx) -> Blocks<'f, 'c> {
        let t = cx.make();
        let bid = t.id;
        let b = f.alloc(t);
        let w: SVec<'_, 'f, Tracked<'c>> = SVec::new_in(f);
        let mut sm = Vec::new();
        for _ in 0..n {
            let t = cx.make();
            sm.push(t.id);
            w.push(t);
        }
        Blocks {
            b,
            bid,
            s: w.freeze(),
            sm,
        }
    }

    fn check(&self) {
        assert_eq!(self.b.id, self.bid, "a block changed");
        assert_eq!(ids(&self.s), self.sm, "a block slice changed");
    }

    /// Drops the slice, then the box.
    fn drop_in_order(self, cx: &Cx) {
        cx.expect(self.sm.iter().rev().copied());
        drop(self.s);
        cx.expect([self.bid]);
        drop(self.b);
    }
}

/// Exercises `Frame::sub` and `reset_above` on `up`, which owns blocks of its
/// own: allocates blocks in `up`, runs `rounds` of boxes, slices, copies and
/// strings in a `sub` frame (re-reading everything afterwards), leaves it,
/// calls `reset_above`, allocates in `up` again, re-reads all of it, and drops
/// what it allocated in a fixed order. `up.end_offset()` must not move during
/// the sub-frame.
fn sub_scope<'c>(up: &mut Frame<'_>, rounds: usize, cx: &'c Cx) {
    let old = Blocks::new(up, rounds, cx);
    let end = up.end_offset();
    {
        let mut sub = up.sub();
        let mut live: Vec<Blocks<'_, 'c>> = Vec::new();
        let mut copies = Vec::new();
        let mut strs = Vec::new();
        for r in 0..rounds {
            live.push(Blocks::new(&mut sub, r + 1, cx));
            let nums = [r as u32, 7, 9];
            copies.push((sub.alloc_slice_copy(&nums), nums));
            let text = format!("sub{r}");
            strs.push((sub.alloc_str(&text), text));
        }
        old.check();
        for l in &live {
            l.check();
        }
        for (c, nums) in &copies {
            assert_eq!(**c, *nums, "a sub-frame copy changed");
        }
        for (t, text) in &strs {
            assert_eq!(&**t, text.as_str(), "a sub-frame str changed");
        }
        for l in live.into_iter().rev() {
            l.drop_in_order(cx);
        }
    }
    assert_eq!(up.end_offset(), end, "a sub-frame moved its parent");
    up.reset_above();
    let fresh = Blocks::new(up, rounds + 1, cx);
    old.check();
    fresh.check();
    fresh.drop_in_order(cx);
    old.check();
    old.drop_in_order(cx);
}

/// Checks a map's entries, in order, and a lookup of every key against the
/// model.
fn check_map(entries: Vec<(u8, u32)>, get: impl Fn(u8) -> Option<u32>, m: &[(u8, u32)]) {
    assert_eq!(entries, m, "map entries differ from the model");
    for k in 0..32 {
        let want = m.iter().find(|e| e.0 == k).map(|e| e.1);
        assert_eq!(get(k), want, "map lookup differs from the model");
    }
}

/// Runs operations on an `SMap<u8, Tracked>` on top of `up`, against a
/// model of its entries in insertion order. Keys are `arg % 32`, so they
/// collide often. Decoding (`byte % 8`; `arg` as in `level`):
///
/// | op   | operation                                                        |
/// |------|------------------------------------------------------------------|
/// | 0..=2 | insert a fresh value under a key (replacing drops the old one)  |
/// | 3    | `swap_remove` a key                                              |
/// | 4    | `shift_remove` a key                                             |
/// | 5    | `entry(key).or_insert_with(fresh)`                               |
/// | 6    | allocate a box above the map, check it, drop it                  |
/// | 7    | end the map: by `arg % 3`, drop it; `freeze` it and allocate a box beside it; or `freeze_into` the scratch frame |
///
/// A script that runs out ends the map by dropping it.
fn map_scope<'c>(
    up: &mut Frame<'_>,
    scratch: &mut Frame<'_>,
    input: &mut dyn Iterator<Item = u8>,
    cx: &'c Cx,
) {
    let mut map: SMap<'_, '_, u8, Tracked<'c>> = SMap::new_in(up);
    let mut m: Vec<(u8, u32)> = Vec::new();
    let mut end = 0;
    let key = |input: &mut dyn Iterator<Item = u8>| (arg(input) % 32) as u8;
    while let Some(op) = input.next() {
        match op % 8 {
            0..=2 => {
                let k = key(input);
                let t = cx.make();
                let id = t.id;
                match m.iter_mut().find(|e| e.0 == k) {
                    Some(e) => {
                        let old = map.insert(k, t).expect("the model has this key");
                        assert_eq!(old.id, std::mem::replace(&mut e.1, id));
                        cx.expect([old.id]);
                    }
                    None => {
                        assert!(map.insert(k, t).is_none());
                        m.push((k, id));
                    }
                }
            }
            3 => {
                let k = key(input);
                let want = m.iter().position(|e| e.0 == k).map(|p| m.swap_remove(p).1);
                let got = map.swap_remove(&k);
                assert_eq!(got.as_ref().map(|t| t.id), want);
                cx.expect(want);
            }
            4 => {
                let k = key(input);
                let want = m.iter().position(|e| e.0 == k).map(|p| m.remove(p).1);
                let got = map.shift_remove(&k);
                assert_eq!(got.as_ref().map(|t| t.id), want);
                cx.expect(want);
            }
            5 => {
                let k = key(input);
                let before = cx.next.get();
                let id = map.entry(k).or_insert_with(|| cx.make()).id;
                match m.iter().find(|e| e.0 == k) {
                    Some(e) => {
                        assert_eq!(e.1, id);
                        assert_eq!(
                            cx.next.get(),
                            before,
                            "or_insert_with ran for a present key"
                        );
                    }
                    None => m.push((k, id)),
                }
            }
            6 => {
                let t = cx.make();
                let id = t.id;
                let b = map.above().alloc(t);
                assert_eq!(b.id, id);
                cx.expect([id]);
                drop(b);
            }
            _ => {
                end = arg(input) % 3;
                break;
            }
        }
        check_map(
            map.iter().map(|(k, t)| (*k, t.id)).collect(),
            |k| map.get(&k).map(|t| t.id),
            &m,
        );
    }
    match end {
        0 => {
            cx.expect(m.iter().rev().map(|e| e.1));
            drop(map);
        }
        1 => {
            let fm = map.freeze();
            let t = cx.make();
            let id = t.id;
            let b = up.alloc(t);
            check_map(
                fm.iter().map(|(k, t)| (*k, t.id)).collect(),
                |k| fm.get(&k).map(|t| t.id),
                &m,
            );
            assert_eq!(b.id, id, "a box beside a frozen map changed");
            cx.expect([id]);
            drop(b);
            cx.expect(m.iter().rev().map(|e| e.1));
            drop(fm);
        }
        _ => {
            let fm = map.freeze_into(scratch);
            check_map(
                fm.iter().map(|(k, t)| (*k, t.id)).collect(),
                |k| fm.get(&k).map(|t| t.id),
                &m,
            );
            cx.expect(m.iter().rev().map(|e| e.1));
            drop(fm);
        }
    }
}

/// Two `allocator_api2` vectors on one adapter, driven until the script
/// ends the scope. Op `b % 8` acts on vector `(b / 8) % 2`: 0..=2 push,
/// 3 pop, 4 reserve `arg % 64`, 5 shrink_to_fit, 6 clear, 7 end (then
/// `arg % 2` names the vector dropped first). Vectors drop front to back.
#[cfg(feature = "allocator-api2")]
fn alloc_scope<'c>(up: &mut Frame<'_>, input: &mut dyn Iterator<Item = u8>, cx: &'c Cx) {
    use allocator_api2::vec::Vec as AVec;
    let a = up.allocator();
    let mut vs: [AVec<Tracked<'c>, _>; 2] = [AVec::new_in(&a), AVec::new_in(&a)];
    let mut ms: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    let mut first = 0;
    while let Some(op) = input.next() {
        let i = (op as usize / 8) % 2;
        match op % 8 {
            0..=2 => {
                let t = cx.make();
                ms[i].push(t.id);
                vs[i].push(t);
            }
            3 => {
                if let Some(t) = vs[i].pop() {
                    let id = ms[i].pop().unwrap();
                    assert_eq!(t.id, id);
                    cx.expect([id]);
                    drop(t);
                }
            }
            4 => vs[i].reserve(arg(input) % 64),
            5 => vs[i].shrink_to_fit(),
            6 => {
                cx.expect(ms[i].drain(..));
                vs[i].clear();
            }
            _ => {
                first = arg(input) % 2;
                break;
            }
        }
        for j in 0..2 {
            assert_eq!(
                ids(&vs[j]),
                ms[j],
                "an allocator-api2 vector differs from the model"
            );
        }
    }
    let [v0, v1] = vs;
    let (a_first, a_second) = if first == 0 { (v0, v1) } else { (v1, v0) };
    let (m_first, m_second) = (ms[first].clone(), ms[1 - first].clone());
    cx.expect(m_first);
    drop(a_first);
    cx.expect(m_second);
    drop(a_second);
}

/// Runs one script. Panics on any disagreement with the model.
///
/// Inputs are expected to be at most [`MAX_LEN`] (4096) bytes: longer scripts
/// can exhaust the fixed stack and are quadratic. To stay total for any
/// input, bytes beyond that are ignored.
pub fn run(bytes: &[u8]) {
    let cx = Cx::new();
    {
        let mut st = Stack::with_reserve(1 << 22);
        let mut frame = st.frame();
        let mut other = Stack::with_reserve(1 << 20);
        let mut scratch = other.frame();
        let mut input = bytes.iter().copied().take(MAX_LEN);
        level(&mut frame, &mut scratch, &mut input, 0, &cx);
    }
    cx.finish();
}
