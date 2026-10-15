// `src/alloc_port.rs` on one ordstack frame, through `FrameAlloc` (feature
// `allocator-api2`). Lines marked BENCH are instrumentation.
#![forbid(unsafe_code)]
#[path = "../alloc_port.rs"]
mod port;

fn main() {
    let mut st = ordstack::Stack::new();
    let mut f = st.frame();
    port::main_in(&f.allocator());
    // BENCH{ stack statistics, read once nothing on the stack is alive.
    if std::env::var_os("ORDSTACK_STATS").is_some() {
        eprintln!(
            "stack data high_water={} extent={}",
            st.high_water(),
            st.extent()
        );
    }
    // BENCH}
}
