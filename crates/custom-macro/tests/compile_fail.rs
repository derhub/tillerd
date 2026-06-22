#[test]
fn missing_error_code_attribute_fails_to_compile() {
    trybuild::TestCases::new().compile_fail("tests/ui/missing_attr.rs");
}
