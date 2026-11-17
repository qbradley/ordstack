#![allow(unused)]
use ordstack::{SVec, Stack};

fn main() {
    let mut stack = Stack::with_reserve(1 << 16);
    let mut frame = stack.frame();
    let mut vector = SVec::from_iter_in([1u32], &mut frame);
    let first = &mut vector[0];
    *first = 3;
}
