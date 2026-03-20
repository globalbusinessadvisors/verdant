#[test]
fn local_data_cannot_be_serialized() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/local_not_serializable.rs");
}
