use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let b = f.alloc(1u32);
    std::thread::scope(|s| { s.spawn(|| assert_eq!(*b, 1)); }); // ERROR
    assert_eq!(*b, 1);
}
