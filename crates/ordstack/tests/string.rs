use ordstack::{AllocErrorKind, SString, Stack};
use std::fmt::Write;

#[test]
fn builds_and_freezes() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut s = SString::new_in(&mut f);
    s.push('h');
    s.push_str("éllo");
    write!(s, " {}", 42).unwrap();
    s.extend(['!', '?']);
    s.extend(["a", "b"]);
    assert_eq!(&*s, "héllo 42!?ab");
    assert_eq!(s.pop(), Some('b'));
    assert_eq!(format!("{s} {s:?}"), "héllo 42!?a \"héllo 42!?a\"");
    let frozen = s.freeze();
    assert_eq!(frozen.as_str(), "héllo 42!?a");
    assert_eq!(f.end_offset(), frozen.len());
    let again = SString::from_str_in("next", &mut f).freeze();
    assert_eq!((frozen.as_str(), again.as_str()), ("héllo 42!?a", "next"));
}

#[test]
fn pop_truncate_clear_respect_char_boundaries() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut s = SString::from_str_in("aé", &mut f);
    assert_eq!(s.pop(), Some('é'));
    assert_eq!(s.pop(), Some('a'));
    assert_eq!(s.pop(), None);
    s.push_str("héllo");
    s.truncate(3);
    assert_eq!(&*s, "hé");
    s.truncate(100);
    s.clear();
    assert!(s.is_empty());
}

#[test]
#[should_panic(expected = "ordstack: truncate index 2 is not a char boundary")]
fn truncate_inside_a_char_panics() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut s = SString::from_str_in("hé", &mut f);
    s.truncate(2);
}

#[test]
fn failed_truncate_leaves_string_intact() {
    let mut st = Stack::with_reserve(1 << 16);
    let mut f = st.frame();
    let mut s = SString::from_str_in("h€llo", &mut f);
    for bad in [2, 3] {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.truncate(bad)));
        assert!(r.is_err());
        assert_eq!(&*s, "h€llo");
    }
    s.truncate(4);
    assert_eq!(&*s, "h€");
}

#[test]
fn freeze_and_descend_and_freeze_into() {
    let mut a = Stack::with_reserve(1 << 16);
    let mut b = Stack::with_reserve(1 << 16);
    let (mut af, mut bf) = (a.frame(), b.frame());
    let mut s = SString::with_capacity_in(8, &mut af);
    assert_eq!(s.capacity(), 8);
    s.push_str("abc");
    {
        let (text, mut above) = s.split_above();
        text.make_ascii_uppercase();
        let mut t = SString::new_in(&mut above);
        t.push_str("scratch");
    }
    {
        let mut above = s.above();
        let _ = above.alloc(1u64);
    }
    s.push_str("def");
    s.reserve(100);
    assert_eq!(
        s.try_reserve(usize::MAX).unwrap_err().kind(),
        AllocErrorKind::CapacityOverflow
    );
    let copy = s.freeze_into(&mut bf);
    assert_eq!(copy.as_str(), "ABCdef");
    assert_eq!(bf.end_offset(), 6);
}
