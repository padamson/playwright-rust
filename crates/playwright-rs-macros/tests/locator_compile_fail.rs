// trybuild harness for compile-fail diagnostics. Each `tests/ui/*.rs`
// file is a snippet that should fail to compile, with the expected
// diagnostic captured in the matching `*.stderr`.
#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/empty.rs");
    t.compile_fail("tests/ui/whitespace_only.rs");
    t.compile_fail("tests/ui/unclosed_bracket.rs");
    t.compile_fail("tests/ui/unmatched_closer.rs");
    t.compile_fail("tests/ui/unknown_engine.rs");
}
