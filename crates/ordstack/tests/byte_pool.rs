use ordstack::{AllocErrorKind, SBytePool, Stack};

#[test]
fn first_fit_skips_small_gaps_and_tracks_freed_and_resized_runs() {
    let mut data = Stack::with_reserve(1 << 16);
    let mut metadata = Stack::with_reserve(1 << 16);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let handles: Vec<_> = (0..160).map(|_| pool.try_alloc(8).unwrap()).collect();
    let address =
        |pool: &SBytePool<'_, '_, '_, '_>, index: usize| pool.get(handles[index]).unwrap().as_ptr();
    let early = address(&pool, 38);
    let late = address(&pool, 120);
    let smaller = address(&pool, 0);

    for i in (0..160)
        .filter(|&i| i % 4 == 0 && i != 40)
        .chain([38, 39, 121])
    {
        assert!(pool.free(handles[i]));
    }
    let a = pool.try_alloc(16).unwrap();
    let b = pool.try_alloc(16).unwrap();
    assert_eq!(pool.get(a).unwrap().as_ptr(), early);
    assert_eq!(pool.get(b).unwrap().as_ptr(), late);

    let shrunk = pool.try_resize(handles[37], 4).unwrap().unwrap();
    let reopened = pool.get(shrunk).unwrap().as_ptr().wrapping_add(4);
    assert_eq!(pool.get(shrunk).unwrap().len(), 4);
    assert!(pool.free(a));
    let c = pool.try_alloc(20).unwrap();
    assert_eq!(pool.get(c).unwrap().as_ptr(), reopened);
    let d = pool.try_alloc(8).unwrap();
    assert_eq!(pool.get(d).unwrap().as_ptr(), smaller);
}

#[test]
fn growable_pool_reuses_adjacent_holes_without_tracking_generations() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let a = pool.try_alloc(4).unwrap();
    let b = pool.try_alloc(4).unwrap();
    let c = pool.try_alloc(4).unwrap();
    pool.get_mut(a).unwrap().fill(42);
    pool.get_mut(c).unwrap().fill(17);
    let first = pool.get(a).unwrap().as_ptr();
    assert!(pool.free(a));
    assert!(!pool.free(a));
    assert!(pool.free(b));
    let reused = pool.try_alloc(8).unwrap();
    assert_eq!(pool.get(reused).unwrap().as_ptr(), first);
    assert_eq!(pool.get(reused), Some(&[0; 8][..]));
    assert_eq!(pool.get(c), Some(&[17; 4][..]));
    assert_eq!(pool.get(a), Some(&[0; 4][..]));
}

#[test]
fn resize_can_move_into_an_overlapping_hole() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let hole = pool.try_alloc(2).unwrap();
    let run = pool.try_alloc(4).unwrap();
    let neighbor = pool.try_alloc(2).unwrap();
    let start = pool.get(hole).unwrap().as_ptr();
    pool.get_mut(run).unwrap().copy_from_slice(b"abcd");
    pool.get_mut(neighbor).unwrap().fill(42);
    assert!(pool.free(hole));
    let moved = pool.try_resize(run, 6).unwrap().unwrap();
    assert_eq!(pool.get(moved).unwrap().as_ptr(), start);
    assert_eq!(pool.get(moved), Some(&b"abcd\0\0"[..]));
    assert_eq!(pool.get(neighbor), Some(&[42, 42][..]));
    let shrunk = pool.try_resize(moved, 3).unwrap().unwrap();
    assert_eq!(pool.get(shrunk), Some(&b"abc"[..]));
    let grown = pool.try_resize(shrunk, 5).unwrap().unwrap();
    assert_eq!(pool.get(grown), Some(&b"abc\0\0"[..]));
    assert_eq!(pool.get(grown).unwrap().as_ptr(), start);
}

#[test]
fn failure_on_either_frame_preserves_live_runs() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(0);
    let data_limit = data.capacity();
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let first = pool.try_alloc(2).unwrap();
    let tail = pool.try_alloc(1).unwrap();
    pool.get_mut(first).unwrap().fill(9);
    assert_eq!(
        pool.try_free(first).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert_eq!(
        pool.try_resize(first, 1).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert_eq!(
        pool.try_resize(first, 3).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert_eq!(
        pool.try_resize(first, data_limit + 1).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert_eq!(pool.get(first), Some(&[9, 9][..]));
    assert!(pool.free(tail));
    assert!(pool.free(first));
    let reused = pool.try_alloc(2).unwrap();
    assert_eq!(pool.get(reused), Some(&[0, 0][..]));
}

#[test]
fn tail_resize_relocates_when_the_byte_frame_cannot_grow() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let limit = data.capacity();
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let first = pool.try_alloc(8).unwrap();
    let left = pool.try_alloc(8).unwrap();
    let tail = pool.try_alloc(limit - 16).unwrap();
    let destination = pool.get(left).unwrap().as_ptr();
    pool.get_mut(first).unwrap().fill(42);
    pool.get_mut(tail).unwrap().fill(17);
    assert!(pool.free(left));

    let grown = pool.try_resize(tail, limit - 15).unwrap().unwrap();
    let grown = pool.get(grown).unwrap();
    assert_eq!(grown.as_ptr(), destination);
    let (prefix, added) = grown.split_at(limit - 16);
    assert!(prefix.iter().all(|&byte| byte == 17));
    assert_eq!(added, [0]);
    assert_eq!(pool.get(first), Some(&[42; 8][..]));
}

#[test]
fn tail_resize_preserves_capacity_overflow() {
    let mut data = Stack::with_reserve(4096);
    let mut metadata = Stack::with_reserve(4096);
    let (mut bytes, mut runs) = (data.frame(), metadata.frame());
    let _prefix = bytes.alloc([0u8; 8]);
    let mut pool = SBytePool::new_in(&mut bytes, &mut runs);
    let left = pool.try_alloc(8).unwrap();
    let tail = pool.try_alloc(4).unwrap();
    pool.get_mut(tail).unwrap().copy_from_slice(b"tail");
    assert!(pool.free(left));

    assert_eq!(
        pool.try_resize(tail, usize::MAX - 8).unwrap_err().kind(),
        AllocErrorKind::CapacityOverflow,
    );
    assert_eq!(pool.get(tail), Some(&b"tail"[..]));
}
