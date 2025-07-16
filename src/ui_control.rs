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
    Track { min: f32, max: f32, step: f32 },
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
            // select block
            let mut options = Vec::new();
            let mut default_value = None;
            let mut var_name = None;
            let start_line = i;
            let mut end_line = i;

            let label = label.trim().to_string();

            while let Some(&(j, next_line)) = lines.peek() {
                let next_line = next_line.trim();
                if let Some(opt) = next_line.strip_prefix("---") {
                    if let Some((name, value)) = opt.split_once('=') {
                        options.push((name.trim().to_string(), value.trim().to_string()));
                    }
                    lines.next();
                } else if next_line.starts_with("local ") && next_line.contains('=') {
                    let assignment = next_line.trim_start_matches("local ").trim();
                    if let Some((name, value)) = assignment.split_once('=') {
                        var_name = Some(name.trim().to_string());
                        default_value = Some(value.trim().to_string());
                        lines.next();
                        if let Some(&(k, line_after_block)) = lines.peek() {
                            if line_after_block.len() == 0 {
                                end_line = k;
                            } else {
                                end_line = j;
                            }
                        } else {
                            end_line = j;
                        }
                        break;
                    }
                } else {
                    break;
                }
            }

            if let (Some(var), Some(def)) = (var_name, default_value) {
                blocks.push(UiControlBlock {
                    kind: UiControlKind::Select,
                    label: label,
                    var_name: var,
                    default_value: def,
                    start_line: start_line,
                    end_line: end_line,
                    meta: Some(UiControlMeta::Select(options)),
                });
            }
        }
    }
    blocks
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
                        line.push_str(&format!(",{}={}", name, val));
                    }
                }
                line
            }
            _ => continue,
        };

        lines.splice(block.start_line..=block.end_line, [result]);
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use std::fs;
    use std::path::PathBuf;

    fn read_fixture(file: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(file);
        fs::read_to_string(&path).expect(&format!("failed to read: {:?}", &path))
    }

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
        let input = read_fixture(input_file);
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
