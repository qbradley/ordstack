//! Compile-fail tests for every rejection the guarantee depends on.
//! Compiler messages differ between releases, so this runs only when asked:
//! `ORDSTACK_UI=1 cargo +1.85 test -p ordstack --all-features --test ui`.

#[test]
fn ui() {
    if std::env::var_os("ORDSTACK_UI").is_none() {
        eprintln!("ui tests skipped: set ORDSTACK_UI=1 and use the 1.85 toolchain");
        return;
    }
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
    t.compile_fail("tests/ui/fail/*.rs");
}
