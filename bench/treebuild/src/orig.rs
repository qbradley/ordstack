// Mirrors run_corpus.sh's perl_oo workload's shape (a tree of blessed hash
// objects with a tags map per node, branching 5, depth 7 -- see the perl
// one-liner in run_corpus.sh's "3. perl (object-oriented, nested data)"
// entry) using Rust's idiomatic Box-linked ownership instead of perl's
// reference-counted hashes: every child is owned outright by its parent
// (Vec<Box<Node>>), so drop order is lexically nested and determined
// entirely by scope exit -- the motivating property from the task's
// "compiler-integrated variant" hypothesis (ownership makes drop order
// lexically nested and moves explicit).
use std::collections::HashMap;
use std::env;

struct Node {
    name: String,
    children: Vec<Box<Node>>,
    tags: HashMap<String, Vec<i32>>,
}

impl Node {
    fn new(name: String) -> Box<Node> {
        Box::new(Node {
            name,
            children: Vec::new(),
            tags: HashMap::new(),
        })
    }
}

fn build(node: &mut Node, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    for i in 1..=5 {
        let mut child = Node::new(format!("{}_{}_{}", node.name, depth, i));
        child
            .tags
            .insert(format!("depth{}", depth), (1..=8).collect());
        build(&mut child, depth + 1, max_depth);
        node.children.push(child);
    }
}

fn count_and_sum(node: &Node) -> (u64, i64) {
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
    let args: Vec<String> = env::args().collect();
    let max_depth: usize = args
        .get(1)
        .map(|s| s.parse().unwrap())
        .unwrap_or(7);

    let mut root = Node::new("root".to_string());
    build(&mut root, 1, max_depth);

    let (n, s) = count_and_sum(&root);
    println!("{} {}", n, s);
    // root (and its whole subtree) drops here, at scope exit -- a single
    // lexically nested teardown, the point of the exercise.
}
