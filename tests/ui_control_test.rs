mod common;

use rstest::rstest;

use aulua::ui_control::{apply_ui_blocks, parse_ui_blocks};

use common::read_fixture;

#[rstest]
#[case::select_1("ui_control_select_1_in.anm2", "ui_control_select_1_out.anm2")]
#[case::select_2("ui_control_select_2_in.anm2", "ui_control_select_2_out.anm2")]
#[case::select_3("ui_control_select_3_in.anm2", "ui_control_select_3_out.anm2")]
#[case::select_4("ui_control_select_4_in.anm2", "ui_control_select_4_out.anm2")]
#[case::select_5("ui_control_select_5_in.anm2", "ui_control_select_5_out.anm2")]
#[case::select_6("ui_control_select_6_in.anm2", "ui_control_select_6_out.anm2")]
#[case::track_1("ui_control_track_1_in.anm2", "ui_control_track_1_out.anm2")]
#[case::track_2("ui_control_track_2_in.anm2", "ui_control_track_2_out.anm2")]
#[case::track_3("ui_control_track_3_in.anm2", "ui_control_track_3_out.anm2")]
#[case::check_1("ui_control_check_1_in.anm2", "ui_control_check_1_out.anm2")]
#[case::color_1("ui_control_color_1_in.anm2", "ui_control_color_1_out.anm2")]
#[case::file_1("ui_control_file_1_in.anm2", "ui_control_file_1_out.anm2")]
#[case::font_1("ui_control_font_1_in.anm2", "ui_control_font_1_out.anm2")]
#[case::figure_1("ui_control_figure_1_in.anm2", "ui_control_figure_1_out.anm2")]
#[case::value_num_1("ui_control_value_num_1_in.anm2", "ui_control_value_num_1_out.anm2")]
#[case::value_num_2("ui_control_value_num_2_in.anm2", "ui_control_value_num_2_out.anm2")]
#[case::value_text_1("ui_control_value_text_1_in.anm2", "ui_control_value_text_1_out.anm2")]
#[case::value_table_1(
    "ui_control_value_table_1_in.anm2",
    "ui_control_value_table_1_out.anm2"
)]
fn test_parse_apply(#[case] input_file: &str, #[case] expected_file: &str) {
    let input = read_fixture(input_file);
    let expected = read_fixture(expected_file);
    let blocks = parse_ui_blocks(&input);
    let output = apply_ui_blocks(&input, &blocks);
    assert_eq!(output, expected);
}
