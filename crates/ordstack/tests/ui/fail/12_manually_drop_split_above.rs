use ordstack::{SBox, SVec, Stack};
use std::mem::ManuallyDrop;
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut lower = ManuallyDrop::new(SVec::<Option<SBox<u32>>>::new_in(&mut f));
    lower.push(None);
    let (elems, mut above) = lower.split_above();
    elems[0] = Some(above.alloc(7u32));
    let _clobber = f.alloc([0u8; 64]); // ERROR
    assert_eq!(**elems[0].as_ref().unwrap(), 7);
}
