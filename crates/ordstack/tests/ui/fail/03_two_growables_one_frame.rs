use ordstack::{SVec, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let a: SVec<u32> = SVec::new_in(&mut f);
    let _b: SVec<u32> = SVec::new_in(&mut f); // ERROR
    a.push(1);
}
