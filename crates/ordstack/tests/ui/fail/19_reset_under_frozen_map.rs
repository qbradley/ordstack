use ordstack::{SMap, Stack};
fn main() {
    let mut st = Stack::with_reserve(1 << 16);
    let fm = {
        let mut f = st.frame();
        let mut m: SMap<u32, u32> = SMap::new_in(&mut f);
        m.insert(1, 1);
        m.freeze()
    };
    st.reset(); // ERROR
    assert_eq!(fm.get(&1), Some(&1));
}
