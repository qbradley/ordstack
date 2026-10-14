// The original program, compiled unchanged, with mimalloc as the global
// allocator (a fast-allocator baseline).
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

include!("../orig.rs");
