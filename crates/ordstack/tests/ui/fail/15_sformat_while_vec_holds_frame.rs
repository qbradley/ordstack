use ordstack::{SVec, Stack, sformat};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let v: SVec<u32> = SVec::new_in(&mut f);
    let _s = sformat!(&mut f, "{}", 1); // ERROR
    v.push(1);
}
