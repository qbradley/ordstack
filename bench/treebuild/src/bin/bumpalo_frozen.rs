// Port of treebuild (src/orig.rs) onto bumpalo 3 with the same node layout
// as the ordstack port: every field is a frozen 16-byte handle (`&str`,
// `&[T]`) instead of a growable 32-byte container. It is the like-for-like
// comparison for the ordstack port; `bumpalo.rs` keeps the growable
// containers a direct port would use. No `unsafe`. Comments marked PORT say
// what changed; lines marked BENCH are instrumentation and are not counted
// as source cost.
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
use bumpalo::collections::Vec;
use bumpalo::Bump;
use std::env;

// PORT: every field is frozen into the arena, as in the ordstack port.
// Nothing in a node has a destructor, so the arena dropping without running
// any is what the original's drop would have done to the memory.
struct Node<'b> {
    name: &'b str,
    children: &'b [&'b Node<'b>],
    // PORT: was `HashMap<String, Vec<i32>>`. bumpalo has no map; a sorted
    // slice of (key, value) pairs, looked up by binary search, stands in.
    tags: &'b [(&'b str, &'b [i32])],
}

// PORT: returns the frozen children of a node named `parent`. The original
// pushed each child on the parent's own Vec; here the five children are
// collected on the call stack (the branching factor is fixed at 5) and
// copied into the arena once, as the ordstack port copies its scratch
// vector into the tree once.
fn build<'b>(b: &'b Bump, parent: &str, depth: usize, max_depth: usize) -> &'b [&'b Node<'b>] {
    if depth > max_depth {
        return &[];
    }
    let children: [&'b Node<'b>; 5] = std::array::from_fn(|i| {
        // PORT: `format!` then frozen with `into_bump_str` (its spare
        // capacity stays in the arena; bumpalo cannot give it back).
        let name = bumpalo::format!(in b, "{}_{}_{}", parent, depth, i + 1).into_bump_str();
        let child = b.alloc(Node {
            name,
            children: &[],
            tags: &[],
        });
        // PORT: was `child.tags.insert(format!(..), (1..=8).collect())`.
        let key = bumpalo::format!(in b, "depth{}", depth).into_bump_str();
        let val: &'b [i32] = Vec::from_iter_in(1..=8, b).into_bump_slice();
        let mut tags = Vec::new_in(b);
        insert_sorted(&mut tags, key, val);
        child.tags = tags.into_bump_slice();
        child.children = build(b, child.name, depth + 1, max_depth);
        &*child
    });
    b.alloc_slice_copy(&children)
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
    for c in node.children {
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
    let root = Node {
        name: bump.alloc_str("root"),
        children: build(&bump, "root", 1, max_depth),
        tags: &[],
    };

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // BENCH{ arena statistics (bytes held in chunks).
    if env::var_os("ORDSTACK_STATS").is_some() {
        eprintln!("bump allocated_bytes={}", bump.allocated_bytes());
    }
    // BENCH}
    // root (and its whole subtree) goes away here, at scope exit, with the
    // arena.
}
