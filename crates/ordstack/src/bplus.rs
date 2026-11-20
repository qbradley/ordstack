//! An ordered map with entries in linked leaves.
//!
//! Future work: build from sorted input by filling leaves directly, then
//! build the branches from their maximum keys.

use core::cmp::Ordering;
use core::mem::{align_of, size_of};

use crate::{AllocError, Frame, SVec};

#[repr(C)]
struct Page<T, const N: usize> {
    len: usize,
    entries: [Option<T>; N],
}

impl<T: Copy, const N: usize> Page<T, N> {
    const MIN: usize = N.div_ceil(2);

    fn new() -> Self {
        Self {
            entries: [None; N],
            len: 0,
        }
    }

    fn at(&self, i: usize) -> T {
        self.entries[i].unwrap()
    }

    fn insert(&mut self, i: usize, entry: T) {
        self.entries.copy_within(i..self.len, i + 1);
        self.entries[i] = Some(entry);
        self.len += 1;
    }

    fn remove(&mut self, i: usize) -> T {
        let entry = self.at(i);
        self.entries.copy_within(i + 1..self.len, i);
        self.len -= 1;
        self.entries[self.len] = None;
        entry
    }

    fn insert_or_split(&mut self, i: usize, entry: T) -> Option<Self> {
        if self.len < N {
            self.insert(i, entry);
            return None;
        }
        // Leave room on the side that receives the new entry.
        let cut = Self::MIN - usize::from(i < Self::MIN);
        let mut right = Self::new();
        right.len = N - cut;
        right.entries[..right.len].copy_from_slice(&self.entries[cut..]);
        self.entries[cut..].fill(None);
        self.len = cut;
        if i < Self::MIN {
            self.insert(i, entry);
        } else {
            right.insert(i - cut, entry);
        }
        Some(right)
    }
}

#[repr(C)]
struct Leaf<K, V, const N: usize> {
    len: usize,
    // Only the prefix of length `len` belongs to the tree. The rest holds
    // copies of entries so both arrays stay initialized without slot tags.
    keys: [K; N],
    values: [V; N],
}

impl<K: Copy, V: Copy, const N: usize> Leaf<K, V, N> {
    const MIN: usize = N.div_ceil(2);

    fn new((key, value): (K, V)) -> Self {
        Self {
            len: 1,
            keys: [key; N],
            values: [value; N],
        }
    }

    fn at(&self, i: usize) -> (K, V) {
        (self.keys[i], self.values[i])
    }

    fn insert(&mut self, i: usize, (key, value): (K, V)) {
        self.keys.copy_within(i..self.len, i + 1);
        self.values.copy_within(i..self.len, i + 1);
        self.keys[i] = key;
        self.values[i] = value;
        self.len += 1;
    }

    fn remove(&mut self, i: usize) -> V {
        let value = self.values[i];
        self.keys.copy_within(i + 1..self.len, i);
        self.values.copy_within(i + 1..self.len, i);
        self.len -= 1;
        value
    }

    fn insert_or_split(&mut self, i: usize, entry: (K, V)) -> Option<Self> {
        if self.len < N {
            self.insert(i, entry);
            return None;
        }
        let cut = Self::MIN - usize::from(i < Self::MIN);
        let mut right = Self::new(entry);
        right.len = N - cut;
        right.keys[..right.len].copy_from_slice(&self.keys[cut..]);
        right.values[..right.len].copy_from_slice(&self.values[cut..]);
        self.len = cut;
        if i < Self::MIN {
            self.insert(i, entry);
        } else {
            right.insert(i - cut, entry);
        }
        Some(right)
    }

    fn balance(&mut self, right: &mut Self) -> bool {
        let total = self.len + right.len;
        if total <= N {
            self.keys[self.len..total].copy_from_slice(&right.keys[..right.len]);
            self.values[self.len..total].copy_from_slice(&right.values[..right.len]);
            self.len = total;
            true
        } else {
            if self.len < Self::MIN {
                let key = right.keys[0];
                self.insert(self.len, (key, right.remove(0)));
            } else {
                let i = self.len - 1;
                let key = self.keys[i];
                right.insert(0, (key, self.remove(i)));
            }
            false
        }
    }
}

#[derive(Clone, Copy)]
struct Edge<K, A = ()> {
    max: K,
    child: usize,
    summary: A,
}

// A summary combines consecutive entries in key order.
pub trait Summary<K, V>: Copy {
    const ENABLED: bool = true;
    const EMPTY: Self;
    fn entry(key: K, value: V) -> Self;
    fn join(self, right: Self) -> Self;
}

impl<K, V> Summary<K, V> for () {
    const ENABLED: bool = false;
    const EMPTY: Self = ();
    fn entry(_: K, _: V) {}
    fn join(self, _: Self) {}
}

enum Node<K, V, const L: usize, const B: usize, A = ()> {
    Leaf(Leaf<K, V, L>, Option<usize>),
    Branch(Page<Edge<K, A>, B>),
    Free(Option<usize>),
}

impl<K: Ord + Copy, V: Copy, const L: usize, const B: usize, A: Copy> Node<K, V, L, B, A> {
    fn len(&self) -> usize {
        match self {
            Self::Leaf(page, _) => page.len,
            Self::Branch(page) => page.len,
            Self::Free(_) => unreachable!(),
        }
    }

    fn max(&self) -> K {
        match self {
            Self::Leaf(page, _) => page.keys[page.len - 1],
            Self::Branch(page) => page.at(page.len - 1).max,
            Self::Free(_) => unreachable!(),
        }
    }

    fn has_minimum_entries(&self) -> bool {
        self.len()
            >= match self {
                Self::Leaf(..) => Leaf::<K, V, L>::MIN,
                Self::Branch(_) => Page::<Edge<K, A>, B>::MIN,
                Self::Free(_) => unreachable!(),
            }
    }

    fn route(page: &Page<Edge<K, A>, B>, key: K) -> usize {
        let last = page.len - 1;
        if last == 0 || key > page.at(last - 1).max {
            return last;
        }
        page.entries[..last - 1]
            .iter()
            .position(|entry| key <= entry.unwrap().max)
            .unwrap_or(last - 1)
    }
}

/// A stack-backed ordered map for copyable keys and values.
///
/// Lookups, inserts, and removals take logarithmic time. Iteration follows
/// the leaves in key order. The tree keeps an exclusive borrow of its frame.
///
/// `L` and `B` set the maximum number of entries in a leaf and a branch,
/// respectively. Both default to eight.
pub struct SBPlusTree<'v, 'f, K, V, const L: usize = 8, const B: usize = 8, A = ()> {
    nodes: SVec<'v, 'f, Node<K, V, L, B, A>>,
    root: Option<usize>,
    free: Option<usize>,
    free_count: usize,
    len: usize,
}

impl<'v, 'f, K: Ord + Copy, V: Copy> SBPlusTree<'v, 'f, K, V> {
    /// Creates an empty tree with the default fanouts.
    pub fn new_in(frame: &'v mut Frame<'f>) -> Self {
        Self::with_fanout_in(frame)
    }
}

impl<'v, 'f, K: Ord + Copy, V: Copy, const L: usize, const B: usize, A: Summary<K, V>>
    SBPlusTree<'v, 'f, K, V, L, B, A>
{
    /// Returns the largest branch fanout that fits within the leaf payload.
    ///
    /// If the leaf payload is too small, returns the minimum fanout of three.
    /// Use this in a const argument for `B` once the key and value types are known.
    pub const fn balanced_branch_fanout() -> usize {
        let leaf = size_of::<(Leaf<K, V, L>, Option<usize>)>();
        let align = align_of::<Page<Edge<K, A>, 0>>();
        let budget = leaf / align * align;
        let edges = budget.saturating_sub(size_of::<Page<Edge<K, A>, 0>>())
            / size_of::<Option<Edge<K, A>>>();
        if edges < 3 { 3 } else { edges }
    }

    /// Creates an empty tree with the specified fanouts and no reserved nodes.
    ///
    /// # Panics
    /// If either fanout is less than three.
    pub fn with_fanout_in(frame: &'v mut Frame<'f>) -> Self {
        assert!(
            L >= 3 && B >= 3,
            "ordstack: B+ tree fanout must be at least three"
        );
        Self {
            nodes: SVec::new_in(frame),
            root: None,
            free: None,
            free_count: 0,
            len: 0,
        }
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether the tree contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn leaf_for(&self, mut node: usize, key: K) -> usize {
        while let Node::Branch(page) = &self.nodes[node] {
            node = page.at(Node::<K, V, L, B, A>::route(page, key)).child;
        }
        node
    }

    fn leaf(&self, node: usize) -> &Leaf<K, V, L> {
        match &self.nodes[node] {
            Node::Leaf(page, _) => page,
            _ => unreachable!(),
        }
    }

    /// Looks up a key.
    pub fn get(&self, key: K) -> Option<V> {
        let page = self.leaf(self.leaf_for(self.root?, key));
        for (i, stored) in page.keys[..page.len].iter().enumerate() {
            match key.cmp(stored) {
                Ordering::Less => return None,
                Ordering::Equal => return Some(page.values[i]),
                Ordering::Greater => {}
            }
        }
        None
    }

    fn add_node(&mut self, node: Node<K, V, L, B, A>) -> usize {
        if let Some(i) = self.free {
            let Node::Free(next) = self.nodes[i] else {
                unreachable!()
            };
            self.free = next;
            self.free_count -= 1;
            self.nodes[i] = node;
            i
        } else {
            let i = self.nodes.len();
            self.nodes.push(node);
            i
        }
    }

    fn recycle(&mut self, i: usize) {
        self.nodes[i] = Node::Free(self.free);
        self.free = Some(i);
        self.free_count += 1;
    }

    fn edge(&self, child: usize) -> Edge<K, A> {
        let node = &self.nodes[child];
        let summary = if A::ENABLED {
            match node {
                Node::Leaf(page, _) => (0..page.len)
                    .map(|i| {
                        let (key, value) = page.at(i);
                        A::entry(key, value)
                    })
                    .reduce(A::join)
                    .unwrap(),
                Node::Branch(page) => (0..page.len)
                    .map(|i| page.at(i).summary)
                    .reduce(A::join)
                    .unwrap(),
                Node::Free(_) => unreachable!(),
            }
        } else {
            A::EMPTY
        };
        Edge {
            max: node.max(),
            child,
            summary,
        }
    }

    /// Inserts an entry or replaces its value and returns the old value.
    ///
    /// If node allocation fails, the entries stay unchanged.
    pub fn try_insert(&mut self, key: K, value: V) -> Result<Option<V>, AllocError> {
        let Some(root) = self.root else {
            self.nodes
                .try_reserve(1usize.saturating_sub(self.free_count))?;
            let page = Leaf::new((key, value));
            self.root = Some(self.add_node(Node::Leaf(page, None)));
            self.len = 1;
            return Ok(None);
        };
        let (old, right) = self.insert_at(root, key, value, 1)?;
        if old.is_some() {
            return Ok(old);
        }
        if let Some(right) = right {
            let mut page = Page::new();
            page.insert(0, self.edge(root));
            page.insert(1, self.edge(right));
            self.root = Some(self.add_node(Node::Branch(page)));
        }
        self.len += 1;
        Ok(None)
    }

    fn insert_at(
        &mut self,
        node: usize,
        key: K,
        value: V,
        split_ancestors: usize,
    ) -> Result<(Option<V>, Option<usize>), AllocError> {
        let right = match &self.nodes[node] {
            Node::Leaf(page, next) => {
                let i = page.keys[..page.len]
                    .iter()
                    .position(|stored| key <= *stored)
                    .unwrap_or(page.len);
                let found = i < page.len && page.keys[i] == key;
                let next = *next;
                if !found && page.len == L {
                    // A split propagates only through full ancestors. Reserve
                    // all needed nodes before changing the leaf.
                    self.nodes
                        .try_reserve((split_ancestors + 1).saturating_sub(self.free_count))?;
                }
                let Node::Leaf(page, _) = &mut self.nodes[node] else {
                    unreachable!()
                };
                if found {
                    let old_value = core::mem::replace(&mut page.values[i], value);
                    return Ok((Some(old_value), None));
                }
                let Some(right) = page.insert_or_split(i, (key, value)) else {
                    return Ok((None, None));
                };
                Node::Leaf(right, next)
            }
            Node::Branch(page) => {
                let i = Node::<K, V, L, B, A>::route(page, key);
                let edge = page.at(i);
                let raises_max = key > edge.max;
                let splits = if page.len == B {
                    split_ancestors + 1
                } else {
                    0
                };
                let (old, sibling) = self.insert_at(edge.child, key, value, splits)?;
                if let Some(sibling) = sibling {
                    let left = self.edge(edge.child);
                    let right = self.edge(sibling);
                    let Node::Branch(page) = &mut self.nodes[node] else {
                        unreachable!()
                    };
                    page.entries[i] = Some(left);
                    let Some(right) = page.insert_or_split(i + 1, right) else {
                        return Ok((None, None));
                    };
                    Node::Branch(right)
                } else {
                    if A::ENABLED || raises_max {
                        let updated = if A::ENABLED {
                            self.edge(edge.child)
                        } else {
                            Edge { max: key, ..edge }
                        };
                        let Node::Branch(page) = &mut self.nodes[node] else {
                            unreachable!()
                        };
                        page.entries[i] = Some(updated);
                    }
                    return Ok((old, None));
                }
            }
            Node::Free(_) => unreachable!(),
        };
        let sibling = self.add_node(right);
        if let Node::Leaf(_, next) = &mut self.nodes[node] {
            *next = Some(sibling);
        }
        Ok((None, Some(sibling)))
    }

    /// Removes a key and returns its value.
    pub fn remove(&mut self, key: K) -> Option<V> {
        let root = self.root?;
        let value = self.remove_from(root, key)?;
        match &self.nodes[root] {
            Node::Branch(page) if page.len == 1 => {
                self.root = Some(page.at(0).child);
                self.recycle(root);
            }
            Node::Leaf(page, _) if page.len == 0 => {
                self.root = None;
                self.recycle(root);
            }
            _ => {}
        }
        self.len -= 1;
        Some(value)
    }

    fn remove_from(&mut self, node: usize, key: K) -> Option<V> {
        let (i, edge) = match &mut self.nodes[node] {
            Node::Leaf(page, _) => {
                let i = page.keys[..page.len]
                    .iter()
                    .position(|stored| key == *stored)?;
                return Some(page.remove(i));
            }
            Node::Branch(page) => {
                let i = Node::<K, V, L, B, A>::route(page, key);
                (i, page.at(i))
            }
            Node::Free(_) => unreachable!(),
        };
        let child = edge.child;
        let changes_max = key == edge.max;
        let value = self.remove_from(child, key)?;
        if self.nodes[child].has_minimum_entries() {
            if A::ENABLED || changes_max {
                let edge = self.edge(child);
                let Node::Branch(page) = &mut self.nodes[node] else {
                    unreachable!()
                };
                page.entries[i] = Some(edge);
            }
            return Some(value);
        }
        let Node::Branch(page) = &self.nodes[node] else {
            unreachable!()
        };
        let left_index = if i == 0 { 0 } else { i - 1 };
        let left = page.at(left_index).child;
        let right = page.at(left_index + 1).child;
        let merged = self.rebalance(left, right);
        let left_edge = self.edge(left);
        let right_edge = (!merged).then(|| self.edge(right));
        let Node::Branch(page) = &mut self.nodes[node] else {
            unreachable!()
        };
        page.entries[left_index] = Some(left_edge);
        if let Some(edge) = right_edge {
            page.entries[left_index + 1] = Some(edge);
        } else {
            page.remove(left_index + 1);
        }
        Some(value)
    }

    fn rebalance(&mut self, left: usize, right: usize) -> bool {
        fn balance<T: Copy, const N: usize>(left: &mut Page<T, N>, right: &mut Page<T, N>) -> bool {
            let total = left.len + right.len;
            if total <= N {
                left.entries[left.len..total].copy_from_slice(&right.entries[..right.len]);
                left.len = total;
                true
            } else {
                if left.len < Page::<T, N>::MIN {
                    left.insert(left.len, right.remove(0));
                } else {
                    right.insert(0, left.remove(left.len - 1));
                }
                false
            }
        }

        let (a, b) = if left < right {
            let (before, after) = self.nodes.split_at_mut(right);
            (&mut before[left], &mut after[0])
        } else {
            let (before, after) = self.nodes.split_at_mut(left);
            (&mut after[0], &mut before[right])
        };
        let merged = match (a, b) {
            (Node::Leaf(a, next), Node::Leaf(b, after)) => {
                let merged = a.balance(b);
                if merged {
                    *next = *after;
                }
                merged
            }
            (Node::Branch(a), Node::Branch(b)) => balance(a, b),
            _ => unreachable!(),
        };
        if merged {
            self.recycle(right);
        }
        merged
    }

    // The predicate must accept a subtree exactly when it contains a match.
    pub(crate) fn search_summaries(&self, matches: impl Fn(A) -> bool) -> Option<(K, V)> {
        let mut node = self.root?;
        loop {
            match &self.nodes[node] {
                Node::Branch(page) => {
                    let edge = (0..page.len)
                        .map(|i| page.at(i))
                        .find(|e| matches(e.summary))?;
                    node = edge.child;
                }
                Node::Leaf(page, _) => {
                    return (0..page.len)
                        .map(|i| page.at(i))
                        .find(|&(key, value)| matches(A::entry(key, value)));
                }
                Node::Free(_) => unreachable!(),
            }
        }
    }

    // Returns the entries immediately before and at or after the search key.
    pub(crate) fn around(&self, key: K) -> [Option<(K, V)>; 2] {
        let Some(mut node) = self.root else {
            return [None, None];
        };
        let mut before = None;
        while let Node::Branch(page) = &self.nodes[node] {
            let i = Node::<K, V, L, B, A>::route(page, key);
            if i > 0 {
                before = Some(page.at(i - 1).child);
            }
            node = page.at(i).child;
        }
        let Node::Leaf(page, next) = &self.nodes[node] else {
            unreachable!()
        };
        let i = page.keys[..page.len]
            .iter()
            .position(|stored| *stored >= key)
            .unwrap_or(page.len);
        let prev = if i > 0 {
            Some(page.at(i - 1))
        } else {
            before.map(|mut node| {
                while let Node::Branch(page) = &self.nodes[node] {
                    node = page.at(page.len - 1).child;
                }
                let leaf = self.leaf(node);
                leaf.at(leaf.len - 1)
            })
        };
        let next = if i < page.len {
            Some(page.at(i))
        } else {
            next.map(|node| self.leaf(node).at(0))
        };
        [prev, next]
    }

    /// Iterates over entries in ascending key order.
    pub fn iter(&self) -> impl Iterator<Item = (K, V)> + '_ {
        let mut leaf = self.root;
        while let Some(Node::Branch(page)) = leaf.map(|i| &self.nodes[i]) {
            leaf = Some(page.at(0).child);
        }
        let mut index = 0;
        core::iter::from_fn(move || {
            let Node::Leaf(page, next) = &self.nodes[leaf?] else {
                unreachable!()
            };
            let entry = page.at(index);
            index += 1;
            if index == page.len {
                index = 0;
                leaf = *next;
            }
            Some(entry)
        })
    }
}

#[cfg(test)]
#[path = "../tests/bplus/invariants.rs"]
mod tests;
