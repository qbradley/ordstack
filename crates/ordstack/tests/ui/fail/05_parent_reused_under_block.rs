use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let b = f.sub().alloc(5u32);
    let _c = f.alloc(6u32); // ERROR
    assert_eq!(*b, 5);
}
