# ordstack — allocator adapter (issue #4) — design

Status: approved in conversation 2026-10-14, for written review. Extends
[`2026-10-13-ordstack-design.md`](2026-10-13-ordstack-design.md) (the
spec) and [`2026-10-14-ordstack-0.2-design.md`](2026-10-14-ordstack-0.2-design.md).
Everything there still holds; this adds one type and one method behind a
cargo feature and changes no existing signature or behaviour.

## 1. Goal

Code that is generic over an allocator runs on an ordstack frame:
`allocator_api2::vec::Vec<T, A>`, `hashbrown::HashMap<K, V, S, A>`, and
anything else bounded by `A: allocator_api2::alloc::Allocator`. Several
such collections live on one frame at once, grow in place when they are on
top, copy when they are not, and are reclaimed together. This is a way in
for code being ported from `Vec`, alongside `SVec`/`SString`/`SMap`, not a
replacement for them.

Success: the tests in section 6 pass natively, under Miri (Stacked and
Tree Borrows) and on Rust 1.85, and `scripts/check.sh` is green.

## 2. Deviation from the issue

The issue proposed `impl Allocator for &Frame`. That needs `Frame::end`
in a `Cell` (the trait allocates through `&self`), which changes every
`Frame` method and invariant E, and memory allocated that way could not be
reclaimed before the frame itself ends. Instead an adapter borrows the
frame mutably, as `SVec` does, and allocates through the stack's existing
`Cell`s (`Stack::try_bump`/`try_extend`/`reset_to` already take `&self`).
`Frame` does not change. The issue's acceptance items carry over with
`&FrameAlloc` in place of `&Frame`.

## 3. API

Feature `allocator-api2` (off by default) adds the optional dependency
`allocator-api2 = "0.2"` (default features, so its `Vec` is available).

```rust
impl<'f> Frame<'f> {
    /// An allocator for collections generic over `allocator_api2`'s
    /// `Allocator`. This frame is frozen while it, or anything allocated
    /// through it, is alive; everything allocated through it is reclaimed
    /// when the frame is next used after that.
    pub fn allocator(&mut self) -> FrameAlloc<'_, 'f>;
}

/// Allocates on the top of the stack above its frame.
pub struct FrameAlloc<'v, 'f> { frame: &'v mut Frame<'f> }

unsafe impl allocator_api2::alloc::Allocator for FrameAlloc<'_, '_>;
```

`allocator_api2` implements `Allocator for &A` for every `A: Allocator`,
so both forms work: `Vec::new_in(&a)` (many collections share `a`) and
`Vec::new_in(f.allocator())` (one collection owns it). `FrameAlloc` is not
`Clone`. It is `!Send` and `!Sync` because `Stack` is `!Sync`.

Re-export: `ordstack::FrameAlloc` under the feature.

## 4. Behaviour

`allocator()` resets the stack to `frame.end` (reset on reacquire, as
`sub()` and `SVec::above` do). While the adapter lives, nothing else can
allocate on the stack: the frame is borrowed by it and every frame below
is frozen by the borrows that created this one. So the adapter owns
`[frame.end, region end)`, and its operations act on the stack top:

| Operation | Behaviour |
|---|---|
| `allocate(l)` | `try_bump(l.size(), l.align())`. Zero-size layouts get a real, aligned zero-byte block at the top, so every pointer the adapter returns lies in the region and needs no special case later. |
| `deallocate(p, l)` | If `p + l.size() == top`, reset top to `p`. Otherwise nothing; the space comes back when the adapter's memory is reclaimed. |
| `grow(p, old, new)` | In place when `p + old.size() == top` and `p` is aligned to `new.align()`: `try_extend` to `p + new.size()`. Otherwise allocate, copy `old.size()` bytes, deallocate the old block. |
| `grow_zeroed` | As `grow`, then zero `[old.size(), new.size())`. |
| `shrink(p, old, new)` | If `p` is aligned to `new.align()`: at the top, reset top to `p + new.size()`; elsewhere keep the block. Otherwise allocate, copy `new.size()` bytes, deallocate. |

Errors: `align > MAX_ALIGN` (4096), arithmetic overflow and exhaustion
return `allocator_api2::alloc::AllocError`. A failed operation changes
nothing (the stack's existing "a failed allocation changes nothing"
argument applies: `try_bump`/`try_extend` move `top` only on success).
The adapter never panics on user input.

No user code runs inside any adapter method, so H3 (unwinding and
re-entrancy) is satisfied trivially. A panic in user code between calls
(a `Drop` or iterator inside `Vec::extend`) leaves the stack consistent,
because every adapter call is complete before it returns.

## 5. Soundness argument (new SOUNDNESS.md material)

- **New per-function rows** for `Frame::allocator` and each trait method;
  **new unsafe-block rows** for the trait impl itself (`unsafe impl`) and
  every `unsafe` block in it (pointer arithmetic, `copy_nonoverlapping`,
  `write_bytes`). The counts the doc states and
  `scripts/check-unsafe-table.sh` checks are updated, and the
  `reset_to` call-site table gets a `FrameAlloc::` row.
- **I.** Every block the adapter returns lies above `frame.end`. A
  collection that holds it holds `&FrameAlloc` or the `FrameAlloc`
  itself, both of which keep the frame mutably borrowed, which is what I
  requires.
- **T.** Allocation and in-place growth move `top` to the block's end.
  `deallocate` and `shrink` lower `top` only for the top block, so `top`
  stays `>=` the end of every live block.
- **E.** `frame.end` does not change. The adapter's blocks are not frozen
  blocks and carry no `'f` brand: their lifetime is the adapter borrow.
- **The trait contract.** "Memory stays valid while the allocator, or a
  copy, is alive": a block is released only by `deallocate`/`shrink` on
  that block, or by a reset, and no reset can happen while the adapter is
  borrowed or alive. Blocks never overlap: fresh blocks are bumped above
  `top`, and in-place growth requires the block to be the top block.
  `grow`/`shrink` return a block of the requested size and alignment.
- **H8 (threads).** `!Send`/`!Sync`; a compile-fail case proves it.
- **H9 (debug shadow).** All top changes go through `Stack::set_top`.

## 6. Tests

New file `crates/ordstack/tests/allocator.rs`, `#![cfg(feature =
"allocator-api2")]`, with `hashbrown` as a dev-dependency
`= "0.17"` (`default-features = false`, features `allocator-api2` and
`default-hasher`; it depends on allocator-api2 0.2 and its MSRV is 1.85). All of these run under Miri and on 1.85:

1. `vec_push_grow_drop`: an `allocator_api2::vec::Vec<Tracked, &FrameAlloc>`
   pushed past several growths matches a `std::vec::Vec`, and every
   element is dropped exactly once.
2. `growth_at_the_top_is_in_place`: a lone vector's pointer stays the same
   across `reserve` calls, and `Stack::extent` grows by exactly the extra
   bytes.
3. `interleaved_vectors_copy`: two vectors pushed alternately both hold the
   right values; the lower one moves when it grows, the upper one does not.
4. `deallocating_the_top_block_reclaims_it`: drop the top vector, and the
   next allocation through the adapter gets the same address.
5. `shrink_at_the_top_releases_the_tail`: `shrink_to_fit` keeps the
   pointer and lowers `Stack::extent`.
6. `memory_is_reclaimed_when_the_adapter_ends`: after the adapter's scope,
   `f.alloc` returns the address of the adapter's first block.
7. `hashbrown_map_on_a_frame`: a `hashbrown::HashMap<u32, String, _,
   &FrameAlloc>` with 10 000 entries (100 under Miri) answers every lookup.
8. `a_panic_during_extend_leaves_everything_usable`: an iterator that
   panics partway through `extend` (after at least one growth), caught
   with `catch_unwind`; every element is dropped exactly once, and the
   vector and frame are usable afterwards.
9. `errors_change_nothing`: on a small stack, `try_reserve` past the end
   fails, the vector's contents and `extent` are unchanged; a layout with
   alignment 8192 is refused by `allocate`.
10. `zero_size`: zero-size layouts and a `Vec<(), _>` work, and their
    pointers are aligned.
11. `grow_and_shrink_change_alignment`: direct `Allocator` calls that grow
    and shrink with a larger alignment return correctly aligned blocks
    holding the old bytes.

Compile-fail cases (`tests/ui/fail`, each with its passing twin):

- `20_alloc_while_frame_allocator_lives`: `f.alloc` while a vector built
  on `f.allocator()` is still used.
- `21_vec_outlives_frame_allocator`: a `Vec<_, &FrameAlloc>` used after
  the adapter is dropped.
- `22_stack_dropped_under_owned_allocator`: the stack dropped while a
  `Vec<_, FrameAlloc>` lives.
- `23_frame_allocator_not_send`: a `FrameAlloc` sent to another thread.

The UI suite and its twins run with the feature enabled.

**Model and fuzz.** `tests/common/model.rs` gains an `alloc_scope`,
reached like `map_scope`: two allocator-api2 vectors of `Tracked` on one
adapter, driven by push/pop/reserve/shrink/clear ops interleaved between
them and compared with `std` vectors, ending by dropping them in either
order; the existing blocks below are re-checked afterwards. The scope is
compiled only with the feature, so `tests/model.rs` and the fuzz crate
(`crates/ordstack/fuzz/Cargo.toml` enables the feature) exercise it.

## 7. Build, CI and docs

- `scripts/check.sh` and the CI workflow run the native, release, MSRV,
  UI, Miri (both models) and ASan steps with `--all-features`; clippy
  already uses `--all-targets` and gains `--all-features`. One feature-off
  `cargo test` step stays, so the default build is still tested.
- Crate docs: a section "Collections generic over an allocator" with an
  example of two `allocator_api2` vectors and a `hashbrown` map on one
  frame, and when to choose which: `SVec`/`SMap` give `freeze` into
  frame-lifetime handles, `freeze_into`, insertion order and no dead space;
  `Vec<T, &FrameAlloc>` gives drop-in code reuse and several growable
  collections per frame, but growth that is not at the top copies and
  leaves dead space until the adapter ends, and nothing outlives the
  adapter.
- README: one paragraph naming the feature and linking the crate-docs section.
- CHANGELOG: an entry.

## 8. Out of scope

- A nightly `std::alloc::Allocator` impl. (allocator-api2's own `nightly`
  feature may cover it; not claimed or tested here.)
- Collections that outlive the adapter (would need `&Frame` allocation,
  section 2).
- Benchmarks.
