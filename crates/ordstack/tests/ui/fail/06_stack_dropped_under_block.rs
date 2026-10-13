use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let b = st.frame().alloc(5u32);
    drop(st); // ERROR
    assert_eq!(*b, 5);
}
