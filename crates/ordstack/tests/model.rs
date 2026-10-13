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
