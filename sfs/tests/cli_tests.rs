#[test]
fn create() {
    trycmd::TestCases::new()
        .case("tests/create/*.toml")
        .env("TRYCMD", "true")
        .default_bin_name("sfs");
}

#[test]
fn fold() {
    trycmd::TestCases::new()
        .case("tests/fold/*.toml")
        .default_bin_name("sfs");
}

#[test]
fn stat() {
    trycmd::TestCases::new()
        .case("tests/stat/*.toml")
        .default_bin_name("sfs");
}

#[test]
fn view() {
    trycmd::TestCases::new()
        .case("tests/view/*.toml")
        .default_bin_name("sfs");
}
