// Port of treebuild (src/orig.rs) onto bumpalo 3 (features `collections`,
// `boxed`). No `unsafe`. Comments marked PORT say what changed; lines
// marked BENCH are instrumentation and are not counted as source cost.
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
use bumpalo::boxed::Box;
use bumpalo::collections::{String, Vec};
use bumpalo::Bump;
use std::env;

// PORT: every container lives in one bump arena; `'b` is its borrow.
struct Node<'b> {
    name: String<'b>,
    children: Vec<'b, Box<'b, Node<'b>>>,
    // PORT: was `HashMap<String, Vec<i32>>`. bumpalo has no map; a sorted
    // vector of (key, value) pairs, looked up by binary search, stands in.
    tags: Vec<'b, (String<'b>, Vec<'b, i32>)>,
}

impl<'b> Node<'b> {
    fn new(b: &'b Bump, name: String<'b>) -> Box<'b, Node<'b>> {
        Box::new_in(
            Node {
                name,
                children: Vec::new_in(b),
                tags: Vec::new_in(b),
            },
            b,
        )
    }
}

fn build<'b>(b: &'b Bump, node: &mut Node<'b>, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    for i in 1..=5 {
        let mut child = Node::new(b, bumpalo::format!(in b, "{}_{}_{}", node.name, depth, i));
        insert_sorted(
            &mut child.tags,
            bumpalo::format!(in b, "depth{}", depth),
            Vec::from_iter_in(1..=8, b),
        );
        build(b, &mut child, depth + 1, max_depth);
        // Allowed by bumpalo although the child's subtree was allocated after
        // this vector: when it has to grow, it is copied to a fresh block.
        node.children.push(child);
    }
}

// PORT: `HashMap::insert` on a sorted pair vector (replaces an equal key).
fn insert_sorted<K: Ord, V>(map: &mut Vec<'_, (K, V)>, key: K, val: V) {
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
    for c in &node.children {
        let (cn, cs) = count_and_sum(c);
        n += cn;
        s += cs;
    }
    (n, s)
}

fn main() {
    let args: std::vec::Vec<std::string::String> = env::args().collect();
    let max_depth: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(7);

    let bump = Bump::new(); // PORT: the arena, declared before the tree.
    let mut root = Node::new(&bump, String::from_str_in("root", &bump));
    build(&bump, &mut root, 1, max_depth);

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // BENCH{ arena statistics (bytes held in chunks, including dead copies).
    if env::var_os("ORDSTACK_STATS").is_some() {
        eprintln!("bump allocated_bytes={}", bump.allocated_bytes());
    }
    // BENCH}
    // root (and its whole subtree) drops here, at scope exit -- a single
    // lexically nested teardown, the point of the exercise.
}
