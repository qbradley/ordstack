// The original program (src/orig.rs) generic over an allocator: allocator-api2's
// `Vec` and `Box`, and hashbrown's `HashMap`, all in `A`. Two binaries run it:
// `ordstack-alloc` with `A = &FrameAlloc` (one ordstack frame) and
// `alloc-control` with `A = &Global` (the system allocator), so the
// difference between them is the allocator alone. Comments marked PORT say
// what changed against the original and why.
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
use allocator_api2::alloc::Allocator;
use allocator_api2::boxed::Box;
use allocator_api2::vec::Vec;
use hashbrown::HashMap;
use std::collections::hash_map::RandomState;
use std::env;
use std::hash::{Hash, Hasher};
use std::io::Write as _;

// PORT: allocator-api2 has no `String`; a string is its UTF-8 bytes.
type Str<A> = Vec<u8, A>;

// PORT: a map key hashes as `str` does (bytes, then 0xff), not as a byte
// slice (length, then bytes), so the map does the original's hashing work.
// The map keeps std's hasher (`RandomState`).
struct Key<A: Allocator>(Str<A>);

impl<A: Allocator> PartialEq for Key<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<A: Allocator> Eq for Key<A> {}

impl<A: Allocator> Hash for Key<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0);
        state.write_u8(0xff);
    }
}

struct Node<A: Allocator + Copy> {
    name: Str<A>,
    children: Vec<Box<Node<A>, A>, A>,
    tags: HashMap<Key<A>, Vec<i32, A>, RandomState, A>,
}

impl<A: Allocator + Copy> Node<A> {
    fn new(name: Str<A>, a: A) -> Box<Node<A>, A> {
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
        // PORT: was `format!(..)`; `write!` into a byte vector with the
        // capacity `format!` starts from (0 here, as the template starts
        // with a placeholder; 10 for the key below).
        let mut name = Vec::new_in(a);
        name.extend_from_slice(&node.name);
        write!(name, "_{}_{}", depth, i).unwrap();
        let mut child = Node::new(name, a);
        let mut key = Vec::with_capacity_in(10, a);
        write!(key, "depth{}", depth).unwrap();
        // PORT: was `(1..=8).collect()`.
        let mut val = Vec::with_capacity_in(8, a);
        val.extend(1..=8);
        child.tags.insert(Key(key), val);
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

// PORT: was `main`; the binary supplies the allocator.
pub fn main_in<A: Allocator + Copy>(a: A) {
    let args: std::vec::Vec<String> = env::args().collect();
    let max_depth: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(7);

    let mut name = Vec::with_capacity_in(4, a);
    name.extend_from_slice(b"root");
    let mut root = Node::new(name, a);
    build(&mut root, 1, max_depth, a);

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // root (and its whole subtree) drops here, at scope exit -- a single
    // lexically nested teardown, the point of the exercise.
}
