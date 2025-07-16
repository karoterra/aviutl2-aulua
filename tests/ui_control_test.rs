use std::fs;
use std::path::PathBuf;

use rstest::rstest;

use aulua::ui_control::{apply_ui_blocks, parse_ui_blocks};

fn read_fixture(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(file);
    fs::read_to_string(&path).expect(&format!("failed to read: {:?}", &path))
}

#[rstest]
#[case("ui_control_select_1_in.anm2", "ui_control_select_1_out.anm2")]
#[case("ui_control_select_2_in.anm2", "ui_control_select_2_out.anm2")]
#[case("ui_control_select_3_in.anm2", "ui_control_select_3_out.anm2")]
#[case("ui_control_select_4_in.anm2", "ui_control_select_4_out.anm2")]
#[case("ui_control_select_5_in.anm2", "ui_control_select_5_out.anm2")]
#[case("ui_control_select_6_in.anm2", "ui_control_select_6_out.anm2")]
fn test_parse_apply(#[case] input_file: &str, #[case] expected_file: &str) {
    let input = read_fixture(input_file);
    let expected = read_fixture(expected_file);
    let blocks = parse_ui_blocks(&input);
    let output = apply_ui_blocks(&input, &blocks);
    assert_eq!(output, expected);
}
