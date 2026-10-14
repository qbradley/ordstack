// Port of treebuild (src/orig.rs) onto ordstack 0.1. Adapted from the
// stackfit spike's port. No `unsafe`, no Rc, no leaking. Comments marked
// PORT say what the API forced; lines marked BENCH are instrumentation and
// are not counted as source cost.
//
// Mirrors run_corpus.sh's perl_oo workload's shape (a tree of blessed hash
// objects with a tags map per node, branching 5, depth 7 -- see the perl
// one-liner in run_corpus.sh's "3. perl (object-oriented, nested data)"
// entry) using Rust's idiomatic Box-linked ownership instead of perl's
// reference-counted hashes: every child is owned outright by its parent
// (Vec<Box<Node>>), so drop order is lexically nested and determined
// entirely by scope exit -- the motivating property from the task's
// "compiler-integrated variant" hypothesis (ownership makes drop order
// lexically nested and moves explicit).
#![forbid(unsafe_code)]
use ordstack::{Frame, SBox, SSlice, SStr, SString, SVec, Stack};
use std::env;
use std::fmt::Write as _;

// PORT: every field is a frozen block of the `tree` stack. A node cannot
// hold a growable container: a growable holds `&mut Frame`, and a node that
// kept the frame could not coexist with its siblings.
struct Node<'t> {
    name: SStr<'t>,
    children: SSlice<'t, SBox<'t, Node<'t>>>,
    // PORT: was `HashMap<String, Vec<i32>>`. ordstack has no map; a sorted
    // slice of (key, value) pairs, looked up by binary search, stands in.
    tags: SSlice<'t, (SStr<'t>, SSlice<'t, i32>)>,
}

impl<'t> Node<'t> {
    fn new(tree: &mut Frame<'t>, name: SStr<'t>) -> SBox<'t, Node<'t>> {
        tree.alloc(Node {
            name,
            children: SSlice::empty(),
            tags: SSlice::empty(),
        })
    }
}

// PORT: two stacks. `tree` receives everything that stays; `scratch` holds
// the children vector of each node that is still being built, one per
// recursion level, each above its parent's (freeze and descend).
fn build<'t>(
    node: &mut Node<'t>,
    tree: &mut Frame<'t>,
    scratch: &mut Frame<'_>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }
    // PORT: was `node.children.push(child)` on the node's own Vec. That Vec
    // grows while the subtrees of its earlier elements are alive above it,
    // which no stack allows. It now grows in place on the scratch stack and
    // is copied once into the tree when complete.
    let mut children = SVec::new_in(scratch);
    for i in 1..=5 {
        // PORT: was `format!(..)`; ordstack has no `format!` twin, so
        // `write!` into an `SString` (which implements `fmt::Write`), then
        // freeze. The name is built before the box: an argument cannot use
        // `tree` while `tree` is also being passed.
        let mut name = SString::new_in(tree);
        write!(name, "{}_{}_{}", node.name, depth, i).unwrap();
        let name = name.freeze();
        let mut child = Node::new(tree, name);
        // PORT: was `child.tags.insert(format!(..), (1..=8).collect())`.
        // Key and value must exist before the pair vector starts growing on
        // the same stack, and it is frozen before it goes in the node.
        let mut key = SString::new_in(tree);
        write!(key, "depth{}", depth).unwrap();
        let key = key.freeze();
        let val = SVec::from_iter_in(1..=8, tree).freeze();
        let mut tags = SVec::new_in(tree);
        insert_sorted(&mut tags, key, val);
        child.tags = tags.freeze();
        build(
            &mut child,
            tree,
            &mut children.above(),
            depth + 1,
            max_depth,
        );
        children.push(child);
    }
    node.children = children.freeze_into(tree);
}

// PORT: `HashMap::insert` on a sorted pair vector (replaces an equal key).
fn insert_sorted<K: Ord, V>(map: &mut SVec<'_, '_, (K, V)>, key: K, val: V) {
    match map.binary_search_by(|(k, _)| k.cmp(&key)) {
        Ok(i) => map[i].1 = val,
        Err(i) => map.insert(i, (key, val)),
    }
}

fn count_and_sum(node: &Node) -> (u64, i64) {
    let mut n: u64 = 1;
    // PORT: was `node.tags.values().flatten()`.
    let mut s: i64 = node
        .tags
        .iter()
        .flat_map(|(_, v)| v.iter())
        .map(|&x| x as i64)
        .sum();
    // PORT: was `for c in &node.children`; `&SSlice` is not `IntoIterator`.
    for c in node.children.iter() {
        let (cn, cs) = count_and_sum(c);
        n += cn;
        s += cs;
    }
    (n, s)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let max_depth: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(7);

    // PORT: two stacks and their frames, declared before anything on them.
    let mut tree_stack = Stack::new();
    let mut scratch_stack = Stack::new();
    let mut tree = tree_stack.frame();
    let mut scratch = scratch_stack.frame();

    let name = tree.alloc_str("root");
    let mut root = Node::new(&mut tree, name);
    build(&mut root, &mut tree, &mut scratch, 1, max_depth);

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // BENCH{ stack statistics. They can only be read once nothing branded by
    // the stack is alive, so the tree is dropped first (it would be dropped
    // here anyway).
    if env::var_os("ORDSTACK_STATS").is_some() {
        drop(root);
        for (name, st) in [("tree", &tree_stack), ("scratch", &scratch_stack)] {
            eprintln!(
                "stack {} high_water={} extent={}",
                name,
                st.high_water(),
                st.extent()
            );
        }
    }
    // BENCH}
    // root (and its whole subtree) drops here, at scope exit -- a single
    // lexically nested teardown, the point of the exercise.
}
