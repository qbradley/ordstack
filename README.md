# ordstack

Stack-allocated growable containers for Rust, where the borrow checker
proves the stack discipline.

- Only the top block of a stack ever grows, and it grows in place.
- Memory is reclaimed only when nothing can reach it.
- Code that would break either rule does not compile. There is no per-block
  header, no copy-on-grow and no slow path for a block that is not on top.

```rust
use ordstack::{SVec, Stack};

let mut st = Stack::new();
let mut f = st.frame();
let mut tokens: SVec<&str> = SVec::new_in(&mut f);
tokens.extend("let x = 1".split(' '));
{
    // `tokens` is frozen from here: readable through `toks`, not growable.
    let (toks, mut above) = tokens.split_above();
    let mut lens: SVec<usize> = SVec::new_in(&mut above);
    lens.extend(toks.iter().map(|t| t.len()));
    assert_eq!(&*lens, [3, 1, 1, 1]);
    // tokens.push("oops"); // would not compile here
}
tokens.push(";"); // top again
assert_eq!(tokens.len(), 5);
```

## What it is and is not

`ordstack` is a typed region arena with a growable top. Frozen blocks are
reclaimed in bulk, not one by one, and reclamation is lazy: dropping a handle
runs destructors but returns no memory. It is not an `Allocator` for `std`
collections, not a global allocator, and not thread-safe. It makes no speed
or memory claim against general-purpose allocators.

The crate documentation teaches the model: the guarantee, freeze and descend,
frozen blocks, lazy reclamation, failure, using several stacks, and the
limits.

## How it differs from other bump allocators

What happens when a container that is not the most recent allocation has to
grow:

| Crate | Behaviour |
|---|---|
| `ordstack` | The program does not compile. |
| `bumpalo` | Allowed. The contents are copied to a fresh allocation; the old bytes are not reused until the arena is reset. |
| `bump-scope` | Allowed. Its documentation says growth then needs a new allocation and the old block becomes wasted space. Separately, while an allocator is claimed (`claim`), it is replaced by a dummy that "errors when trying to allocate or grow memory", a run-time failure. |
| `dyn-stack` | Does not arise for its own arrays: a `DynArray` is created with a fixed length. |

These rows were checked against the bumpalo 3.21 source and the bump-scope
2.4 and dyn-stack 0.13 documentation. Those crates are more flexible and far
more mature. Use `ordstack` when you want the discipline checked at compile
time.

## Status

0.1, experimental, not yet published. Unix only; developed and tested on
Linux, and intended to work on macOS. Safety is argued in
[SOUNDNESS.md](SOUNDNESS.md) and tested with compile-fail tests, Miri under
Stacked Borrows and Tree Borrows, AddressSanitizer, a shadow-model property
test and a fuzz target (`scripts/check.sh` runs all of them); it is not
mechanically proved. Expect breaking changes before 1.0.

Minimum Rust version: 1.85.

## License

MIT or Apache-2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and
[LICENSE-APACHE](LICENSE-APACHE).
