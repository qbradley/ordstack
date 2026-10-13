# Disposition of the 0.1 review findings

Two external reviews were run against the crate. Neither found a
memory-safety bug.

- **Fable** received `BRIEF.md` verbatim.
- **Codex**: its content filter rejected `BRIEF.md` and a first rewording. It
  accepted `BRIEF-codex.md`, which asks the same six checks phrased as a
  correctness review.
- Codex ran compiler-only probes (38 extra borrow, variance and auto-trait
  probes, plus the existing compile-fail cases against the `cfg(miri)`
  backend). It ran no Miri and no runtime tests; its panic and arithmetic
  conclusions come from reading the source.

Tests named below are in `crates/ordstack/tests/`.

## Fable

### D1. `lib.rs` "Failure" says a failed allocation loses no value
- **Finding:** `Frame::try_alloc` drops its argument on failure, so "no value
  is lost" is false.
- **Reproduced:** yes, by reading `frame.rs` (`try_alloc_array(1)?` returns
  with `value` still owned); also reported independently by Codex.
- **Disposition:** fixed in the `lib.rs` "Failure" section. Behaviour unchanged;
  the method's own documentation was already correct.
- **Test added:** `vec.rs::documented_error_kinds_capacity_and_reclamation`
  asserts one drop after a failed `try_alloc`.

### D2. SOUNDNESS.md says every handle's brand is covariant
- **Finding:** `SVec` and `SString` are invariant in `'f` (H4 says so).
- **Reproduced:** yes, by reading H4 and the types; Codex's compiler probes
  agree.
- **Disposition:** sentence rewritten to "covariant or invariant (H4 lists
  which)". The argument was already sound, since invariance is stricter.
- **Test added:** none. The variance table is covered by the compile-fail
  cases in `tests/ui`.

### D3. `SString::with_capacity_in` / `from_str_in` `# Panics`
- **Finding:** `with_capacity_in(usize::MAX, ..)` panics with capacity
  overflow, which the `SString` docs omit.
- **Reproduced:** yes, by reading `Stack::try_bump`.
- **Disposition:** doc fixed.
- **Test added:** none (documentation only; the error kinds are asserted in
  the test above).

### Residual: `Stack::offset_of` and `Stack::release_above` are `debug_assert!`-only
- **Finding:** their contracts are not checked in release builds (H10).
- **Reproduced:** n/a; it is a design observation, not a bug.
- **Disposition:** left as is. Both are private functions, their callers are
  audited in SOUNDNESS.md, and no violating path was found.
- **Test added:** none.

## Codex

### H3. "nor allocate from its frame" is false for frozen handles
- **Finding:** a frozen handle does not borrow its frame, so an element
  destructor can allocate from the frame that holds it. Codex's `RefCell<Frame>`
  program passes the borrow checker. It is not a UB reproducer.
- **Reproduced:** yes, the program compiles and runs; the statement in
  SOUNDNESS.md was therefore false.
- **Disposition:** the H3 re-entrancy paragraph rewritten to separate (a)
  growable containers, which hold `&mut Frame`, (b) frozen handles, which
  permit re-entrant frame use soundly because their blocks lie below
  `frame.end` and every allocation, `sub`, `reset_above` and `trim_above`
  acts at or above it (with `trim_above` rounding the discard start up to a
  page), and (c) `&self` comparison, hash and format impls. No unsound case
  was found. Behaviour unchanged.
- **Test added:** `reentrant.rs` (SBox, SSlice, SFixedVec, `into_slice`, a
  partly consumed `IntoIter`, a sub-frame, each with destructors that
  allocate, take a sub-frame, `reset_above` and `trim_above`).

### Mapping alignment checked only by `debug_assert!`
- **Finding:** `region.rs` checked `base % 4096 == 0` only in debug builds;
  H6 assumed it.
- **Reproduced:** yes, by reading the source (no counterexample target found
  by Codex either).
- **Disposition:** now an `assert_eq!` with an `ordstack: ` message, in every
  build. The SAFETY comment in `release_pages`, unsafe-table row 32, H6 and the
  limitations list cite the assert. This is the only behaviour change.
- **Test added:** none. The assert cannot fail on a supported target, so it is
  exercised only as every `Stack` creation passes through it.

### `lib.rs` "no value is lost"
- **Finding:** same as Fable D1.
- **Disposition:** duplicate of D1; fixed there.

### `CapacityOverflow` doc over-claims `isize::MAX`
- **Finding:** a size above `isize::MAX` can report `Exhausted`.
- **Reproduced:** yes, by reading the code: `CapacityOverflow` comes only from
  `usize` arithmetic overflow (`checked_mul`, `checked_add`) and from
  `Region::reserve` for reservations beyond `isize::MAX` minus a page. A size
  that fits in `usize` but exceeds the reservation is `Exhausted`.
- **Disposition:** doc in `error.rs` reworded to say exactly that.
- **Test added:** `vec.rs::documented_error_kinds_capacity_and_reclamation`.

### "Room for exactly `cap` elements" for a zero-size `T`
- **Finding:** `SVec<ZST>` reports capacity `usize::MAX`.
- **Reproduced:** yes, by reading `new_in`; `SVec::with_capacity_in` was the
  only place with the claim for elements. `SString` counts bytes (never zero
  size), and `SFixedVec::capacity` returns the requested `cap`, so they need
  no change. There is no `reserve_exact`.
- **Disposition:** `SVec::with_capacity_in` doc now states the exception.
- **Test added:** same test (`SVec::<()>::with_capacity_in(3, ..).capacity()`).

### `Frame` type doc says reclamation happens when the lower object is "used"
- **Finding:** reclamation happens only when the lower object next needs the
  top; read-only queries and operations within existing capacity reclaim
  nothing.
- **Reproduced:** yes, against the crate-level "Reclamation is lazy" text.
- **Disposition:** `Frame` doc reworded to agree with it and link to it.
- **Test added:** same test (a read-only `end_offset` and dropping the frame
  leave the sub-frame's space; the next `alloc` reclaims it).

### Covariant-brand remark
- **Finding:** Codex repeated that the blanket covariance statement should say
  H4's two invariant cases.
- **Disposition:** duplicate of Fable D2; fixed there.
