use ordstack::{AllocErrorKind, Stack};

#[test]
fn new_stack_is_empty() {
    let st = Stack::with_reserve(1 << 16);
    assert!(st.capacity() >= 1 << 16);
    assert_eq!(st.extent(), 0);
    assert_eq!(st.high_water(), 0);
    assert!(format!("{st:?}").contains("capacity"));
}

#[test]
fn default_reserve_maps() {
    if cfg!(miri) {
        return; // Miri backs the region with a real heap block.
    }
    let st = Stack::new();
    assert_eq!(st.capacity(), ordstack::DEFAULT_RESERVE);
    let _ = Stack::default();
}

#[test]
fn try_with_reserve_reports_failure() {
    let e = Stack::try_with_reserve(usize::MAX).unwrap_err();
    assert_eq!(e.kind(), AllocErrorKind::CapacityOverflow);
    assert_eq!(e.to_string(), "capacity overflow");
}

#[test]
#[should_panic(expected = "ordstack: capacity overflow")]
fn with_reserve_panics_on_failure() {
    let _ = Stack::with_reserve(usize::MAX);
}

#[test]
fn reset_and_trim_on_an_empty_stack() {
    let mut st = Stack::with_reserve(1 << 16);
    st.reset();
    st.trim();
    let mut z = Stack::with_reserve(0);
    assert_eq!(z.capacity(), 0);
    z.trim();
}
