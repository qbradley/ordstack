use ordstack::{SVec, Stack};

fn main() {
    let mut stack = Stack::with_reserve(1 << 16);
    let mut frame = stack.frame();
    let mut vector = SVec::from_iter_in([1u32], &mut frame);
    let first = &vector[0];
    vector.pop(); // ERROR
    assert_eq!(*first, 1);
}
