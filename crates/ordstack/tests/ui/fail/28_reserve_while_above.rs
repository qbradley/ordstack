use ordstack::{SVec, Stack};

fn main() {
    let mut stack = Stack::with_reserve(1 << 16);
    let mut frame = stack.frame();
    let mut lower: SVec<u32> = SVec::new_in(&mut frame);
    let above = lower.above();
    lower.reserve(1); // ERROR
    assert_eq!(above.end_offset(), 0);
}
