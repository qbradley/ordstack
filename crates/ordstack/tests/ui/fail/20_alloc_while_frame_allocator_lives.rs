use allocator_api2::vec::Vec;
use ordstack::Stack;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a = f.allocator();
    let mut v: Vec<u32, _> = Vec::new_in(&a);
    let _b = f.alloc(1u32); // ERROR
    v.push(1);
}
