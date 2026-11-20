use ordstack::{AllocErrorKind, SBytePool, SFixedBytePool, Stack};

#[test]
fn freeze_keeps_handles_and_addresses_and_releases_the_byte_frame() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let handle = pool.try_alloc(16).unwrap();
    pool.get_mut(handle).unwrap()[0] = 17;
    let second = pool.try_alloc(1).unwrap();
    let address = pool.get(handle).unwrap().as_ptr();
    let mut frozen = pool.freeze();
    let other = bytes.alloc([42; 32]);
    assert_eq!(frozen.get(handle).unwrap().as_ptr(), address);
    assert_eq!(frozen.get(handle).unwrap()[0], 17);
    assert_eq!(frozen.get(second), Some(&[0][..]));
    let spare = frozen.capacity() - 17;
    assert!(spare >= 8);
    for _ in 0..spare {
        let run = frozen.try_alloc(1).unwrap();
        assert_eq!(frozen.get(run), Some(&[0][..]));
    }
    assert_eq!(
        frozen.try_alloc(1).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert!(frozen.free(handle));
    assert!(!frozen.free(handle));
    assert_eq!(*other, [42; 32]);
}

#[test]
fn fixed_pool_fits_only_after_neighboring_space_is_freed() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SFixedBytePool::with_capacity_in(8, &mut bytes, &mut runs);
    let first = pool.try_alloc(4).unwrap();
    let second = pool.try_alloc(4).unwrap();
    pool.get_mut(first).unwrap().copy_from_slice(b"data");
    assert_eq!(
        pool.try_resize(first, 8).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert_eq!(pool.get(first), Some(&b"data"[..]));
    assert!(pool.free(second));
    let grown = pool.try_resize(first, 8).unwrap().unwrap();
    assert_eq!(pool.get(grown), Some(&b"data\0\0\0\0"[..]));
}

#[test]
fn fixed_pool_can_relocate_a_tail_run_into_the_combined_space() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SFixedBytePool::with_capacity_in(16, &mut bytes, &mut runs);
    let first = pool.try_alloc(4).unwrap();
    let left = pool.try_alloc(4).unwrap();
    let tail = pool.try_alloc(4).unwrap();
    let address = pool.get(left).unwrap().as_ptr();
    pool.get_mut(first).unwrap().fill(42);
    pool.get_mut(tail).unwrap().copy_from_slice(b"tail");
    assert!(pool.free(left));
    let moved = pool.try_resize(tail, 10).unwrap().unwrap();
    assert_eq!(pool.get(moved).unwrap().as_ptr(), address);
    assert_eq!(pool.get(moved), Some(&b"tail\0\0\0\0\0\0"[..]));
    assert_eq!(pool.get(first), Some(&[42; 4][..]));
    let last = pool.try_alloc(2).unwrap();
    assert_eq!(pool.get(last).unwrap().as_ptr(), address.wrapping_add(10));
}

#[test]
fn partial_hole_claim_keeps_the_remainder_and_exact_claim_consumes_it() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SFixedBytePool::with_capacity_in(12, &mut bytes, &mut runs);
    let hole = pool.try_alloc(8).unwrap();
    let last = pool.try_alloc(4).unwrap();
    let address = pool.get(hole).unwrap().as_ptr();
    pool.get_mut(hole).unwrap().fill(42);
    assert!(pool.free(hole));

    let first = pool.try_alloc(3).unwrap();
    let second = pool.try_alloc(5).unwrap();
    assert_eq!(pool.get(first).unwrap().as_ptr(), address);
    assert_eq!(pool.get(second).unwrap().as_ptr(), address.wrapping_add(3));
    assert_eq!(pool.get(first), Some(&[0; 3][..]));
    assert_eq!(pool.get(second), Some(&[0; 5][..]));
    assert_eq!(
        pool.try_alloc(1).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert!(pool.free(last));
    assert!(pool.free(second));
    assert!(pool.free(first));
    assert!(pool.try_alloc(12).is_ok());
}

#[test]
fn exact_reuse_does_not_exhaust_metadata_in_a_dense_pool() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SFixedBytePool::with_capacity_in(2048, &mut bytes, &mut runs);
    let handles: Vec<_> = (0..2048).map(|_| pool.try_alloc(1).unwrap()).collect();

    // At most one hole exists at a time, regardless of the number of runs.
    for &run in handles.iter().step_by(2) {
        let address = pool.get(run).unwrap().as_ptr();
        assert_eq!(pool.try_free(run), Ok(true));
        let reused = pool.try_alloc(1).unwrap();
        assert_eq!(pool.get(reused).unwrap().as_ptr(), address);
    }
}
