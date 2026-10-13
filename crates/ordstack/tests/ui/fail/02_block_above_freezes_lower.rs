use ordstack::{SVec, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut lower: SVec<u32> = SVec::new_in(&mut f);
    let b = lower.above().alloc(7u32);
    lower.push(1); // ERROR
    assert_eq!(*b, 7);
}
