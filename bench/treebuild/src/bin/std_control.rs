// CONTROL, not a port: the original program on the system allocator with
// only the non-allocator change the ordstack and bumpalo ports made: the
// per-node `HashMap<String, Vec<i32>>` is a sorted vector of pairs. It
// separates the effect of that data-structure change from the allocator's.
// Lines marked CONTROL differ from src/orig.rs.
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
use std::env;

struct Node {
    name: String,
    children: Vec<Box<Node>>,
    tags: Vec<(String, Vec<i32>)>, // CONTROL
}

impl Node {
    fn new(name: String) -> Box<Node> {
        Box::new(Node {
            name,
            children: Vec::new(),
            tags: Vec::new(), // CONTROL
        })
    }
}

// CONTROL: `HashMap::insert` on a sorted pair vector, as in the ports.
fn insert_sorted<K: Ord, V>(map: &mut Vec<(K, V)>, key: K, val: V) {
    match map.binary_search_by(|(k, _)| k.cmp(&key)) {
        Ok(i) => map[i].1 = val,
        Err(i) => map.insert(i, (key, val)),
    }
}

fn build(node: &mut Node, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    for i in 1..=5 {
        let mut child = Node::new(format!("{}_{}_{}", node.name, depth, i));
        insert_sorted(
            &mut child.tags,
            format!("depth{}", depth),
            (1..=8).collect(),
        ); // CONTROL
        build(&mut child, depth + 1, max_depth);
        node.children.push(child);
    }
}

fn count_and_sum(node: &Node) -> (u64, i64) {
    let mut n: u64 = 1;
    let mut s: i64 = node
        .tags
        .iter()
        .flat_map(|(_, v)| v)
        .map(|&x| x as i64)
        .sum(); // CONTROL
    for c in &node.children {
        let (cn, cs) = count_and_sum(c);
        n += cn;
        s += cs;
    }
    (n, s)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let max_depth: usize = args.get(1).map(|s| s.parse().unwrap()).unwrap_or(7);

    let mut root = Node::new("root".to_string());
    build(&mut root, 1, max_depth);

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // root (and its whole subtree) drops here, at scope exit -- a single
    // lexically nested teardown, the point of the exercise.
}
