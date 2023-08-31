#[test]
fn create() {
    trycmd::TestCases::new()
        .case("tests/create/*.toml")
        .env("SFS_ALLOW_STDIN", "true")
        .default_bin_name("sfs");
}

#[test]
fn fold() {
    trycmd::TestCases::new()
        .case("tests/fold/*.toml")
        .env("SFS_ALLOW_STDIN", "true")
        .default_bin_name("sfs");
}

#[test]
fn stat() {
    trycmd::TestCases::new()
        .case("tests/stat/*.toml")
        .env("SFS_ALLOW_STDIN", "true")
        .default_bin_name("sfs");
}

#[test]
fn view() {
    trycmd::TestCases::new()
        .case("tests/view/*.toml")
        .env("SFS_ALLOW_STDIN", "true")
        .default_bin_name("sfs");
}
