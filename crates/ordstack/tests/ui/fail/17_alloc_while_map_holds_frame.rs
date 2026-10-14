use ordstack::{SMap, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut m: SMap<u32, u32> = SMap::new_in(&mut f);
    let _b = f.alloc(1u32); // ERROR
    m.insert(1, 1);
}
