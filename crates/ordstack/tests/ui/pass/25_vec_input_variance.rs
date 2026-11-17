#![allow(unused)]
use ordstack::{SVec, Stack};

fn accepts_short<'a>(_: &SVec<'_, '_, &'a str>, _: &'a str) {}

fn main() {
    let mut stack = Stack::with_reserve(1 << 16);
    let mut frame = stack.frame();
    let vector = SVec::from_iter_in(["long-lived"], &mut frame);
    let short = String::from("short-lived");
}
