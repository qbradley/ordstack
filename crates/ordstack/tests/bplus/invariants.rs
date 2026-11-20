use super::{Leaf, Node, Page, SBPlusTree};
use crate::{AllocErrorKind, Stack};

#[test]
fn balanced_fanout_uses_available_leaf_space() {
    use super::Edge;
    type Value = (usize, usize);
    const B: usize = SBPlusTree::<usize, Value>::balanced_branch_fanout();
    let leaf_bytes = size_of::<(Leaf<usize, Value, 8>, Option<usize>)>();
    assert!(size_of::<Page<Edge<usize>, B>>() <= leaf_bytes);
    assert!(size_of::<Page<Edge<usize>, { B + 1 }>>() > leaf_bytes);
}

#[test]
fn inserts_that_need_no_split_work_without_room_for_more_nodes() {
    type Tree<'v, 'f> = SBPlusTree<'v, 'f, u64, u64, 3, 3>;
    let mut stack = Stack::with_reserve(4096);
    let prefix = stack.capacity() - 4 * size_of::<Node<u64, u64, 3, 3>>();
    let mut frame = stack.frame();
    let _prefix = frame.alloc_slice_from_iter(core::iter::repeat_n(0u8, prefix));
    let mut tree = Tree::with_fanout_in(&mut frame);

    // Seven ascending keys fill the root and the rightmost leaf. The
    // leftmost leaf still has space, though the frame cannot fit a node.
    for key in (0..14).step_by(2) {
        assert_eq!(tree.try_insert(key, key).unwrap(), None);
    }
    assert_eq!(tree.try_insert(1, 1).unwrap(), None);
    assert_eq!(tree.try_insert(12, 42).unwrap(), Some(12));
    assert_eq!(
        tree.try_insert(14, 14).unwrap_err().kind(),
        AllocErrorKind::Exhausted
    );
    assert!(tree.iter().eq([
        (0, 0),
        (1, 1),
        (2, 2),
        (4, 4),
        (6, 6),
        (8, 8),
        (10, 10),
        (12, 42)
    ]));
}

#[test]
fn split_and_reuse_keep_nodes_balanced_and_separators_current() {
    type Tree<'v, 'f> = SBPlusTree<'v, 'f, u32, (), 5, 5>;

    fn audit(tree: &Tree<'_, '_>) {
        fn walk(tree: &Tree<'_, '_>, node: usize, root: bool) -> (u32, usize) {
            match &tree.nodes[node] {
                Node::Leaf(page, _) => {
                    assert!(root || page.len >= Leaf::<u32, (), 5>::MIN);
                    (page.at(page.len - 1).0, page.len)
                }
                Node::Branch(page) => {
                    assert!(
                        page.len
                            >= if root {
                                2
                            } else {
                                page.entries.len().div_ceil(2)
                            }
                    );
                    let mut count = 0;
                    for edge in page.entries[..page.len].iter().flatten() {
                        let (max, entries) = walk(tree, edge.child, false);
                        assert_eq!(edge.max, max);
                        count += entries;
                    }
                    (page.at(page.len - 1).max, count)
                }
                Node::Free(_) => panic!("tree points to a free node"),
            }
        }

        if let Some(root) = tree.root {
            assert_eq!(walk(tree, root, true).1, tree.len());
        }
        let mut cursor = tree.free;
        let mut count = 0;
        while let Some(node) = cursor {
            let Node::Free(next) = tree.nodes[node] else {
                panic!("free list contains a live node")
            };
            count += 1;
            assert!(count <= tree.nodes.len(), "free list contains a cycle");
            cursor = next;
        }
        assert_eq!(count, tree.free_count);
    }

    let mut stack = Stack::with_reserve(1 << 16);
    let mut frame = stack.frame();
    let mut tree = Tree::with_fanout_in(&mut frame);
    for key in [0, 2, 4, 6, 8, 5] {
        tree.try_insert(key, ()).unwrap();
        audit(&tree);
    }
    for key in 9..100 {
        tree.try_insert(key, ()).unwrap();
        audit(&tree);
    }
    for key in (0..100).rev() {
        tree.remove(key);
        audit(&tree);
    }
    tree.try_insert(1, ()).unwrap();
    audit(&tree);
}
