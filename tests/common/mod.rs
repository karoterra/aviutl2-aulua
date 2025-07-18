use std::fs;
use std::path::PathBuf;

pub fn read_fixture(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("failed to read: {:?}", &path))
}
