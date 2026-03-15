use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiControlKind {
    Select,
    Track,
    Check,
    CheckSection,
    Color,
    File,
    Folder,
    Font,
    Figure,
    Text,
    String,
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct TrackOptions {
    min: Option<String>,
    max: Option<String>,
    step: Option<String>,
}

enum TrackOptionLine<'a> {
    Min(&'a str),
    Max(&'a str),
    Step(&'a str),
}

pub fn parse_ui_blocks(source: &str) -> Vec<UiControlBlock> {
    let mut blocks = Vec::new();
    let mut lines = source.split("\n").enumerate().peekable();

    while let Some((i, line)) = lines.next() {
        let line = line.trim();
        if let Some(label) = line.strip_prefix("---$select:")
            && let Some(block) = parse_select_block(i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$track:")
            && let Some(block) = parse_track_block(i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$check:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Check, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$checksection:")
            && let Some(block) =
                parse_ui_block_no_meta(UiControlKind::CheckSection, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$color:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Color, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$file:")
            && let Some(block) = parse_file_block(i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$folder:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Folder, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$font:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Font, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$figure:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Figure, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$text:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Text, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$string:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::String, i, label, &mut lines)
        {
            blocks.push(block);
        } else if let Some(label) = line.strip_prefix("---$value:")
            && let Some(block) = parse_ui_block_no_meta(UiControlKind::Value, i, label, &mut lines)
        {
            blocks.push(block);
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

fn parse_track_header(input: &str) -> Option<(String, TrackOptions)> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    let mut parts = input.split(',').map(|s| s.trim());
    let label = parts.next()?.to_string();
    if label.is_empty() {
        return None;
    }

    let mut options = TrackOptions::default();

    for part in parts {
        let (key, value) = part.split_once('=')?;
        let key = key.trim();
        let value = value.trim();

        if value.is_empty() {
            return None;
        }

        match key {
            "min" => {
                if options.min.is_some() {
                    return None;
                }
                options.min = Some(value.to_string());
            }
            "max" => {
                if options.max.is_some() {
                    return None;
                }
                options.max = Some(value.to_string());
            }
            "step" => {
                if options.step.is_some() {
                    return None;
                }
                options.step = Some(value.to_string());
            }
            _ => return None,
        }
    }

    Some((label, options))
}

fn parse_track_option_line(line: &str) -> Option<TrackOptionLine<'_>> {
    let line = line.trim();

    line.strip_prefix("---min=")
        .map(|value| TrackOptionLine::Min(value.trim()))
        .or_else(|| {
            line.strip_prefix("---max=")
                .map(|value| TrackOptionLine::Max(value.trim()))
        })
        .or_else(|| {
            line.strip_prefix("---step=")
                .map(|value| TrackOptionLine::Step(value.trim()))
        })
}

fn apply_track_option_line(options: &mut TrackOptions, line: TrackOptionLine<'_>) -> Option<()> {
    match line {
        TrackOptionLine::Min(value) => {
            if options.min.is_some() {
                return None;
            }
            options.min = Some(value.to_string());
        }
        TrackOptionLine::Max(value) => {
            if options.max.is_some() {
                return None;
            }
            options.max = Some(value.to_string());
        }
        TrackOptionLine::Step(value) => {
            if options.step.is_some() {
                return None;
            }
            options.step = Some(value.to_string());
        }
    }
    Some(())
}

fn parse_track_block<'a, I>(
    start_line: usize,
    header: &'a str,
    lines: &mut Peekable<I>,
) -> Option<UiControlBlock>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let (label, mut options) = parse_track_header(header)?;

    while let Some(&(i, line)) = lines.peek() {
        let line = line.trim();

        if let Some(option_line) = parse_track_option_line(line) {
            apply_track_option_line(&mut options, option_line)?;
            lines.next();
        } else if line.starts_with("--") {
            lines.next();
        } else if let Some((name, value, end_offset)) = parse_assignment(line, lines) {
            return Some(UiControlBlock {
                kind: UiControlKind::Track,
                label,
                var_name: name,
                default_value: value,
                start_line,
                end_line: i + end_offset,
                meta: Some(UiControlMeta::Track {
                    min: options.min.unwrap_or_else(|| "0".to_string()),
                    max: options.max.unwrap_or_else(|| "100".to_string()),
                    step: options.step,
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
            UiControlKind::CheckSection => {
                format!(
                    "--checksection@{}:{},{}",
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
            UiControlKind::Folder => {
                format!("--folder@{}:{}", block.var_name, block.label)
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
            UiControlKind::Text => {
                format!(
                    "--text@{}:{},{}",
                    block.var_name,
                    block.label,
                    block.default_value.trim_matches('"')
                )
            }
            UiControlKind::String => {
                format!(
                    "--string@{}:{},{}",
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
