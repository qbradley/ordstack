// The original program (src/orig.rs) with its containers on one ordstack
// frame through `FrameAlloc` (feature `allocator-api2`): allocator-api2's
// `Vec` and `Box`, and hashbrown's `HashMap`, all on `&FrameAlloc`. No
// `unsafe`. The tree is built exactly as in the original; comments marked
// PORT say what changed and why. Lines marked BENCH are instrumentation.
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
use allocator_api2::alloc::Allocator;
use allocator_api2::boxed::Box;
use allocator_api2::vec::Vec;
use hashbrown::HashMap;
use ordstack::Stack;
use std::collections::hash_map::RandomState;
use std::env;
use std::io::Write as _;

// PORT: allocator-api2 has no `String`; a name is its UTF-8 bytes. The
// map keeps std's hasher (`RandomState`), so only the allocator differs.
struct Node<A: Allocator + Copy> {
    name: Vec<u8, A>,
    children: Vec<Box<Node<A>, A>, A>,
    tags: HashMap<Vec<u8, A>, Vec<i32, A>, RandomState, A>,
}

impl<A: Allocator + Copy> Node<A> {
    fn new(name: Vec<u8, A>, a: A) -> Box<Node<A>, A> {
        Box::new_in(
            Node {
                name,
                children: Vec::new_in(a),
                tags: HashMap::with_hasher_in(RandomState::new(), a),
            },
            a,
        )
    }
}

// PORT: the allocator is passed down, as a `Copy` handle.
fn build<A: Allocator + Copy>(node: &mut Node<A>, depth: usize, max_depth: usize, a: A) {
    if depth > max_depth {
        return;
    }
    for i in 1..=5 {
        // PORT: was `format!(..)`; `write!` into a byte vector.
        let mut name = Vec::new_in(a);
        name.extend_from_slice(&node.name);
        write!(name, "_{}_{}", depth, i).unwrap();
        let mut child = Node::new(name, a);
        let mut key = Vec::new_in(a);
        write!(key, "depth{}", depth).unwrap();
        // PORT: was `(1..=8).collect()`.
        let mut val = Vec::new_in(a);
        val.extend(1..=8);
        child.tags.insert(key, val);
        build(&mut child, depth + 1, max_depth, a);
        node.children.push(child);
    }
}

fn count_and_sum<A: Allocator + Copy>(node: &Node<A>) -> (u64, i64) {
    let mut n: u64 = 1;
    let mut s: i64 = node.tags.values().flatten().map(|&x| x as i64).sum();
    for c in &node.children {
        let (cn, cs) = count_and_sum(c);
        n += cn;
        s += cs;
    }
    (n, s)
}

fn main() {
    let args: std::vec::Vec<String> = env::args().collect();
    let max_depth: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(7);

    // PORT: a stack, a frame and its allocator, declared before the tree.
    let mut st = Stack::new();
    let mut f = st.frame();
    let alloc = f.allocator();
    let a = &alloc;

    let mut name = Vec::new_in(a);
    name.extend_from_slice(b"root");
    let mut root = Node::new(name, a);
    build(&mut root, 1, max_depth, a);

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // BENCH{ stack statistics, read once nothing on the stack is alive.
    if env::var_os("ORDSTACK_STATS").is_some() {
        drop(root);
        eprintln!(
            "stack tree high_water={} extent={}",
            st.high_water(),
            st.extent()
        );
    }
    // BENCH}
    // root (and its whole subtree) drops here, at scope exit -- a single
    // lexically nested teardown, the point of the exercise.
}
