#[test]
fn create() {
    trycmd::TestCases::new()
        .case("tests/create/*.toml")
        .default_bin_name("sfs");
}
