use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.allocator();
    let r = &a;
    std::thread::scope(|s| drop(s.spawn(move || drop(r)))); // ERROR
}
