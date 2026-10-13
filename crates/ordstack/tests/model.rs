#[path = "common/model.rs"]
mod model;

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig {
        cases: if cfg!(miri) { 16 } else { 2000 },
        failure_persistence: None,
        ..ProptestConfig::default()
    })]
    #[test]
    fn matches_the_shadow_model(bytes in proptest::collection::vec(any::<u8>(), 0..600)) {
        model::run(&bytes);
    }
}

#[test]
fn fixed_scripts() {
    model::run(&[]);
    // push x3, descend, push x2, freeze, push, ascend, truncate 1, insert 0,
    // remove 0, retain 3, reserve 9, shrink, pop, freeze, push, descend,
    // freeze, freeze
    model::run(&[
        0, 1, 2, 11, 0, 0, 13, 0, 12, 7, 1, 8, 0, 9, 0, 10, 3, 14, 9, 15, 6, 13, 0, 11, 13, 13,
    ]);
    // deep descent past MAX_DEPTH with a push at each level
    let deep: Vec<u8> = std::iter::repeat_n([0u8, 11], 10).flatten().collect();
    model::run(&deep);
}

#[test]
fn fixed_script_restart_at_depth_zero() {
    // push x3, op 12 drops them and starts afresh, push x2, pop, then a
    // freeze and a second restart with frozen blocks alive.
    model::run(&[0, 1, 2, 12, 3, 4, 6, 13, 0, 12, 5, 12, 12]);
}

#[test]
fn fixed_script_swap_remove() {
    // push x5, swap_remove 1, swap_remove 200 (index 0 of 4), clear, then
    // op 16 on an empty vector, which reads no argument (so the 5 is a push),
    // swap_remove 0, push, swap_remove 0.
    model::run(&[0, 0, 0, 0, 0, 16, 1, 16, 200, 17, 16, 5, 16, 0, 0, 16, 0]);
    // swap_remove of the last index with length > 1: 3 % 4 == 3.
    model::run(&[0, 0, 0, 0, 16, 3, 16, 1, 16, 0]);
}

#[test]
fn fixed_script_clear() {
    model::run(&[0, 1, 2, 17, 3, 17, 17, 4, 17]);
}

#[test]
fn fixed_script_extend() {
    // extend by 5, 0 and 7 values, with pushes and a pop between.
    model::run(&[
        18, 5, 0, 18, 0, 6, 18, 7, 18, 13, 18, 255, 18, 8, 18, 11, 18, 12,
    ]);
}

#[test]
fn fixed_script_freeze_into() {
    // At depth 0: push x3, freeze_into, push, freeze_into twice (the second
    // from empty). Then descend, push, freeze_into at depth 1, push x2, leave.
    model::run(&[
        0, 0, 0, 19, 2, 0, 19, 0, 19, 3, 0, 11, 0, 19, 1, 0, 0, 12, 0, 13,
    ]);
}

#[test]
fn fixed_script_sub_scope() {
    // sub scopes of 1 and 4 rounds, with elements below, at depth 0 and
    // inside a descended level, then an ordinary freeze.
    model::run(&[0, 0, 20, 0, 20, 3, 11, 0, 20, 1, 20, 255, 12, 13, 20, 2]);
}
