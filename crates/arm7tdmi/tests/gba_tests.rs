mod common;

use common::runner::{run_gba_test, RomKind};

#[test]
fn arm_gba_tests_pass() {
    let outcome = run_gba_test(RomKind::Arm).unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(outcome.result, 0);
}

#[test]
fn thumb_gba_tests_pass() {
    let outcome = run_gba_test(RomKind::Thumb).unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(outcome.result, 0);
}

#[test]
fn memory_gba_tests_pass() {
    let outcome = run_gba_test(RomKind::Memory).unwrap_or_else(|message| panic!("{message}"));
    assert_eq!(outcome.result, 0);
}
