use ordstack::{SBox, SVec, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut lower: SVec<Option<SBox<u32>>> = SVec::new_in(&mut f);
    lower.push(None);
    let (elems, mut above) = lower.split_above();
    elems[0] = Some(above.alloc(7u32)); // ERROR
}
