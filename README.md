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

The crate documentation's *Patterns* section shows several stacks,
children on the same stack, phases, and how to choose a reservation.

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

What happens when a container has to grow while a later allocation on the
same stack is still alive:

| Crate | Behaviour |
|---|---|
| `ordstack` | The program does not compile. Once the later allocation is dead, the container grows in place, reclaiming the space above it. |
| `bumpalo` | Allowed. Growing an allocation that is not the most recent one copies the contents to a fresh allocation; the old bytes are not reused until the arena is reset. |
| `bump-scope` | Allowed. Growing an allocation that is not the most recent one needs a new allocation, and the old block becomes wasted space. Separately, an allocator that has been claimed fails at run time if it is asked to allocate or grow. |
| `dyn-stack` | Does not arise for its own arrays, which are allocated with a length given up front. |

The other rows describe those crates as their source and documentation
stood at the time of writing; check their current documentation before
relying on them. Those crates are more flexible and far more mature. Use `ordstack` when you want the discipline checked at compile
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
