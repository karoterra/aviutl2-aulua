use aulua::build::build_all;
use aulua::config_loader::load_config;
use std::fs;
use std::path::Path;

#[test]
fn test_basic_build() {
    let config = load_config("tests/fixtures/build_minimal.yaml").unwrap();
    let out_dir = Path::new("tests/temp_build");
    let output_file = out_dir.join("test_output.obj2");

    let _ = fs::remove_dir_all(out_dir);
    fs::create_dir_all(out_dir).unwrap();

    build_all(&config, out_dir).unwrap();

    let result = fs::read_to_string(output_file).unwrap();
    let expect = r#"@スクリプト1
print("hello from script1)

"#;
    assert_eq!(result, expect);
}
