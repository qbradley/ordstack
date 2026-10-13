//! Operation interpreter shared by the property test and the fuzz target.
//!
//! Bytes are decoded into operations on a vector; "descend" recurses into a
//! frame above the current vector, so the borrow chain is the call stack.
//! Each level also keeps frozen slices and boxes side by side in its frame.
//!
//! Byte decoding (the operation is `byte % 24`; an argument byte, written
//! `arg`, is the next byte, or 0 once the input is exhausted):
//!
//! | op        | operation                                                    |
//! |-----------|--------------------------------------------------------------|
//! | 0..=5, 21..=23 | push a fresh value                                      |
//! | 6         | pop                                                          |
//! | 7         | truncate to `arg % (len + 1)`                                |
//! | 8         | insert a fresh value at `arg % (len + 1)`                    |
//! | 9         | remove at `arg % len` (nothing if empty)                     |
//! | 10        | retain by a mask derived from `arg`                          |
//! | 11        | descend: `split_above` and run the rest in the new frame     |
//! | 12        | depth > 0: leave this level. Depth 0: drop the vector, start a fresh one |
//! | 13        | freeze the vector in place, then allocate a box              |
//! | 14        | reserve `arg` more                                           |
//! | 15        | shrink_to_fit                                                |
//! | 16        | swap_remove at `arg % len` (nothing if empty)                |
//! | 17        | clear                                                        |
//! | 18        | extend with `arg % 8` fresh values                           |
//! | 19        | move the elements (reversed) into a vector on a second stack, `freeze_into` this level's frame, then allocate a box |
//! | 20        | `split_above`, allocate `arg % 4 + 1` rounds of boxes, slices and strings in a `sub` frame, leave it, `reset_above` |

use ordstack::{Frame, SBox, SSlice, SVec, Stack};
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
        while let Some(op) = input.next() {
            match op % 24 {
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
                    let run: Vec<Tracked<'c>> = (0..arg(input) % 8).map(|_| cx.make()).collect();
                    m.extend(run.iter().map(|t| t.id));
                    v.extend(run);
                }
                19 => {
                    next = Next::FreezeViaScratch;
                    break;
                }
                _ => {
                    let rounds = arg(input) % 4 + 1;
                    let (elems, mut up) = v.split_above();
                    let end = up.end_offset();
                    sub_scope(&mut up, rounds, cx);
                    assert_eq!(up.end_offset(), end, "a sub-frame moved its parent");
                    up.reset_above();
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
                continue;
            }
            Next::Freeze => v.freeze(),
            Next::FreezeViaScratch => {
                // Move the elements one by one, so the copy is reversed.
                let mut tmp: SVec<'_, 's, Tracked<'c>> = SVec::with_capacity_in(m.len(), scratch);
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
        for (s, m) in &frozen {
            assert_eq!(ids(s), *m, "a frozen slice changed");
        }
        for (b, id) in &boxes {
            assert_eq!(b.id, *id, "a frozen box changed");
        }
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

/// Allocates boxes, slices and strings in a `sub` frame of `up`, checks them,
/// and drops them in a fixed order before leaving it.
fn sub_scope<'c>(up: &mut Frame<'_>, rounds: usize, cx: &'c Cx) {
    let mut sub = up.sub();
    let mut live: Vec<(SBox<'_, Tracked<'c>>, u32)> = Vec::new();
    let mut slices: Vec<(SSlice<'_, Tracked<'c>>, Vec<u32>)> = Vec::new();
    for r in 0..rounds {
        let t = cx.make();
        let id = t.id;
        live.push((sub.alloc(t), id));
        let mut w: SVec<'_, '_, Tracked<'c>> = SVec::new_in(&mut sub);
        let mut wm = Vec::new();
        for _ in 0..=r {
            let t = cx.make();
            wm.push(t.id);
            w.push(t);
        }
        slices.push((w.freeze(), wm));
        let nums = [r as u32, 7, 9];
        assert_eq!(*sub.alloc_slice_copy(&nums), nums);
        assert_eq!(&*sub.alloc_str("sub"), "sub");
    }
    for (b, id) in &live {
        assert_eq!(b.id, *id, "a sub-frame box changed");
    }
    for (s, m) in &slices {
        assert_eq!(ids(s), *m, "a sub-frame slice changed");
    }
    for (s, m) in slices.into_iter().rev() {
        cx.expect(m.iter().rev().copied());
        drop(s);
    }
    for (b, id) in live.into_iter().rev() {
        cx.expect([id]);
        drop(b);
    }
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
