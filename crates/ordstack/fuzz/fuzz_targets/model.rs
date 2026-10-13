#![no_main]

// Run with `-max_len=4096`: the interpreter ignores bytes beyond that anyway.
#[path = "../../tests/common/model.rs"]
mod model;

libfuzzer_sys::fuzz_target!(|data: &[u8]| model::run(data));
