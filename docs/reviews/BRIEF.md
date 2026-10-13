You are reviewing `ordstack`, a Rust crate whose public API is entirely safe
and whose claim is that the borrow checker proves a stack discipline.

Your job is to break it. Find a program that uses only safe code and the
public API of `crates/ordstack` and causes undefined behaviour: a use after
reclaim, two live `&mut` to the same memory, a double drop, a read of
uninitialised memory, an out-of-bounds access, or a data race.

Read `SOUNDNESS.md` first, then `crates/ordstack/src/`. Attack in this order:
1. Lifetimes and variance: can a brand be lengthened, a frame duplicated,
   swapped, or smuggled into a longer-lived place?
2. `mem::forget`, `ManuallyDrop`, `mem::swap`, `mem::replace`, `Option::take`
   on every handle type.
3. Drop check: element types with destructors that read other blocks.
4. Panics in user code at every call-out listed in SOUNDNESS.md H3.
5. Zero-size types, over-aligned types, capacity and offset arithmetic.
6. Anything SOUNDNESS.md asserts without a reason.

For each finding give: a complete program, what goes wrong, and which
sentence of SOUNDNESS.md is false. Separately list claims in the README or
docs that the code does not support. If you find nothing, say which attacks
you tried. Do not report style issues.
