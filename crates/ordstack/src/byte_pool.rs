//! Byte storage with reusable contiguous runs.

use crate::bplus::Summary;
use crate::{AllocError, AllocErrorKind, Frame, SBPlusTree, SVec};

const OVERFLOW: AllocError = AllocError::new(AllocErrorKind::CapacityOverflow);

/// A handle to a run of bytes in the pool that created it.
///
/// Use this handle only with the pool that issued it and while the run is
/// live. After a successful resize, use only the returned handle.
/// The pool does not track generations. An old handle can refer to bytes
/// that the pool has since given to another run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ByteRun {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct MaxSpan(usize);

impl Summary<usize, usize> for MaxSpan {
    const EMPTY: Self = Self(0);

    fn entry(end: usize, start: usize) -> Self {
        Self(end - start)
    }

    fn join(self, right: Self) -> Self {
        Self(self.0.max(right.0))
    }
}

const SPAN_LEAF_FANOUT: usize = 12;
type SpanTree<'v, 'f> = SBPlusTree<
    'v,
    'f,
    usize,
    usize,
    SPAN_LEAF_FANOUT,
    { SBPlusTree::<usize, usize, SPAN_LEAF_FANOUT, 8, MaxSpan>::balanced_branch_fanout() },
    MaxSpan,
>;

struct FreeSpans<'v, 'f> {
    holes: SpanTree<'v, 'f>,
    // The free tail starts here and has no tree entry.
    top: usize,
}

impl<'v, 'f> FreeSpans<'v, 'f> {
    fn new(frame: &'v mut Frame<'f>) -> Self {
        Self {
            holes: SpanTree::with_fanout_in(frame),
            top: 0,
        }
    }

    fn neighbors(&self, run: ByteRun) -> Option<[Option<ByteRun>; 2]> {
        if run.end > self.top {
            return None;
        }
        let [left, right] = self.holes.around(run.start + 1);
        if right.is_some_and(|(_, start)| start < run.end) {
            return None;
        }
        Some([left, right].map(|span| span.map(|(end, start)| ByteRun { start, end })))
    }

    fn try_free(&mut self, run: ByteRun) -> Result<bool, AllocError> {
        let Some(neighbors) = self.neighbors(run) else {
            return Ok(false);
        };
        self.release(run, neighbors)?;
        Ok(true)
    }

    fn release(
        &mut self,
        run: ByteRun,
        [left, right]: [Option<ByteRun>; 2],
    ) -> Result<Option<ByteRun>, AllocError> {
        let left = left.filter(|span| span.end == run.start);
        let right = right.filter(|span| span.start == run.end);
        let start = left.map_or(run.start, |span| span.start);
        let end = right.map_or(run.end, |span| span.end);
        let interior = end < self.top;
        if !interior {
            self.top = start;
        } else {
            // Insert before removing the left span so failure changes nothing.
            self.holes.try_insert(end, start)?;
        }
        if let Some(span) = left {
            self.holes.remove(span.end);
        }
        Ok(interior.then_some(ByteRun { start, end }))
    }

    fn first_fit(&self, len: usize) -> Option<ByteRun> {
        self.holes
            .search_summaries(|span| span.0 >= len)
            .map(|(end, start)| ByteRun { start, end })
    }

    fn claim(&mut self, end: usize, hole: Option<ByteRun>) {
        match hole {
            None => self.top = end,
            Some(span) if end == span.end => {
                self.holes.remove(span.end);
            }
            Some(span) => {
                // Key by end so a partial claim replaces an existing record.
                self.holes
                    .try_insert(span.end, end)
                    .unwrap_or_else(|_| unreachable!());
            }
        }
    }

    fn alloc(&mut self, bytes: &mut impl Bytes, len: usize) -> Result<ByteRun, AllocError> {
        assert!(len > 0, "ordstack: byte run must not be empty");
        let hole = self.first_fit(len);
        let start = hole.map_or(self.top, |span| span.start);
        let end = start.checked_add(len).ok_or(OVERFLOW)?;
        bytes.reserve_end(end)?;
        self.claim(end, hole);
        bytes.initialize_to(end)[start..end].fill(0);
        Ok(ByteRun { start, end })
    }

    fn resize(
        &mut self,
        bytes: &mut impl Bytes,
        handle: ByteRun,
        len: usize,
    ) -> Result<Option<ByteRun>, AllocError> {
        assert!(len > 0, "ordstack: byte run must not be empty");
        let Some([left, right]) = self.neighbors(handle) else {
            return Ok(None);
        };
        let old_len = handle.end - handle.start;
        if len == old_len {
            return Ok(Some(handle));
        }
        if len < old_len {
            let end = handle.start + len;
            self.release(
                ByteRun {
                    start: end,
                    end: handle.end,
                },
                [None, right],
            )?;
            return Ok(Some(ByteRun { end, ..handle }));
        }
        let adjacent = right.filter(|span| span.start == handle.end);
        let ceiling = adjacent.map_or_else(
            || {
                if handle.end == self.top {
                    bytes.limit()
                } else {
                    handle.end
                }
            },
            |span| span.end,
        );
        if len <= ceiling - handle.start {
            let end = handle.start + len;
            match bytes.reserve_end(end) {
                Ok(()) => {
                    self.claim(end, adjacent);
                    bytes.initialize_to(end)[handle.end..end].fill(0);
                    return Ok(Some(ByteRun { end, ..handle }));
                }
                Err(error) if error.kind() == AllocErrorKind::Exhausted => {}
                Err(error) => return Err(error),
            }
        }
        // Compare first fit with the gap that releasing this run creates.
        let merged_start = left
            .filter(|span| span.end == handle.start)
            .map_or(handle.start, |span| span.start);
        let hole = self.first_fit(len);
        let first = hole.map_or(self.top, |span| span.start);
        let overlaps = len <= ceiling - merged_start && merged_start <= first;
        let start = if overlaps { merged_start } else { first };
        let end = start.checked_add(len).ok_or(OVERFLOW)?;
        bytes.reserve_end(end)?;
        let released = self.release(handle, [left, right])?;
        self.claim(end, if overlaps { released } else { hole });
        let bytes = bytes.initialize_to(end);
        bytes.copy_within(handle.start..handle.end, start);
        bytes[start + old_len..end].fill(0);
        Ok(Some(ByteRun { start, end }))
    }
}

trait Bytes {
    fn limit(&self) -> usize;
    fn reserve_end(&mut self, end: usize) -> Result<(), AllocError>;
    fn initialize_to(&mut self, end: usize) -> &mut [u8];
}

impl Bytes for SVec<'_, '_, u8> {
    fn limit(&self) -> usize {
        usize::MAX
    }

    fn reserve_end(&mut self, end: usize) -> Result<(), AllocError> {
        self.try_reserve(end.saturating_sub(self.len()))
    }

    fn initialize_to(&mut self, end: usize) -> &mut [u8] {
        self.extend(core::iter::repeat_n(0, end.saturating_sub(self.len())));
        self
    }
}

/// A growable byte pool on two separate stacks.
///
/// One frame holds the byte buffer and the other holds the free-span tree. Both
/// frames stay exclusively borrowed while the pool is live. Use handles
/// only for live runs in the pool that issued them. Rust ties slice borrows
/// to the pool, so a caller cannot free or resize a run while it uses a slice.
///
/// The pool takes the first free span that fits in address order. The
/// search and tree updates take logarithmic time in the number of free
/// spans. Borrowing a run only checks the byte buffer bounds.
///
/// # Choosing a pool or a vector
///
/// A pool lets you free or resize runs independently. It stores a tree
/// entry for each hole. It does not store entries for live runs or for the
/// free space at the end. The metadata stack can retain spare nodes after
/// holes coalesce.
///
/// If run lengths are close to a small maximum, an [`SVec`] of fixed-size
/// items can use less space. Direct indexing avoids a tree lookup. The
/// vector does not manage free slots.
/// The pool saves space if the padding in fixed-size slots costs more
/// than the pool's metadata, gaps, and spare capacity.
///
pub struct SBytePool<'v, 'f, 'm, 'g> {
    bytes: SVec<'v, 'f, u8>,
    spans: FreeSpans<'m, 'g>,
}

impl<'v, 'f, 'm, 'g> SBytePool<'v, 'f, 'm, 'g> {
    /// Creates an empty pool with no initial reservation on either frame.
    pub fn new_in(bytes: &'v mut Frame<'f>, metadata: &'m mut Frame<'g>) -> Self {
        Self {
            bytes: SVec::new_in(bytes),
            spans: FreeSpans::new(metadata),
        }
    }

    /// Allocates a zeroed run. Returns an error if the byte frame cannot fit it.
    ///
    /// # Panics
    /// If `len` is zero.
    pub fn try_alloc(&mut self, len: usize) -> Result<ByteRun, AllocError> {
        self.spans.alloc(&mut self.bytes, len)
    }

    /// Borrows the indicated bytes if they are within the buffer.
    /// The pool does not track whether the run is live.
    pub fn get(&self, handle: ByteRun) -> Option<&[u8]> {
        self.bytes.get(handle.start..handle.end)
    }

    /// Mutably borrows the indicated bytes if they are within the buffer.
    /// The pool does not track whether the run is live.
    pub fn get_mut(&mut self, handle: ByteRun) -> Option<&mut [u8]> {
        self.bytes.get_mut(handle.start..handle.end)
    }

    /// Frees a whole run. Returns `false` if it overlaps free space.
    ///
    /// # Panics
    /// If the metadata frame cannot record the free span.
    pub fn free(&mut self, handle: ByteRun) -> bool {
        self.try_free(handle)
            .expect("ordstack: cannot record free byte span")
    }

    /// Frees a whole run or returns an error if the metadata frame is full.
    /// On error the run remains live. Returns `Ok(false)` for an overlap
    /// with free space, but does not detect reuse of stale handles.
    pub fn try_free(&mut self, handle: ByteRun) -> Result<bool, AllocError> {
        self.spans.try_free(handle)
    }

    /// Resizes a run, preserving the common prefix and zeroing added bytes.
    ///
    /// On success, use the returned handle. Returns `Ok(None)` if the run
    /// overlaps free space. On error the original run and handle stay valid.
    ///
    /// # Panics
    /// If `len` is zero.
    pub fn try_resize(
        &mut self,
        handle: ByteRun,
        len: usize,
    ) -> Result<Option<ByteRun>, AllocError> {
        self.spans.resize(&mut self.bytes, handle, len)
    }
}
