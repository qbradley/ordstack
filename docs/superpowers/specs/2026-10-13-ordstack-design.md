# ordstack — design

Date: 2026-10-13
Status: draft for owner review
Origin: `malloc-stacks` issue #3 and the throwaway spike in `malloc-stacks/stackfit_spike/`

## 1. Purpose

`ordstack` is a Rust library of stack-allocated containers in which the stock
borrow checker proves the stack discipline. It is for code that wants bump
allocation with growable containers and wants the "only the top block grows"
rule enforced at compile time rather than tested at run time.

**The guarantee.** For every stack, in safe code:

1. only the top block is ever grown, and it grows in place;
2. memory is reclaimed only when no live value can reach it.

Because of (1) and (2) the allocator keeps no per-block header, never copies a
block in order to grow it, and never abandons a buffer because it had to grow
from beneath another block. Growth asserts in every build that the block being
grown is the top one; the borrow rules make that assertion unreachable from
safe code.

**What it is not.** It is a typed region arena with a growable top. Frozen
blocks have no individual free; they are reclaimed in bulk when their frame is
reused. The library makes no claim that every free is a pop, and no speed or
memory claim against any general-purpose allocator.

**Success criteria for 0.1.**

- The public API contains no `unsafe` function and needs no `unsafe` in user
  code.
- Every check in section 8 passes, and the adversarial review gate in
  section 8.6 is closed.
- A reader can learn the model from the crate documentation alone.

## 2. Scope

### In 0.1

- `Stack`, `Frame`
- Growable: `SVec`, `SString`
- Frozen: `SBox`, `SSlice`, `SStr`
- Fixed capacity: `SFixedVec`
- `AllocError`
- `SOUNDNESS.md`, the test suites of section 8, crate documentation, README

### In 0.2 (section 10; planned separately after 0.1 is tagged)

- `SMap`, `SFrozenMap`
- serde `DeserializeSeed` support and the `ordstack-derive` crate
- `CloneIn`
- Three ported programs as the acceptance test

### Out of scope

- An implementation of `core::alloc::Allocator` or `allocator-api2`. Standard
  containers allocate through `&self` and so cannot carry the proof.
- A global allocator.
- `Send` or `Sync` for any type; cross-thread hand-off.
- `no_std` and caller-supplied buffers (reconsidered after 0.1).
- Windows (reconsidered after 0.1).
- A static analysis pass over user code.

## 3. Model

A stack is one contiguous address range and a single offset `top`. Every block
lies in `[0, top)`.

Three kinds of object exist:

- A **frame** is an allocation cursor. It owns the range from its start to its
  `end` offset, which holds the frozen blocks it has allocated. Fresh
  allocations go at `end`.
- A **growable container** holds a frame exclusively and places its buffer at
  that frame's `end`. While the container is alive, nothing else can allocate
  through that frame, so the buffer is the top block of that frame.
- A **frozen block** is a value at a fixed address and size, valid for the
  frame's brand lifetime `'f`.

New space above an object is obtained only by mutably borrowing that object:

| Call | Borrows | Yields |
|---|---|---|
| `Stack::frame(&mut self)` | the stack | `Frame<'_>` at offset 0 |
| `Frame::sub(&mut self)` | the frame | `Frame<'_>` starting at this frame's `end` |
| `SVec::above(&mut self)` | the vector | `Frame<'_>` starting at the buffer's capacity end |
| `SVec::split_above(&mut self)` | the vector | `(&mut [T], Frame<'_>)` |

While the yielded frame, or anything allocated through it, is alive, the
object beneath is mutably borrowed: it can be neither grown nor used to
allocate. That is the freeze. `SString` has the same two methods.

### 3.1 The invariant

> **I.** Every pointer into the region above an object's end carries a
> lifetime that keeps that object mutably borrowed.

Consequences, each of which `SOUNDNESS.md` must derive:

- When any `&mut` method on a frame or growable container runs, nothing above
  that object's end is reachable. The method may therefore set `top` to the
  object's own end before doing anything else.
- A growable container's buffer ends at or below `top`, and after that reset
  it ends exactly at the space the container may extend into. Growth is a
  bump of `top`.
- A frozen block lies below its frame's `end`, and `end` never decreases while
  the brand `'f` is alive.

### 3.2 Reclamation is lazy

No type reclaims memory in `Drop`. A frame that is dropped, forgotten with
`mem::forget`, or abandoned by unwinding leaves `top` where it was. The space
is reclaimed the next time the object beneath it is used through `&mut`
("reset on reacquire"). Explicit reclamation:

- `Frame::reset_above(&mut self)` sets `top` to this frame's `end`.
- `Stack::reset(&mut self)` sets `top` to 0. It needs the whole stack
  unborrowed, so every frame and block is already dead.

Leaking any handle is safe: the worst outcome is a destructor that does not
run and space held until the parent is next used.

### 3.3 Lifetimes

- `Frame<'f>`: `'f` is the brand. All frozen blocks from this frame carry
  `'f`, so they can coexist and outlive the `&mut` borrow used to allocate
  them.
- `SVec<'v, 'f, T>` holds `&'v mut Frame<'f>`. `'v` is how long the frame is
  held; `'f` is what a later `freeze` returns.
- `SOUNDNESS.md` records the variance of every type in each parameter and why
  it is sound. `&'v mut Frame<'f>` makes growable containers invariant in
  `'f`. Frozen handles are covariant in `'f` and `T`.

## 4. Public API (0.1)

Signatures are normative in shape; names may change only where the plan
records a reason.

### 4.1 `Stack`

```rust
impl Stack {
    pub fn new() -> Stack;                                  // DEFAULT_RESERVE
    pub fn with_reserve(bytes: usize) -> Stack;             // panics on failure
    pub fn try_with_reserve(bytes: usize) -> Result<Stack, AllocError>;
    pub fn frame(&mut self) -> Frame<'_>;                   // resets top to 0
    pub fn reset(&mut self);
    pub fn trim(&mut self);                                 // reset, then release all pages
    pub fn capacity(&self) -> usize;
    pub fn extent(&self) -> usize;                          // current top
    pub fn high_water(&self) -> usize;
}
```

`Stack` implements `Default`, `Debug` and `Drop` (unmaps the range). It is
neither `Send` nor `Sync`.

### 4.2 `Frame<'f>`

```rust
impl<'f> Frame<'f> {
    pub fn sub(&mut self) -> Frame<'_>;
    pub fn reset_above(&mut self);
    pub fn trim_above(&mut self);                           // release pages above end
    pub fn alloc<T>(&mut self, value: T) -> SBox<'f, T>;
    pub fn try_alloc<T>(&mut self, value: T) -> Result<SBox<'f, T>, AllocError>;
    pub fn alloc_with<T>(&mut self, f: impl FnOnce() -> T) -> SBox<'f, T>;
    pub fn alloc_slice_copy<T: Copy>(&mut self, src: &[T]) -> SSlice<'f, T>;
    pub fn alloc_slice_clone<T: Clone>(&mut self, src: &[T]) -> SSlice<'f, T>;
    pub fn alloc_str(&mut self, src: &str) -> SStr<'f>;
    pub fn end_offset(&self) -> usize;
}
```

`Frame` has no `Drop`. `alloc_with` runs `f` before reserving space, so `f`
cannot observe a half-initialised block; it cannot allocate from this frame
because the frame is mutably borrowed.

### 4.3 Frozen handles

- `SBox<'f, T>`: owns one `T`. `Deref`, `DerefMut`, and the associated
  functions `SBox::into_inner(b) -> T` and `SBox::leak(b) -> &'f mut T` (as
  on `Box`, so they cannot shadow methods of `T`). Dropping it drops the
  `T`; the bytes stay until the frame is reclaimed.
- `SSlice<'f, T>`: owns `[T]`. `Deref<Target = [T]>`, `DerefMut`,
  `IntoIterator` by value, `leak`. Drops elements back to front.
- `SStr<'f>`: owns `str`. `Deref<Target = str>`, `as_str`, `leak`.

All three implement `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`
and `Borrow` where the target does; `SStr` also implements `Display`.

### 4.4 `SVec<'v, 'f, T>`

```rust
impl<'v, 'f, T> SVec<'v, 'f, T> {
    pub fn new_in(frame: &'v mut Frame<'f>) -> Self;
    pub fn with_capacity_in(cap: usize, frame: &'v mut Frame<'f>) -> Self;
    pub fn from_iter_in<I: IntoIterator<Item = T>>(iter: I, frame: &'v mut Frame<'f>) -> Self;

    pub fn push(&mut self, value: T);
    pub fn try_push(&mut self, value: T) -> Result<(), (T, AllocError)>;
    pub fn pop(&mut self) -> Option<T>;
    pub fn extend_from_slice(&mut self, src: &[T]) where T: Copy;
    pub fn insert(&mut self, index: usize, value: T);
    pub fn remove(&mut self, index: usize) -> T;
    pub fn swap_remove(&mut self, index: usize) -> T;
    pub fn truncate(&mut self, len: usize);
    pub fn clear(&mut self);
    pub fn retain(&mut self, f: impl FnMut(&T) -> bool);
    pub fn reserve(&mut self, additional: usize);
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), AllocError>;
    pub fn shrink_to_fit(&mut self);                        // lowers top; never copies
    pub fn capacity(&self) -> usize;

    pub fn above(&mut self) -> Frame<'_>;
    pub fn split_above(&mut self) -> (&mut [T], Frame<'_>);

    pub fn freeze(self) -> SSlice<'f, T>;
    pub fn freeze_into<'g>(self, dst: &mut Frame<'g>) -> SSlice<'g, T>;
}
```

Also `Deref<Target = [T]>`, `DerefMut`, `Extend<T>`, `Debug`, comparison
traits, and `Drop`.

- **Growth** doubles capacity (minimum 4 elements) by bumping `top`. No copy.
- **`above` and `split_above`** start the new frame at the capacity end, so
  the vector keeps its spare capacity.
- **`freeze`** shrinks the buffer to its length, advances the frame's `end`
  past it and returns the block. The frame can be used again afterwards and
  the block sits beside its other frozen blocks.
- **`freeze_into`** moves the elements bitwise into a block on `dst` and
  abandons the source buffer without dropping elements. In practice `dst`
  is a frame of another stack: every frame of this stack that could be named
  here is either borrowed by the vector or borrows it.
- **Drop order.** `Drop`, `truncate`, `clear` and `SSlice`'s `Drop` destroy
  elements back to front. If an element's destructor panics, the rest are
  still dropped, as in `std`; a second panic aborts.
- **Element destructors cannot reach the frame**, since the vector holds it.

### 4.5 `SString<'v, 'f>`

A UTF-8 wrapper over `SVec<'v, 'f, u8>`: `new_in`, `with_capacity_in`,
`from_str_in`, `push`, `push_str`, `pop`, `truncate`, `clear`, `reserve`,
`try_reserve`, `above`, `split_above`, `freeze() -> SStr<'f>`, `freeze_into`,
`Deref<Target = str>`, `fmt::Write`, `Extend<char>`, `Extend<&str>`, `Debug`,
`Display`.

### 4.6 `SFixedVec<'f, T>`

A vector whose capacity is fixed at creation. It is a frozen block, so it
does not hold the frame and the frame can allocate above it. This is what
lets a list's elements own blocks on the same stack as the list.

`with_capacity_in(cap, &mut Frame<'f>)`, `push` (panics when full),
`try_push -> Result<(), T>`, `pop`, `len`, `capacity`, `is_full`,
`Deref<Target = [T]>`, `DerefMut`, `into_slice(self) -> SSlice<'f, T>`
(spare capacity is not reclaimed), `Drop` back to front.

### 4.7 Errors and panics

`AllocError` is an opaque `Copy` type implementing `Error` and `Display`,
with `kind()` returning `Exhausted`, `CapacityOverflow`, `AlignTooLarge` or
`MapFailed`.

- Every infallible allocating method panics on any of these. The fallible
  counterparts in 0.1 are `try_with_reserve`, `try_alloc`, `try_push` and
  `try_reserve`; reserving first makes any later push infallible.
- A failed allocation changes nothing: `top`, the container's length and its
  capacity are as before.
- All size arithmetic is checked.

## 5. Memory

- **Region.** One anonymous private mapping of the whole reservation, created
  with `PROT_READ | PROT_WRITE` and `MAP_NORESERVE`; the kernel commits pages
  on first touch. No guard page is needed because every allocation is bounds
  checked against capacity.
- **`DEFAULT_RESERVE`** is 4 GiB on 64-bit targets and 64 MiB on 32-bit
  targets. Reservation sizes are rounded up to the page size. A zero
  reservation is allowed and makes every non-zero-size allocation fail.
- **Under Miri** (`cfg(miri)`) the region is a heap allocation of the
  requested size, because Miri rejects these mapping flags. Tests under Miri
  use small reservations.
- **Returning pages.** Never automatic. `Stack::trim` and `Frame::trim_above`
  call `madvise` with `MADV_DONTNEED` on Linux and `MADV_FREE` on macOS over
  the whole pages above the relevant offset. Under Miri they are no-ops.
- **Alignment.** The base is page aligned. Each block is aligned by rounding
  its start up. Alignments above 4096 fail with `AlignTooLarge`.
- **Zero-size types** take no space and never fail. Their handles use a
  dangling aligned pointer.
- **Overhead.** No per-block header. `Frame` is two words; `SVec` is four.

## 6. Crate structure

A Cargo workspace from the start, since 0.2 adds a second crate.

```
ordstack/
  Cargo.toml                 workspace
  README.md  LICENSE-MIT  LICENSE-APACHE  CHANGELOG.md  SOUNDNESS.md
  crates/ordstack/
    src/lib.rs               crate docs, re-exports
    src/region.rs            OS mapping; the only file that calls libc
    src/stack.rs             Stack, top, bump/extend/reset primitives
    src/frame.rs             Frame
    src/boxed.rs             SBox
    src/slice.rs             SSlice, SStr
    src/vec.rs               SVec
    src/string.rs            SString
    src/fixed.rs             SFixedVec
    src/error.rs             AllocError
    src/macros.rs            trait impls that delegate to a Deref target
    src/debug.rs             debug-build shadow (cfg(debug_assertions))
    tests/                   behaviour, drop order, panic paths, model tests
    tests/ui/                trybuild compile-fail cases
    fuzz/                    cargo-fuzz target
```

- **Boundaries.** `region` knows nothing about blocks. `stack` exposes a small
  internal set of primitives (`reset_to`, `bump`, `extend`, `shrink`) and is
  the only module that writes `top`. Containers call those primitives and
  never do offset arithmetic on the region themselves.
- **Unsafe policy.** `#![deny(unsafe_op_in_unsafe_fn)]`,
  `#![warn(missing_docs)]`, `clippy::undocumented_unsafe_blocks` denied.
  Every `unsafe` block has a `SAFETY:` comment that cites invariant I or a
  local invariant by name.
- **Dependencies.** `libc` on Unix. Nothing else at run time. Development:
  `trybuild`, `proptest`, `libfuzzer-sys`.
- **Toolchain.** Edition 2024, MSRV 1.85, stable only. Nightly is used only
  to run Miri, AddressSanitizer and the fuzzer.
- **Platforms.** Linux, 64-bit, tested; macOS is intended to work but is not
  yet tested. Other Unix targets compile
  on a best-effort basis. Non-Unix targets fail with a `compile_error!` that
  names the limitation.
- **License.** `MIT OR Apache-2.0`.

## 7. Documentation

- **Crate docs** teach the model in this order: the guarantee, one worked
  example, freeze and descend, frozen blocks, lazy reclamation, using several
  stacks, patterns (scratch-then-copy tree building; fixed-capacity child
  lists), and the limits.
- **Limits stated plainly:** no growable field inside a struct that also
  needs the frame; two containers that grow in alternation need two stacks;
  lifetime parameters spread through signatures; frozen blocks are not freed
  individually.
- **README** carries a short comparison with `bump-scope`, `bumpalo` and
  `dyn-stack`, restricted to claims verified from their sources: `ordstack`
  rejects at compile time what those crates handle with a run-time check, a
  copy, or fixed sizes. It makes no performance claim.
- **Every public item** has a doc comment and, where sensible, an example.

## 8. Verification

### 8.1 `SOUNDNESS.md`

- States invariant I and the local invariants of each type.
- Argues, for every public function, that it preserves them.
- Has a named section for each known hazard:
  - `mem::forget` and `ManuallyDrop` on every handle type;
  - drop-check: a `T` whose destructor reads a frozen block of the same or a
    lower frame;
  - unwinding out of user code called by the library (`Clone`, iterators,
    `retain` predicates, element destructors, `alloc_with` closures);
  - variance of every type;
  - re-entrancy: user code running while a container is in an intermediate
    state;
  - zero-size types and maximum alignment;
  - arithmetic overflow.
- It is an argument, not a mechanised proof, and says so.

### 8.2 Compile-fail tests

`trybuild` cases, pinned to the MSRV toolchain in CI, for every rejection the
guarantee depends on. At least:

- pushing to a vector while a frame from `above` is alive;
- allocating from a frame while a sub-frame, or a vector holding it, is alive;
- using a block after `Stack::reset`, after the stack is dropped, or after
  its frame's parent is reused;
- returning a block from the scope of its stack;
- the `ManuallyDrop` and `split_above` cases recorded in the spike;
- moving a `Stack`, `Frame` or any handle to another thread.

Each case has a matching compiling test that differs only in the offending
line.

### 8.3 Dynamic checks

- **Miri** runs the full test suite under Stacked Borrows and under Tree
  Borrows.
- **AddressSanitizer** runs the suite on the real mapping.
- **Model tests** (`proptest`): random sequences of operations over a small
  tree of frames and containers are applied both to `ordstack` and to a
  shadow model built from `Vec`. Contents, lengths and drop order must agree,
  and every element must be dropped exactly once.
- **Fuzz target** drives the same interpreter from bytes, run under ASan.
- **Panic-path tests:** destructors, `Clone` impls and closures that panic at
  each possible point, checked for no double drop and a usable stack
  afterwards.
- **Drop-order tests** for every type that owns more than one value.

### 8.4 Debug-build shadow

With `debug_assertions`, each thread keeps a table of its live regions and
their current `top`. Every access through a handle asserts that the handle's
block lies below `top`, and every grow asserts that the block ends exactly
at `top`. These assertions are expected never to fire; a firing is a
soundness bug. Release builds contain none of this.

The shadow does not record individual blocks: a record of "live" blocks
would fire when a handle is forgotten with `mem::forget` and its space is
later reclaimed, which is safe.

### 8.5 Continuous integration

A GitHub Actions workflow is written with the crate but runs only once the
repository is pushed. A `scripts/check.sh` runs the same jobs locally:
format, clippy, tests on stable and MSRV, docs with warnings denied,
trybuild, Miri under both models, ASan, a time-boxed fuzz run.

### 8.6 Review gate

Before 0.1 is tagged, Codex and Fable each review the code and
`SOUNDNESS.md` adversarially and independently, with the brief to find
unsoundness in safe code. Every finding is fixed, or refuted in
`SOUNDNESS.md` with a test. The reviews and dispositions are kept in
`docs/reviews/`.

## 9. Release of 0.1

- Nothing is pushed to GitHub or published to crates.io until the owner asks.
- Before publication: confirm the names `ordstack` and `ordstack-derive` are
  free on crates.io; run `cargo publish --dry-run`; check the rendered docs.
- The README and crate docs label 0.x as subject to breaking changes and
  describe the soundness evidence accurately: argued and tested, not proved.

## 10. Release 0.2 (outline)

Each item gets its own plan once 0.1 is tagged. The outline fixes intent, not
signatures.

- **`SMap<'v, 'f, K, V>` and `SFrozenMap<'f, K, V>`.** Insertion-ordered: an
  entry array plus an index table in one block that grows and re-indexes in
  place. Drops entries in reverse insertion order. Removal is added
  (`swap_remove` and an order-preserving `shift_remove`). The hasher is a
  type parameter with a default.
- **Serde.** `DeserializeSeed` implementations that deserialize into a frame
  for the containers and frozen handles, and `Serialize` for all of them.
  `ordstack-derive` provides a derive that generates the seed for user
  structs and enums, so no hand-written visitors are needed. Behind a `serde`
  feature.
- **`CloneIn`.** A trait for deep-copying a value onto a frame, implemented
  for the library's types and derivable.
- **Acceptance.** `treebuild`, `serde_json_xform` and `wordfreq` from
  `malloc-stacks` are rewritten against the published API with no `unsafe`,
  identical output, and no hand-written visitor. Their line counts and the
  share of bytes served from the stacks are reported beside the spike's
  figures.

## 11. Relation to the spike

`stackfit_spike` proved the mechanism and exposed the design changes adopted
here: branded frames, two-lifetime growables, lazy reclamation, `SFixedVec`.
The crate is rewritten from this spec. Spike code is read for reference and
its tests and compile-fail cases are carried over as cases, not copied as
implementation.
