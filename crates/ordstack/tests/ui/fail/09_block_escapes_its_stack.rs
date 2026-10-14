use ordstack::{SBox, Stack};
fn main() {
    let mut keep: Vec<SBox<u32>> = Vec::new();
    {
        let mut st = Stack::with_reserve(1 << 16);
        let b = st.frame().alloc(5u32);
        keep.push(b); // ERROR
    }
    assert!(keep.len() <= 1);
}
