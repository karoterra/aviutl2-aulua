use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiControlKind {
    Select,
    Track,
    Check,
    Color,
    File,
    Font,
    Figure,
    Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiControlMeta {
    Select(Vec<(String, String)>),
    Track {
        min: String,
        max: String,
        step: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiControlBlock {
    pub kind: UiControlKind,
    pub label: String,
    pub var_name: String,
    pub default_value: String,
    pub start_line: usize,
    pub end_line: usize,
    pub meta: Option<UiControlMeta>,
}

pub fn parse_ui_blocks(source: &str) -> Vec<UiControlBlock> {
    let mut blocks = Vec::new();
    let mut lines = source.split("\n").enumerate().peekable();

    while let Some((i, line)) = lines.next() {
        let line = line.trim();
        if let Some(label) = line.strip_prefix("---$select:") {
            if let Some(block) = parse_select_block(i, label, &mut lines) {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$track:") {
            if let Some(block) = parse_track_block(i, label, &mut lines) {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$check:") {
            if let Some(block) = parse_ui_block_no_meta(UiControlKind::Check, i, label, &mut lines)
            {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$color:") {
            if let Some(block) = parse_ui_block_no_meta(UiControlKind::Color, i, label, &mut lines)
            {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$file:") {
            if let Some(block) = parse_file_block(i, label, &mut lines) {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$font:") {
            if let Some(block) = parse_ui_block_no_meta(UiControlKind::Font, i, label, &mut lines) {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$figure:") {
            if let Some(block) = parse_ui_block_no_meta(UiControlKind::Figure, i, label, &mut lines)
            {
                blocks.push(block);
            }
        } else if let Some(label) = line.strip_prefix("---$value:") {
            if let Some(block) = parse_ui_block_no_meta(UiControlKind::Value, i, label, &mut lines)
            {
                blocks.push(block);
            }
        }
    }
    blocks
}

fn parse_assignment<'a, I>(
    line: &'a str,
    lines: &mut Peekable<I>,
) -> Option<(String, String, usize)>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if line.starts_with("local ") && line.contains('=') {
        let assignment = line.trim_start_matches("local ").trim();
        if let Some((name, value)) = assignment.split_once('=') {
            let var_name = name.trim().to_string();
            let default_value = value.trim().to_string();
            lines.next();
            let end_line: usize = if let Some(&(_, next_line)) = lines.peek() {
                if next_line.is_empty() { 1 } else { 0 }
            } else {
                0
            };
            return Some((var_name, default_value, end_line));
        }
    }
    None
}

fn parse_select_block<'a, I>(
    start_line: usize,
    label: &'a str,
    lines: &mut Peekable<I>,
) -> Option<UiControlBlock>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut options = Vec::new();

    while let Some(&(i, line)) = lines.peek() {
        let line = line.trim();
        if let Some(opt) = line.strip_prefix("---") {
            if let Some((name, value)) = opt.split_once('=') {
                options.push((name.trim().to_string(), value.trim().to_string()));
            }
            lines.next();
        } else if let Some((name, value, end_offset)) = parse_assignment(line, lines) {
            return Some(UiControlBlock {
                kind: UiControlKind::Select,
                label: label.trim().to_string(),
                var_name: name,
                default_value: value,
                start_line,
                end_line: i + end_offset,
                meta: Some(UiControlMeta::Select(options)),
            });
        } else {
            break;
        }
    }
    None
}

fn parse_track_block<'a, I>(
    start_line: usize,
    label: &'a str,
    lines: &mut Peekable<I>,
) -> Option<UiControlBlock>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut min_value = "0".to_string();
    let mut max_value = "100".to_string();
    let mut step = None;

    while let Some(&(i, line)) = lines.peek() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("---min=") {
            min_value = value.trim().to_string();
            lines.next();
        } else if let Some(value) = line.strip_prefix("---max=") {
            max_value = value.trim().to_string();
            lines.next();
        } else if let Some(value) = line.strip_prefix("---step=") {
            step = Some(value.trim().to_string());
            lines.next();
        } else if line.starts_with("--") {
            lines.next();
        } else if let Some((name, value, end_offset)) = parse_assignment(line, lines) {
            return Some(UiControlBlock {
                kind: UiControlKind::Track,
                label: label.trim().to_string(),
                var_name: name,
                default_value: value,
                start_line,
                end_line: i + end_offset,
                meta: Some(UiControlMeta::Track {
                    min: min_value,
                    max: max_value,
                    step,
                }),
            });
        } else {
            break;
        }
    }
    None
}

fn parse_ui_block_no_meta<'a, I>(
    kind: UiControlKind,
    start_line: usize,
    label: &'a str,
    lines: &mut Peekable<I>,
) -> Option<UiControlBlock>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    while let Some(&(i, line)) = lines.peek() {
        let line = line.trim();
        if line.starts_with("--") {
            lines.next();
        } else if let Some((name, value, end_offset)) = parse_assignment(line, lines) {
            return Some(UiControlBlock {
                kind,
                label: label.trim().to_string(),
                var_name: name,
                default_value: value,
                start_line,
                end_line: i + end_offset,
                meta: None,
            });
        }
    }
    None
}

fn parse_file_block<'a, I>(
    start_line: usize,
    label: &'a str,
    lines: &mut Peekable<I>,
) -> Option<UiControlBlock>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    if let Some(&(i, line)) = lines.peek() {
        let line = line.trim();
        if let Some((name, value, end_offset)) = parse_assignment(line, lines) {
            return Some(UiControlBlock {
                kind: UiControlKind::File,
                label: label.trim().to_string(),
                var_name: name,
                default_value: value,
                start_line,
                end_line: i + end_offset,
                meta: None,
            });
        }
    }
    None
}

pub fn apply_ui_blocks(source: &str, blocks: &[UiControlBlock]) -> String {
    let mut lines: Vec<String> = source.split("\n").map(|s| s.to_string()).collect();

    for block in blocks.iter().rev() {
        let result = match block.kind {
            UiControlKind::Select => {
                let mut line = format!(
                    "--select@{}:{}={}",
                    block.var_name, block.label, block.default_value
                );
                if let Some(UiControlMeta::Select(opts)) = &block.meta {
                    for (name, val) in opts {
                        line.push_str(&format!(",{name}={val}"));
                    }
                }
                line
            }
            UiControlKind::Track => {
                let mut line = format!("--track@{}:{}", block.var_name, block.label);
                if let Some(UiControlMeta::Track { min, max, step }) = &block.meta {
                    line.push_str(&format!(",{},{},{}", min, max, block.default_value));
                    if let Some(step) = step {
                        line.push_str(&format!(",{step}"));
                    }
                }
                line
            }
            UiControlKind::Check => {
                format!(
                    "--check@{}:{},{}",
                    block.var_name, block.label, block.default_value
                )
            }
            UiControlKind::Color => {
                format!(
                    "--color@{}:{},{}",
                    block.var_name, block.label, block.default_value
                )
            }
            UiControlKind::File => {
                format!("--file@{}:{}", block.var_name, block.label)
            }
            UiControlKind::Font => {
                format!(
                    "--font@{}:{},{}",
                    block.var_name,
                    block.label,
                    block.default_value.trim_matches('"')
                )
            }
            UiControlKind::Figure => {
                format!(
                    "--figure@{}:{},{}",
                    block.var_name,
                    block.label,
                    block.default_value.trim_matches('"')
                )
            }
            UiControlKind::Value => {
                format!(
                    "--value@{}:{},{}",
                    block.var_name, block.label, block.default_value
                )
            }
        };

        lines.splice(block.start_line..=block.end_line, [result]);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    use crate::{common::get_fixture_path, text_utils::read_text};

    #[test]
    fn test_parse_ui_blocks_select() {
        let src = r#"
---$select:目
---通常=0
---ちょい閉じ=1
---半目=2
local eye = 1
"#;
        let blocks = parse_ui_blocks(src);
        assert_eq!(blocks.len(), 1);

        let block = &blocks[0];
        assert_eq!(block.kind, UiControlKind::Select);
        assert_eq!(block.label, "目");
        assert_eq!(block.var_name, "eye");
        assert_eq!(block.default_value, "1");
        assert_eq!(block.start_line, 1);
        assert_eq!(block.end_line, 6);

        if let Some(UiControlMeta::Select(options)) = &block.meta {
            assert_eq!(options.len(), 3);
            assert_eq!(options[0], ("通常".to_string(), "0".to_string()));
            assert_eq!(options[1], ("ちょい閉じ".to_string(), "1".to_string()));
            assert_eq!(options[2], ("半目".to_string(), "2".to_string()));
        } else {
            panic!("Exptected Select");
        }
    }

    #[rstest]
    #[case("ui_control_select_1_in.anm2", 0, 4)]
    #[case("ui_control_select_2_in.anm2", 0, 5)]
    #[case("ui_control_select_3_in.anm2", 0, 5)]
    #[case("ui_control_select_4_in.anm2", 0, 4)]
    #[case("ui_control_select_5_in.anm2", 0, 5)]
    #[case("ui_control_select_6_in.anm2", 0, 5)]
    fn test_parse_ui_blocks_line_range(
        #[case] input_file: &str,
        #[case] start_line: usize,
        #[case] end_line: usize,
    ) {
        let input_path = get_fixture_path(input_file);
        let input = read_text(&input_path).unwrap();
        let blocks = parse_ui_blocks(&input);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_line, start_line);
        assert_eq!(blocks[0].end_line, end_line);
    }

    #[test]
    fn test_apply_ui_blocks_select() {
        let src = r#"
---$select:目
---通常=0
---ちょい閉じ=1
---半目=2
local eye = 1
"#;
        let blocks = parse_ui_blocks(src);
        let result = apply_ui_blocks(src, &blocks);

        let expected = "\n--select@eye:目=1,通常=0,ちょい閉じ=1,半目=2";
        assert_eq!(result, expected);
    }
}
