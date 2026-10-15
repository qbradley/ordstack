// CONTROL, not a port: `src/alloc_port.rs` on the system allocator
// (allocator-api2's `Global`), so that against `ordstack-alloc` only the
// allocator differs: same source, same container types and handle sizes.
#![forbid(unsafe_code)]
#[path = "../alloc_port.rs"]
mod port;

fn main() {
    port::main_in(&allocator_api2::alloc::Global);
}
