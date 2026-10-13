//! Operation interpreter shared by the property test and the fuzz target.
//!
//! Bytes are decoded into operations on a vector; "descend" recurses into a
//! frame above the current vector, so the borrow chain is the call stack.
//! Each level also keeps frozen slices and boxes side by side in its frame.

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

fn ids(xs: &[Tracked<'_>]) -> Vec<u32> {
    xs.iter().map(|t| t.id).collect()
}

fn arg(input: &mut dyn Iterator<Item = u8>) -> usize {
    input.next().unwrap_or(0) as usize
}

fn level<'f, 'c>(
    frame: &mut Frame<'f>,
    input: &mut dyn Iterator<Item = u8>,
    depth: u32,
    cx: &'c Cx,
) {
    let mut frozen: Vec<(SSlice<'f, Tracked<'c>>, Vec<u32>)> = Vec::new();
    let mut boxes: Vec<(SBox<'f, Tracked<'c>>, u32)> = Vec::new();
    loop {
        let mut v: SVec<'_, 'f, Tracked<'c>> = SVec::new_in(frame);
        let mut m: Vec<u32> = Vec::new();
        let mut freeze = false;
        while let Some(op) = input.next() {
            match op % 16 {
                0..=5 => {
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
                        level(&mut up, input, depth + 1, cx);
                        assert_eq!(ids(elems), m, "a lower vector changed while frozen");
                    }
                }
                12 => break,
                13 => {
                    freeze = true;
                    break;
                }
                14 => {
                    let n = arg(input);
                    v.reserve(n);
                    assert!(v.capacity() >= m.len() + n);
                }
                _ => v.shrink_to_fit(),
            }
            assert_eq!(ids(&v), m);
        }
        if !freeze {
            cx.expect(m.iter().rev().copied());
            drop(v);
            break;
        }
        let s = v.freeze();
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

/// Runs one script. Panics on any disagreement with the model.
pub fn run(bytes: &[u8]) {
    let cx = Cx::new();
    {
        let mut st = Stack::with_reserve(1 << 22);
        let mut frame = st.frame();
        let mut input = bytes.iter().copied();
        level(&mut frame, &mut input, 0, &cx);
    }
    cx.finish();
}
