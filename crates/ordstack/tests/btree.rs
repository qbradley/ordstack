use ordstack::{SBPlusTree, Stack};
use std::collections::BTreeMap;

#[test]
fn split_exhaustion_preserves_all_entries_and_allows_overwrites() {
    let mut stack = Stack::with_reserve(4096);
    let capacity = stack.capacity();
    for available in (128..=capacity).step_by(32) {
        let mut frame = stack.frame();
        let _prefix = frame.alloc_slice_from_iter(core::iter::repeat_n(0u8, capacity - available));
        let mut tree = SBPlusTree::<u64, u64, 3, 3>::with_fanout_in(&mut frame);
        let mut model = BTreeMap::new();
        // Descending keys force splits on the left side as the tree grows.
        let failed_key = (0..256)
            .rev()
            .find(|&key| {
                if tree.try_insert(key, key).is_err() {
                    return true;
                }
                model.insert(key, key);
                false
            })
            .expect("the frame must run out of room");
        if model.is_empty() {
            assert!(tree.is_empty());
            continue;
        }
        assert!(!tree.is_empty());
        assert_eq!(tree.get(failed_key), None);
        assert!(tree.iter().eq(model.iter().map(|(&k, &v)| (k, v))));
        for (&key, &value) in &model {
            assert_eq!(tree.try_insert(key, value + 1).unwrap(), Some(value));
        }
        assert!(tree.iter().eq(model.iter().map(|(&k, &v)| (k, v + 1))));
    }
}

#[test]
fn balanced_fanout_can_build_a_tree_with_small_leaf_payloads() {
    const FANOUT: usize = SBPlusTree::<u8, (), 3>::balanced_branch_fanout();
    let mut stack = Stack::with_reserve(4096);
    let mut frame = stack.frame();
    let mut tree = SBPlusTree::<_, _, 3, FANOUT>::with_fanout_in(&mut frame);
    for key in 0u8..17 {
        tree.try_insert(key, ()).unwrap();
    }
    assert!(tree.iter().eq((0..17).map(|key| (key, ()))));
}

#[test]
fn ordered_updates_match_a_reference_map() {
    let mut stack = Stack::with_reserve(1 << 20);
    let mut frame = stack.frame();
    let mut tree = SBPlusTree::new_in(&mut frame);
    let mut model = BTreeMap::new();

    for round in 0..3 {
        for i in 0..257 {
            let key = i * 73 % 257;
            assert_eq!(tree.try_insert(key, i).unwrap(), model.insert(key, i));
        }

        for i in 0..257 {
            let key = i * 113 % 257;
            assert_eq!(tree.get(key), model.get(&key).copied());
            if (i + round) % 3 == 0 {
                assert_eq!(tree.try_insert(key, -i).unwrap(), model.insert(key, -i));
            } else {
                assert_eq!(tree.remove(key), model.remove(&key));
            }
            assert!(tree.iter().eq(model.iter().map(|(&k, &v)| (k, v))));
            assert_eq!(tree.len(), model.len());
        }
        for key in 0..257 {
            assert_eq!(tree.remove(key), model.remove(&key));
        }
        assert!(tree.is_empty());
        assert_eq!(tree.iter().next(), None);
    }
}

#[test]
fn failed_insert_preserves_entries_and_freed_nodes_can_be_reused() {
    let mut stack = Stack::with_reserve(4096);
    let mut frame = stack.frame();
    let mut tree = SBPlusTree::new_in(&mut frame);
    let limit = (0..4096)
        .find(|&key| tree.try_insert(key, key).is_err())
        .expect("the frame must run out of room");
    assert!(tree.iter().eq((0..limit).map(|key| (key, key))));
    for key in 0..limit {
        assert_eq!(tree.remove(key), Some(key));
    }
    for key in 0..limit {
        assert_eq!(tree.try_insert(key, key).unwrap(), None);
    }
}

#[test]
fn unequal_odd_fanouts_preserve_order_through_splits_and_merges() {
    let mut stack = Stack::with_reserve(1 << 20);
    let mut frame = stack.frame();
    let mut tree = SBPlusTree::<_, _, 5, 7>::with_fanout_in(&mut frame);

    for i in 0..127 {
        let key = i * 53 % 127;
        assert_eq!(tree.try_insert(key, -key).unwrap(), None);
    }
    assert!(tree.iter().eq((0..127).map(|key| (key, -key))));
    for i in 0..127 {
        let key = i * 73 % 127;
        assert_eq!(tree.remove(key), Some(-key));
        assert_eq!(tree.get(key), None);
        assert!(tree.iter().is_sorted_by_key(|(key, _)| key));
    }
    assert!(tree.is_empty());
}
