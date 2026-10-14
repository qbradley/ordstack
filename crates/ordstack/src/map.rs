//! [`SMap`] and [`SFrozenMap`], insertion-ordered hash maps, with their
//! entry and iterator types.
//!
//! One block holds the entries, in insertion order, followed by an index
//! table of `u32` slots (0 is empty, otherwise the entry's index plus one),
//! probed linearly and at most 7/8 full. Growth extends the block in place:
//! the entries stay where they are and the index is rebuilt at its new
//! offset from the stored hashes, so growing runs no user code.

use core::borrow::Borrow;
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;
use core::mem::{align_of, size_of};
use core::ptr::NonNull;
use std::hash::RandomState;

use crate::debug;
use crate::error::{AllocError, AllocErrorKind, fail};
use crate::frame::Frame;
use crate::slice::drop_back_to_front;

const OVERFLOW: AllocError = AllocError::new(AllocErrorKind::CapacityOverflow);

/// The most entries a map can hold: a slot stores an entry index plus one
/// in a `u32`.
const MAX_LEN: usize = u32::MAX as usize - 1;

/// One entry. Never zero-size, and aligned to at least 4 (it holds a
/// `u64`), so the index that follows the entries is aligned.
struct Bucket<K, V> {
    hash: u64,
    key: K,
    value: V,
}

/// The entries an index of `slots` slots may hold: 7/8 of it, so at least
/// one slot is always empty and every probe ends.
fn usable(slots: usize) -> usize {
    slots - slots / 8
}

/// The smallest index for `n` entries: 0 for none, otherwise a power of two
/// of at least 8.
fn slots_for(n: usize) -> Result<usize, AllocError> {
    if n == 0 {
        return Ok(0);
    }
    if n > MAX_LEN {
        return Err(OVERFLOW);
    }
    let mut s: usize = 8;
    while usable(s) < n {
        s = s.checked_mul(2).ok_or(OVERFLOW)?;
    }
    Ok(s)
}

/// Bytes of a block with an index of `slots` slots: `usable(slots)` entries
/// followed by the index.
fn block_bytes<K, V>(slots: usize) -> Result<usize, AllocError> {
    usable(slots)
        .checked_mul(size_of::<Bucket<K, V>>())
        .and_then(|e| e.checked_add(slots.checked_mul(4)?))
        .ok_or(OVERFLOW)
}

#[cold]
fn corrupt() -> ! {
    panic!("ordstack: internal error: map index is inconsistent")
}

/// Probes `index` (non-empty, power-of-two length) from the home slot of
/// `hash`. Returns the slot and entry of the first entry for which
/// `is_match` holds, or the first empty slot.
fn probe(
    index: &[u32],
    hash: u64,
    mut is_match: impl FnMut(usize) -> bool,
) -> Result<(usize, usize), usize> {
    let mask = index.len() - 1;
    let mut s = hash as usize & mask;
    for _ in 0..index.len() {
        match index[s] {
            0 => return Err(s),
            v => {
                let e = v as usize - 1;
                if is_match(e) {
                    return Ok((s, e));
                }
            }
        }
        s = (s + 1) & mask;
    }
    corrupt()
}

/// The slot that refers to `entry`, whose stored hash is `hash`.
fn slot_of(index: &[u32], hash: u64, entry: usize) -> usize {
    match probe(index, hash, |e| e == entry) {
        Ok((s, _)) => s,
        Err(_) => corrupt(),
    }
}

/// Records `entry` with `hash` in the first empty slot of its probe run.
fn insert_slot(index: &mut [u32], hash: u64, entry: usize) {
    match probe(index, hash, |_| false) {
        Err(s) => index[s] = entry as u32 + 1,
        Ok(_) => corrupt(),
    }
}

/// Empties slot `hole` and moves later slots of the same probe runs back
/// (backward-shift deletion), so no run is broken by the new empty slot.
fn remove_slot(index: &mut [u32], mut hole: usize, hash_of: impl Fn(usize) -> u64) {
    let mask = index.len() - 1;
    let mut s = (hole + 1) & mask;
    loop {
        let v = index[s];
        if v == 0 {
            break;
        }
        let home = hash_of(v as usize - 1) as usize & mask;
        // The entry may move back to `hole` unless its home lies cyclically
        // in `(hole, s]`.
        if s.wrapping_sub(home) & mask >= s.wrapping_sub(hole) & mask {
            index[hole] = v;
            hole = s;
        }
        s = (s + 1) & mask;
    }
    index[hole] = 0;
}

/// Fills an all-zero `index` with the entries whose hashes are `hashes`.
fn fill_index(index: &mut [u32], hashes: impl Iterator<Item = u64>) {
    for (e, h) in hashes.enumerate() {
        insert_slot(index, h, e);
    }
}

/// The entry for `q`, if any.
fn lookup<K, V, Q>(buckets: &[Bucket<K, V>], index: &[u32], hash: u64, q: &Q) -> Option<usize>
where
    K: Borrow<Q>,
    Q: ?Sized + Eq,
{
    if buckets.is_empty() {
        return None;
    }
    probe(index, hash, |e| {
        buckets[e].hash == hash && buckets[e].key.borrow() == q
    })
    .ok()
    .map(|(_, e)| e)
}

/// A growable, insertion-ordered hash map on a stack.
///
/// Like [`SVec`](crate::SVec) it holds its frame exclusively, so its block is
/// the top block of the stack and grows in place; [`above`](Self::above)
/// gives a frame above it. Iteration follows insertion order, so output
/// never depends on the hasher. Entries are dropped back to front.
///
/// # Examples
/// ```
/// use ordstack::{SMap, Stack};
///
/// let mut st = Stack::with_reserve(1 << 16);
/// let mut f = st.frame();
/// let mut m: SMap<&str, u32> = SMap::new_in(&mut f);
/// for w in "a b a c b a".split(' ') {
///     *m.entry(w).or_insert(0) += 1;
/// }
/// assert_eq!(m.get("a"), Some(&3));
/// assert_eq!(m.keys().copied().collect::<Vec<_>>(), ["a", "b", "c"]);
/// ```
pub struct SMap<'v, 'f, K, V, S = RandomState> {
    frame: &'v mut Frame<'f>,
    /// Start of the block; dangling while nothing is allocated.
    ptr: NonNull<Bucket<K, V>>,
    len: usize,
    /// Index slots, after `usable(slots)` entries; 0 while nothing is
    /// allocated.
    slots: usize,
    hasher: S,
    _owns: PhantomData<Bucket<K, V>>,
}

impl<'v, 'f, K, V> SMap<'v, 'f, K, V, RandomState> {
    /// An empty map on top of `frame`, with a fresh [`RandomState`].
    /// Allocates nothing yet.
    pub fn new_in(frame: &'v mut Frame<'f>) -> Self {
        SMap::with_hasher_in(RandomState::new(), frame)
    }

    /// An empty map with room for at least `n` entries.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or an entry needs
    /// alignment above 4096.
    pub fn with_capacity_in(n: usize, frame: &'v mut Frame<'f>) -> Self {
        SMap::with_capacity_and_hasher_in(n, RandomState::new(), frame)
    }
}

impl<'v, 'f, K, V, S> SMap<'v, 'f, K, V, S> {
    /// An empty map on top of `frame` that hashes with `hasher`. Allocates
    /// nothing yet.
    pub fn with_hasher_in(hasher: S, frame: &'v mut Frame<'f>) -> Self {
        SMap {
            frame,
            ptr: NonNull::dangling(),
            len: 0,
            slots: 0,
            hasher,
            _owns: PhantomData,
        }
    }

    /// An empty map with room for at least `n` entries that hashes with
    /// `hasher`.
    ///
    /// # Panics
    /// As [`with_capacity_in`](SMap::with_capacity_in).
    pub fn with_capacity_and_hasher_in(n: usize, hasher: S, frame: &'v mut Frame<'f>) -> Self {
        let mut m = SMap::with_hasher_in(hasher, frame);
        m.reserve(n);
        m
    }

    fn allocated(&self) -> bool {
        self.slots > 0
    }

    /// Bytes of the block. Only meaningful when allocated, and then it
    /// cannot overflow: the block exists.
    fn bytes(&self) -> usize {
        usable(self.slots) * size_of::<Bucket<K, V>>() + self.slots * 4
    }

    /// Offset of the end of this map: its block's end, or the frame's end
    /// while nothing is allocated.
    fn end_off(&self) -> usize {
        if self.allocated() {
            self.frame.st.offset_of(self.ptr.cast()) + self.bytes()
        } else {
            self.frame.end
        }
    }

    fn check(&self) {
        debug::check_live(
            self.ptr.as_ptr().cast(),
            if self.allocated() { self.bytes() } else { 0 },
        );
    }

    /// The entries and the index.
    fn parts(&self) -> (&[Bucket<K, V>], &[u32]) {
        self.check();
        // SAFETY: M: `len` initialised entries at the start of the block and
        // `slots` initialised slots after `usable(slots)` entries, disjoint
        // from them. With nothing allocated both lengths are 0 and the
        // pointers are dangling and aligned.
        unsafe {
            let p = self.ptr.as_ptr();
            (
                core::slice::from_raw_parts(p, self.len),
                core::slice::from_raw_parts(p.add(usable(self.slots)).cast(), self.slots),
            )
        }
    }

    /// The entries and the index, mutably.
    fn parts_mut(&mut self) -> (&mut [Bucket<K, V>], &mut [u32]) {
        self.check();
        // SAFETY: as in `parts`; the two ranges are disjoint and `&mut self`
        // is unique.
        unsafe {
            let p = self.ptr.as_ptr();
            (
                core::slice::from_raw_parts_mut(p, self.len),
                core::slice::from_raw_parts_mut(p.add(usable(self.slots)).cast(), self.slots),
            )
        }
    }

    /// Sets the index size to `new_slots > self.slots`. Never moves the
    /// entries; rebuilds the index from the stored hashes. Runs no user
    /// code. On failure the map is unchanged.
    fn try_grow_to(&mut self, new_slots: usize) -> Result<(), AllocError> {
        debug_assert!(new_slots > self.slots);
        let st = self.frame.st;
        let bytes = block_bytes::<K, V>(new_slots)?;
        let old_end = self.end_off();
        // We hold `&mut self`, so by invariant I nothing above our end is
        // reachable; after this reset our block is the top block.
        st.reset_to(old_end);
        if self.allocated() {
            let start = st.offset_of(self.ptr.cast());
            let new_end = start.checked_add(bytes).ok_or(OVERFLOW)?;
            st.try_extend(old_end, new_end)?;
        } else {
            let start = st.try_bump(bytes, align_of::<Bucket<K, V>>())?;
            self.ptr = st.ptr_at(start).cast();
        }
        // SAFETY: the block now has `bytes` bytes: `usable(new_slots)`
        // entries, then room for `new_slots` slots, which hold no entry
        // because `usable(new_slots) > usable(self.slots) >= len`.
        unsafe {
            let index = self.ptr.as_ptr().add(usable(new_slots)).cast::<u32>();
            index.write_bytes(0, new_slots);
        }
        self.slots = new_slots;
        let (b, index) = self.parts_mut();
        fill_index(index, b.iter().map(|b| b.hash));
        Ok(())
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The number of entries the map can hold without growing.
    pub fn capacity(&self) -> usize {
        usable(self.slots)
    }

    /// The map's hasher.
    pub fn hasher(&self) -> &S {
        &self.hasher
    }

    /// Makes room for at least `additional` more entries.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or an entry needs
    /// alignment above 4096.
    pub fn reserve(&mut self, additional: usize) {
        self.try_reserve(additional).unwrap_or_else(|e| fail(e));
    }

    /// Makes room for at least `additional` more entries. On failure the
    /// map is unchanged (the stack may still have reclaimed unreachable space
    /// above it). Runs no user code.
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        let needed = self.len.checked_add(additional).ok_or(OVERFLOW)?;
        if needed <= self.capacity() {
            return Ok(());
        }
        self.try_grow_to(slots_for(needed)?)
    }

    /// Appends an entry whose key is not in the map and returns its index.
    /// Runs no user code beyond dropping `key` and `value` if it panics.
    fn push(&mut self, hash: u64, key: K, value: V) -> usize {
        self.reserve(1);
        let len = self.len;
        self.check();
        // SAFETY: `len < capacity()` after `reserve`: slot `len` is inside the
        // block and holds no entry.
        unsafe {
            self.ptr
                .as_ptr()
                .add(len)
                .write(Bucket { hash, key, value })
        };
        self.len = len + 1;
        let (_, index) = self.parts_mut();
        insert_slot(index, hash, len);
        len
    }

    /// Removes the last entry, which no slot refers to any more.
    fn pop_bucket(&mut self) -> Bucket<K, V> {
        self.len -= 1;
        // SAFETY: entry `len` was initialised and is now outside `[0, len)`;
        // the caller removed the only slot that referred to it.
        unsafe { self.ptr.as_ptr().add(self.len).read() }
    }

    /// The entry at `index` in insertion order.
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.parts().0.get(index).map(|b| (&b.key, &b.value))
    }

    /// The entry at `index`, with its value mutable.
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&K, &mut V)> {
        self.parts_mut()
            .0
            .get_mut(index)
            .map(|b| (&b.key, &mut b.value))
    }

    /// Removes the entry at `index` and moves the last entry into its place.
    /// O(1). Runs no user code.
    pub fn swap_remove_index(&mut self, index: usize) -> Option<(K, V)> {
        if index >= self.len {
            return None;
        }
        let last = self.len - 1;
        let (b, ix) = self.parts_mut();
        let s = slot_of(ix, b[index].hash, index);
        remove_slot(ix, s, |e| b[e].hash);
        if index != last {
            let s = slot_of(ix, b[last].hash, last);
            ix[s] = index as u32 + 1;
            b.swap(index, last);
        }
        let b = self.pop_bucket();
        Some((b.key, b.value))
    }

    /// Removes the entry at `index` and shifts the later entries down,
    /// keeping their order. O(n). Runs no user code.
    pub fn shift_remove_index(&mut self, index: usize) -> Option<(K, V)> {
        if index >= self.len {
            return None;
        }
        let (b, ix) = self.parts_mut();
        let s = slot_of(ix, b[index].hash, index);
        remove_slot(ix, s, |e| b[e].hash);
        for v in ix.iter_mut() {
            if *v as usize > index + 1 {
                *v -= 1;
            }
        }
        b[index..].rotate_left(1);
        let b = self.pop_bucket();
        Some((b.key, b.value))
    }

    /// Iterates the entries in insertion order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.parts().0.iter(),
        }
    }

    /// Iterates the entries in insertion order, with mutable values.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: self.parts_mut().0.iter_mut(),
        }
    }

    /// Iterates the keys in insertion order.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.iter() }
    }

    /// Iterates the values in insertion order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.iter() }
    }

    /// Iterates the values in insertion order, mutably.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            inner: self.iter_mut(),
        }
    }

    /// Removes every entry, dropping them back to front. Keeps the
    /// capacity. If a destructor panics the remaining entries are still
    /// dropped and the map is left empty.
    pub fn clear(&mut self) {
        let len = core::mem::replace(&mut self.len, 0);
        let (_, index) = self.parts_mut();
        index.fill(0);
        // SAFETY: we own `len` initialised entries; `self.len` is now 0 and
        // the index is empty, so nothing refers to them.
        unsafe { drop_back_to_front(self.ptr.as_ptr(), len) };
    }

    /// Freeze and descend: a frame above this map. The map cannot be used
    /// until the frame and everything allocated through it are gone. The new
    /// frame starts at the end of the map's block, so the map keeps its spare
    /// room.
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SMap, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut m: SMap<u32, u32> = SMap::new_in(&mut f);
    /// m.insert(1, 10);
    /// {
    ///     let mut up = m.above();
    ///     let b = up.alloc(2u32);
    ///     assert_eq!(*b, 2);
    /// }
    /// m.insert(2, 20);
    /// assert_eq!(m.len(), 2);
    /// ```
    pub fn above(&mut self) -> Frame<'_> {
        let end = self.end_off();
        self.frame.st.reset_to(end);
        Frame {
            st: self.frame.st,
            end,
        }
    }
}

impl<'v, 'f, K: Hash + Eq, V, S: BuildHasher> SMap<'v, 'f, K, V, S> {
    /// Inserts `value` under `key`. If the key was present, its value is
    /// replaced and returned, and the entry keeps its position (the old key
    /// is kept, `key` is dropped).
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or an entry needs
    /// alignment above 4096. If `Hash` or `Eq` panics, the map is unchanged.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let hash = self.hasher.hash_one(&key);
        let (b, index) = self.parts();
        match lookup(b, index, hash, &key) {
            Some(i) => Some(core::mem::replace(&mut self.parts_mut().0[i].value, value)),
            None => {
                self.push(hash, key, value);
                None
            }
        }
    }

    /// The position of `key` in insertion order.
    pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        if self.len == 0 {
            return None;
        }
        let hash = self.hasher.hash_one(key);
        let (b, index) = self.parts();
        lookup(b, index, hash, key)
    }

    /// The value under `key`.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let i = self.get_index_of(key)?;
        Some(&self.parts().0[i].value)
    }

    /// The value under `key`, mutably.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let i = self.get_index_of(key)?;
        Some(&mut self.parts_mut().0[i].value)
    }

    /// Whether `key` is in the map.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        self.get_index_of(key).is_some()
    }

    /// Removes `key` and returns its value; the last entry takes its place.
    /// O(1).
    pub fn swap_remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let i = self.get_index_of(key)?;
        self.swap_remove_index(i).map(|(_, v)| v)
    }

    /// Removes `key` and returns its value, keeping the order of the other
    /// entries. O(n).
    pub fn shift_remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Hash + Eq,
    {
        let i = self.get_index_of(key)?;
        self.shift_remove_index(i).map(|(_, v)| v)
    }
}

impl<K, V, S> Drop for SMap<'_, '_, K, V, S> {
    fn drop(&mut self) {
        self.check();
        let len = core::mem::replace(&mut self.len, 0);
        // SAFETY: we own `len` initialised entries and `self.len` is now 0.
        // The block itself is reclaimed when the frame is next used.
        unsafe { drop_back_to_front(self.ptr.as_ptr(), len) };
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for SMap<'_, '_, K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        // A hint that does not fit is not an error; `insert` decides.
        let _ = self.try_reserve(iter.size_hint().0);
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for SMap<'_, '_, K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<'a, K, V, S> IntoIterator for &'a SMap<'_, '_, K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Iter<'a, K, V> {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut SMap<'_, '_, K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> IterMut<'a, K, V> {
        self.iter_mut()
    }
}

/// Iterator over a map's entries, from `iter`.
pub struct Iter<'a, K, V> {
    inner: core::slice::Iter<'a, Bucket<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<(&'a K, &'a V)> {
        self.inner.next().map(|b| (&b.key, &b.value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for Iter<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|b| (&b.key, &b.value))
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}

/// Iterator over a map's entries with mutable values, from `iter_mut`.
pub struct IterMut<'a, K, V> {
    inner: core::slice::IterMut<'a, Bucket<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);
    fn next(&mut self) -> Option<(&'a K, &'a mut V)> {
        self.inner.next().map(|b| (&b.key, &mut b.value))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for IterMut<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|b| (&b.key, &mut b.value))
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {}

/// Iterator over a map's keys, from `keys`.
pub struct Keys<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;
    fn next(&mut self) -> Option<&'a K> {
        self.inner.next().map(|(k, _)| k)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for Keys<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {}

/// Iterator over a map's values, from `values`.
pub struct Values<'a, K, V> {
    inner: Iter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;
    fn next(&mut self) -> Option<&'a V> {
        self.inner.next().map(|(_, v)| v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for Values<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {}

/// Iterator over a map's values, mutably, from `values_mut`.
pub struct ValuesMut<'a, K, V> {
    inner: IterMut<'a, K, V>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = &'a mut V;
    fn next(&mut self) -> Option<&'a mut V> {
        self.inner.next().map(|(_, v)| v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> DoubleEndedIterator for ValuesMut<'_, K, V> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {}

impl<'v, 'f, K: Hash + Eq, V, S: BuildHasher> SMap<'v, 'f, K, V, S> {
    /// The entry for `key`, for in-place insertion or update.
    pub fn entry(&mut self, key: K) -> Entry<'_, 'v, 'f, K, V, S> {
        let hash = self.hasher.hash_one(&key);
        let (b, index) = self.parts();
        match lookup(b, index, hash, &key) {
            Some(index) => Entry::Occupied(OccupiedEntry { map: self, index }),
            None => Entry::Vacant(VacantEntry {
                map: self,
                hash,
                key,
            }),
        }
    }
}

/// A view into one key's place in an [`SMap`], from [`SMap::entry`].
pub enum Entry<'a, 'v, 'f, K, V, S> {
    /// The key is present.
    Occupied(OccupiedEntry<'a, 'v, 'f, K, V, S>),
    /// The key is absent.
    Vacant(VacantEntry<'a, 'v, 'f, K, V, S>),
}

/// A present key, in an [`Entry`].
pub struct OccupiedEntry<'a, 'v, 'f, K, V, S> {
    map: &'a mut SMap<'v, 'f, K, V, S>,
    index: usize,
}

/// An absent key, in an [`Entry`].
pub struct VacantEntry<'a, 'v, 'f, K, V, S> {
    map: &'a mut SMap<'v, 'f, K, V, S>,
    hash: u64,
    key: K,
}

impl<'a, K, V, S> Entry<'a, '_, '_, K, V, S> {
    /// The entry's key.
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }

    /// The value, inserting `default` if the key is absent.
    ///
    /// # Panics
    /// As [`SMap::insert`].
    pub fn or_insert(self, default: V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(default),
        }
    }

    /// The value, inserting `f()` if the key is absent. `f` runs before
    /// anything is reserved.
    ///
    /// # Panics
    /// As [`SMap::insert`].
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> &'a mut V {
        match self {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => e.insert(f()),
        }
    }

    /// The value, inserting `V::default()` if the key is absent.
    ///
    /// # Panics
    /// As [`SMap::insert`].
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }
}

impl<'a, K, V, S> OccupiedEntry<'a, '_, '_, K, V, S> {
    /// The key in the map.
    pub fn key(&self) -> &K {
        &self.map.parts().0[self.index].key
    }

    /// The entry's position in insertion order.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The value.
    pub fn get(&self) -> &V {
        &self.map.parts().0[self.index].value
    }

    /// The value, mutably.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.map.parts_mut().0[self.index].value
    }

    /// The value, mutably, for as long as the map was borrowed.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.map.parts_mut().0[self.index].value
    }
}

impl<'a, K, V, S> VacantEntry<'a, '_, '_, K, V, S> {
    /// The key that would be inserted.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Inserts the key with `value` at the end of the map.
    ///
    /// # Panics
    /// If the stack is exhausted, the capacity overflows, or an entry needs
    /// alignment above 4096.
    pub fn insert(self, value: V) -> &'a mut V {
        let i = self.map.push(self.hash, self.key, value);
        &mut self.map.parts_mut().0[i].value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_sizes() {
        assert_eq!(slots_for(0), Ok(0));
        assert_eq!(slots_for(1), Ok(8));
        assert_eq!(slots_for(7), Ok(8));
        assert_eq!(slots_for(8), Ok(16));
        assert_eq!(slots_for(14), Ok(16));
        assert_eq!(slots_for(15), Ok(32));
        assert_eq!(usable(8), 7);
        assert_eq!(slots_for(MAX_LEN + 1), Err(OVERFLOW));
        assert_eq!(block_bytes::<u64, u64>(8), Ok(7 * 24 + 32));
        assert_eq!(block_bytes::<u64, u64>(usize::MAX / 2 + 1), Err(OVERFLOW));
    }

    /// Every entry is found from its hash, and the slots hold exactly the
    /// entries `0..n`.
    fn assert_consistent(index: &[u32], hashes: &[u64]) {
        for (e, &h) in hashes.iter().enumerate() {
            slot_of(index, h, e);
        }
        let mut seen: Vec<u32> = index.iter().copied().filter(|&v| v != 0).collect();
        seen.sort_unstable();
        assert!(seen.iter().copied().eq(1..=hashes.len() as u32));
    }

    #[test]
    fn insert_and_remove_keep_every_probe_run_intact() {
        // Few distinct homes, so runs are long and wrap around the end.
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut rnd = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut index = vec![0u32; 16];
        let mut hashes: Vec<u64> = Vec::new();
        for _ in 0..4000 {
            if hashes.len() < usable(16) && rnd() % 3 != 0 {
                let h = 13 + rnd() % 5; // homes 13..=17, wrapping to 0 and 1
                insert_slot(&mut index, h, hashes.len());
                hashes.push(h);
            } else if !hashes.is_empty() {
                // swap_remove of a random entry, as `SMap` does it
                let i = (rnd() % hashes.len() as u64) as usize;
                let last = hashes.len() - 1;
                let s = slot_of(&index, hashes[i], i);
                remove_slot(&mut index, s, |e| hashes[e]);
                if i != last {
                    let s = slot_of(&index, hashes[last], last);
                    index[s] = i as u32 + 1;
                    hashes.swap(i, last);
                }
                hashes.pop();
            }
            assert_consistent(&index, &hashes);
        }
    }

    #[test]
    fn fill_index_matches_one_by_one_insertion() {
        let hashes: Vec<u64> = (0..14).map(|i| i * 7 % 5).collect();
        let mut a = vec![0u32; 16];
        fill_index(&mut a, hashes.iter().copied());
        assert_consistent(&a, &hashes);
    }
}
