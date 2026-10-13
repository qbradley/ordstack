Please do a careful correctness review of `ordstack`, a Rust arena-allocator library in this directory (my own crate, before its first release). The public API is entirely safe Rust; internally it uses `unsafe` blocks, each justified in `SOUNDNESS.md`.

The question: is every `unsafe` block's justification actually true for all programs a user can write with safe Rust and the public API of `crates/ordstack`? I want to know about any case where the library's internal invariants (a reference outliving its storage, two exclusive references to one place, a value dropped twice, reading a slot that was never written, indexing past a buffer, or a missing Send/Sync bound) could be violated by a safe caller.

Read `SOUNDNESS.md` first, then `crates/ordstack/src/`. Review in this order:
1. Lifetimes and variance of every handle type: can a brand lifetime be lengthened, or a frame be duplicated, swapped, or moved somewhere longer-lived?
2. Behaviour under `mem::forget`, `ManuallyDrop`, `mem::swap`, `mem::replace`, `Option::take` for every handle type.
3. Drop check: element types whose destructors read other blocks.
4. Panics in caller-supplied code at every call-out listed in SOUNDNESS.md H3.
5. Zero-size types, over-aligned types, capacity and offset arithmetic.
6. Any statement in SOUNDNESS.md given without a reason.

For each bug give: a complete Rust test program, what goes wrong, and which sentence of SOUNDNESS.md is false. Separately list statements in the README or docs that the code does not support. If you find nothing, say which checks you tried and why each one held. No style comments. Skip `docs/superpowers/` (planning notes).
