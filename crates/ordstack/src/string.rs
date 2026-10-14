//! [`SString`], the growable string.

use core::fmt;
use core::ops::Deref;

use crate::error::AllocError;
use crate::frame::Frame;
use crate::macros::deref_traits;
use crate::slice::SStr;
use crate::vec::SVec;

/// A growable UTF-8 string on a stack. See [`SVec`] for the rules; this is
/// an `SVec<u8>` that keeps its bytes valid UTF-8.
pub struct SString<'v, 'f> {
    vec: SVec<'v, 'f, u8>,
}

impl<'v, 'f> SString<'v, 'f> {
    /// An empty string on top of `frame`. Allocates nothing yet.
    pub fn new_in(frame: &'v mut Frame<'f>) -> SString<'v, 'f> {
        SString {
            vec: SVec::new_in(frame),
        }
    }

    /// An empty string with room for exactly `cap` bytes.
    ///
    /// # Panics
    /// If the stack is exhausted or the capacity overflows.
    pub fn with_capacity_in(cap: usize, frame: &'v mut Frame<'f>) -> SString<'v, 'f> {
        SString {
            vec: SVec::with_capacity_in(cap, frame),
        }
    }

    /// A copy of `s` on top of `frame`.
    ///
    /// # Panics
    /// If the stack is exhausted or the capacity overflows.
    pub fn from_str_in(s: &str, frame: &'v mut Frame<'f>) -> SString<'v, 'f> {
        let mut out = SString::with_capacity_in(s.len(), frame);
        out.push_str(s);
        out
    }

    /// Appends `ch`.
    ///
    /// # Panics
    /// If the stack is exhausted or the capacity overflows.
    pub fn push(&mut self, ch: char) {
        self.push_str(ch.encode_utf8(&mut [0; 4]));
    }

    /// Appends `s`.
    ///
    /// # Panics
    /// If the stack is exhausted or the capacity overflows.
    pub fn push_str(&mut self, s: &str) {
        self.vec.extend_from_slice(s.as_bytes());
    }

    /// Removes and returns the last character.
    pub fn pop(&mut self) -> Option<char> {
        let ch = self.chars().next_back()?;
        self.vec.truncate(self.vec.len() - ch.len_utf8());
        Some(ch)
    }

    /// Shortens the string to `len` bytes. Does nothing if `len` is not
    /// below the current length.
    ///
    /// # Panics
    /// If `len` is not on a character boundary. The string is unchanged.
    pub fn truncate(&mut self, len: usize) {
        if len < self.vec.len() {
            assert!(
                self.is_char_boundary(len),
                "ordstack: truncate index {len} is not a char boundary"
            );
            self.vec.truncate(len);
        }
    }

    /// Empties the string. Keeps the capacity.
    pub fn clear(&mut self) {
        self.vec.clear();
    }

    /// Makes room for at least `additional` more bytes.
    ///
    /// # Panics
    /// If the stack is exhausted or the capacity overflows.
    pub fn reserve(&mut self, additional: usize) {
        self.vec.reserve(additional);
    }

    /// Makes room for at least `additional` more bytes. On failure the
    /// string is unchanged (the stack may still have reclaimed unreachable
    /// space above it).
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError> {
        self.vec.try_reserve(additional)
    }

    /// The number of bytes the string can hold without growing.
    pub fn capacity(&self) -> usize {
        self.vec.capacity()
    }

    /// A frame above this string; see [`SVec::above`].
    pub fn above(&mut self) -> Frame<'_> {
        self.vec.above()
    }

    /// Like [`above`](Self::above), with the text still readable and
    /// writable in place.
    pub fn split_above(&mut self) -> (&mut str, Frame<'_>) {
        let (bytes, frame) = self.vec.split_above();
        // SAFETY: the bytes are valid UTF-8, and `&mut str` only allows
        // edits that keep them so.
        (unsafe { core::str::from_utf8_unchecked_mut(bytes) }, frame)
    }

    /// Freezes the string in place; see [`SVec::freeze`].
    ///
    /// # Examples
    /// ```
    /// use ordstack::{SString, Stack};
    ///
    /// let mut st = Stack::with_reserve(1 << 16);
    /// let mut f = st.frame();
    /// let mut s = SString::new_in(&mut f);
    /// s.push_str("ab");
    /// s.push('c');
    /// let frozen = s.freeze();
    /// assert_eq!(frozen.as_str(), "abc");
    /// ```
    pub fn freeze(self) -> SStr<'f> {
        // SAFETY: the bytes are valid UTF-8.
        unsafe { SStr::from_utf8_unchecked(self.vec.freeze()) }
    }

    /// Copies the string into a frozen block of `dst`; see
    /// [`SVec::freeze_into`]. `dst` cannot be the frame this string holds; in
    /// practice it is a frame of another stack.
    ///
    /// # Panics
    /// If `dst`'s stack is exhausted.
    pub fn freeze_into<'g>(self, dst: &mut Frame<'g>) -> SStr<'g> {
        // SAFETY: the bytes are valid UTF-8.
        unsafe { SStr::from_utf8_unchecked(self.vec.freeze_into(dst)) }
    }
}

impl Deref for SString<'_, '_> {
    type Target = str;
    fn deref(&self) -> &str {
        // SAFETY: every mutation keeps the bytes valid UTF-8.
        unsafe { core::str::from_utf8_unchecked(&self.vec) }
    }
}

impl fmt::Write for SString<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

impl Extend<char> for SString<'_, '_> {
    fn extend<I: IntoIterator<Item = char>>(&mut self, iter: I) {
        for ch in iter {
            self.push(ch);
        }
    }
}

impl<'a> Extend<&'a str> for SString<'_, '_> {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        for s in iter {
            self.push_str(s);
        }
    }
}

impl fmt::Display for SString<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

deref_traits!(['v, 'f] SString<'v, 'f>, str);

/// Formats into a frozen string on a frame, like `format!` on the heap.
///
/// The first argument is a `&mut Frame<'f>`; the rest are `format!`'s. The
/// result is an [`SStr<'f>`](crate::SStr). The string is built in place on
/// top of the stack and frozen without a copy.
///
/// # Panics
/// If the stack is exhausted, or if a formatting trait implementation
/// returns an error (as `format!` does). A string that was partly written
/// when a panic occurred is left to be reclaimed.
///
/// # Examples
/// ```
/// use ordstack::{Stack, sformat};
///
/// let mut st = Stack::with_reserve(1 << 16);
/// let mut f = st.frame();
/// let name = sformat!(&mut f, "{}_{}", "node", 7);
/// assert_eq!(name.as_str(), "node_7");
/// ```
#[macro_export]
macro_rules! sformat {
    ($frame:expr, $($arg:tt)*) => {{
        let mut s = $crate::SString::new_in($frame);
        ::core::fmt::Write::write_fmt(&mut s, ::core::format_args!($($arg)*))
            .expect("ordstack: a formatting trait implementation returned an error");
        s.freeze()
    }};
}
