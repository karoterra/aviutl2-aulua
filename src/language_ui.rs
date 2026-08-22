use thiserror::Error;

use crate::ui_control::{
    UiControlBlock, UiControlKind, UiControlMeta, parse_ui_blocks_at_line_start,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageScriptSyntax {
    AuluaSource,
    BuiltScript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUiName {
    pub original: String,
    pub translation_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageUiKind {
    Param,
    ParamCheck,
    ParamSelect,
    Track,
    Check,
    CheckSection,
    Color,
    File,
    Folder,
    Font,
    Figure,
    Select,
    Text,
    String,
    Value,
    Group,
    Separator,
}

impl LanguageUiKind {
    pub(crate) fn is_param(self) -> bool {
        matches!(self, Self::Param | Self::ParamCheck | Self::ParamSelect)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageUiMeta {
    None,
    Track { zero_label: Option<String> },
    Select { options: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUiItem {
    pub kind: LanguageUiKind,
    pub name: LanguageUiName,
    pub meta: LanguageUiMeta,
    pub span: SourceSpan,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LanguageUiExtractError {
    #[error("UI項目名が空です: {kind:?} line {line_number}")]
    EmptyUiName {
        kind: LanguageUiKind,
        line_number: usize,
    },
    #[error("UI項目名から変換した翻訳キーが空です: {kind:?} line {line_number}: {original_name}")]
    EmptyTranslationKey {
        kind: LanguageUiKind,
        original_name: String,
        line_number: usize,
    },
    #[error("selectの選択肢名が空です: line {line_number}, option {option_index}")]
    EmptySelectOptionName {
        line_number: usize,
        option_index: usize,
    },
    #[error("aulua source形式のUI記述が不正です: {kind:?} line {line_number}")]
    InvalidSourceSyntax {
        kind: LanguageUiKind,
        line_number: usize,
    },
    #[error("ビルド済みUI記述が不正です: {kind:?} line {line_number}")]
    InvalidBuiltSyntax {
        kind: LanguageUiKind,
        line_number: usize,
    },
}

pub fn extract_language_ui_items(
    body: &str,
    syntax: LanguageScriptSyntax,
) -> Result<Vec<LanguageUiItem>, LanguageUiExtractError> {
    match syntax {
        LanguageScriptSyntax::AuluaSource => extract_source_ui_items(body),
        LanguageScriptSyntax::BuiltScript => extract_built_ui_items(body),
    }
}

pub(crate) fn extract_tra2_language_ui_items(body: &str) -> Vec<LanguageUiItem> {
    physical_lines(body)
        .enumerate()
        .filter_map(|(line_index, line)| parse_tra2_param(line, line_index + 1))
        .collect()
}

fn parse_tra2_param(line: &str, line_number: usize) -> Option<LanguageUiItem> {
    let parameters = line.strip_prefix("--param:")?;
    let (declaration, initial_value) = parameters.split_once(',')?;

    if initial_value.is_empty() {
        return None;
    }

    let (kind, name, meta) = if let Some(name) = declaration.strip_suffix("/check") {
        (LanguageUiKind::ParamCheck, name, LanguageUiMeta::None)
    } else if let Some((name, options)) = declaration.split_once("/select/") {
        let options = parse_tra2_select_options(options)?;
        (
            LanguageUiKind::ParamSelect,
            name,
            LanguageUiMeta::Select { options },
        )
    } else if declaration.contains('/') {
        return None;
    } else {
        (LanguageUiKind::Param, declaration, LanguageUiMeta::None)
    };

    if name.is_empty() || name.contains('/') {
        return None;
    }

    Some(LanguageUiItem {
        kind,
        name: LanguageUiName {
            original: name.to_string(),
            translation_key: name.to_string(),
        },
        meta,
        span: SourceSpan {
            start_line: line_number,
            end_line: line_number,
        },
    })
}

fn parse_tra2_select_options(options: &str) -> Option<Vec<String>> {
    options
        .split('/')
        .map(|option| {
            let (name, _) = option.split_once('=')?;
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect()
}

fn extract_source_ui_items(body: &str) -> Result<Vec<LanguageUiItem>, LanguageUiExtractError> {
    let blocks = parse_ui_blocks_at_line_start(body);
    let mut items = Vec::new();

    for (line_index, line) in physical_lines(body).enumerate() {
        let line_number = line_index + 1;

        if let Some(item) = parse_group_or_separator(line, line_number)? {
            items.push(item);
            continue;
        }

        let Some(kind) = source_directive_kind(line) else {
            continue;
        };
        let Some(header) = source_directive_header(line, kind) else {
            return Err(LanguageUiExtractError::InvalidSourceSyntax { kind, line_number });
        };

        if source_ui_name(header).is_empty() {
            return Err(LanguageUiExtractError::EmptyUiName { kind, line_number });
        }

        let Some(block) = blocks.iter().find(|block| block.start_line == line_index) else {
            return Err(LanguageUiExtractError::InvalidSourceSyntax { kind, line_number });
        };
        items.push(language_item_from_source_block(block)?);
    }

    Ok(items)
}

fn extract_built_ui_items(body: &str) -> Result<Vec<LanguageUiItem>, LanguageUiExtractError> {
    let mut items = Vec::new();

    for (line_index, line) in physical_lines(body).enumerate() {
        let line_number = line_index + 1;

        if let Some(item) = parse_group_or_separator(line, line_number)? {
            items.push(item);
            continue;
        }

        if is_ignored_built_ui(line) {
            continue;
        }

        let Some(kind) = built_directive_kind(line) else {
            continue;
        };
        items.push(parse_built_ui_item(line, kind, line_number)?);
    }

    Ok(items)
}

pub(crate) fn physical_lines(body: &str) -> impl Iterator<Item = &str> {
    body.split_inclusive('\n').map(|line| {
        if let Some(line_without_lf) = line.strip_suffix('\n') {
            line_without_lf
                .strip_suffix('\r')
                .unwrap_or(line_without_lf)
        } else {
            line
        }
    })
}

fn source_directive_kind(line: &str) -> Option<LanguageUiKind> {
    let rest = line.strip_prefix("---$")?;
    let name = rest
        .split(|ch: char| ch == ':' || ch.is_whitespace())
        .next()?;
    language_kind_from_name(name)
}

fn source_directive_header(line: &str, kind: LanguageUiKind) -> Option<&str> {
    line.strip_prefix("---$")?
        .strip_prefix(language_kind_name(kind))?
        .strip_prefix(':')
}

fn source_ui_name(header: &str) -> &str {
    header.split(',').next().unwrap_or_default().trim()
}

fn language_item_from_source_block(
    block: &UiControlBlock,
) -> Result<LanguageUiItem, LanguageUiExtractError> {
    let kind = language_kind_from_ui_control(&block.kind);
    let line_number = block.start_line + 1;
    let meta = match &block.meta {
        Some(UiControlMeta::Track { zero_label, .. }) => LanguageUiMeta::Track {
            zero_label: zero_label.clone(),
        },
        Some(UiControlMeta::Select(options)) => {
            let mut names = Vec::with_capacity(options.len());
            for (option_index, (name, _)) in options.iter().enumerate() {
                if name.is_empty() {
                    return Err(LanguageUiExtractError::EmptySelectOptionName {
                        line_number,
                        option_index,
                    });
                }
                names.push(name.clone());
            }
            LanguageUiMeta::Select { options: names }
        }
        _ => LanguageUiMeta::None,
    };

    Ok(LanguageUiItem {
        kind,
        name: resolve_language_ui_name(&block.label, kind, line_number)?,
        meta,
        span: SourceSpan {
            start_line: line_number,
            end_line: block.end_line + 1,
        },
    })
}

fn parse_built_ui_item(
    line: &str,
    kind: LanguageUiKind,
    line_number: usize,
) -> Result<LanguageUiItem, LanguageUiExtractError> {
    let prefix = format!("--{}@", language_kind_name(kind));
    let rest = line
        .strip_prefix(&prefix)
        .ok_or(LanguageUiExtractError::InvalidBuiltSyntax { kind, line_number })?;
    let (variable_name, parameters) = rest
        .split_once(':')
        .ok_or(LanguageUiExtractError::InvalidBuiltSyntax { kind, line_number })?;
    if variable_name.is_empty() {
        return Err(LanguageUiExtractError::InvalidBuiltSyntax { kind, line_number });
    }

    let (original_name, meta) = match kind {
        LanguageUiKind::Track => parse_built_track(parameters, kind, line_number)?,
        LanguageUiKind::Select => parse_built_select(parameters, line_number)?,
        LanguageUiKind::File | LanguageUiKind::Folder => (parameters, LanguageUiMeta::None),
        LanguageUiKind::Check
        | LanguageUiKind::CheckSection
        | LanguageUiKind::Color
        | LanguageUiKind::Font
        | LanguageUiKind::Figure
        | LanguageUiKind::Text
        | LanguageUiKind::String
        | LanguageUiKind::Value => {
            let (name, _) = parameters
                .split_once(',')
                .ok_or(LanguageUiExtractError::InvalidBuiltSyntax { kind, line_number })?;
            (name, LanguageUiMeta::None)
        }
        LanguageUiKind::Param
        | LanguageUiKind::ParamCheck
        | LanguageUiKind::ParamSelect
        | LanguageUiKind::Group
        | LanguageUiKind::Separator => unreachable!(),
    };

    Ok(LanguageUiItem {
        kind,
        name: resolve_language_ui_name(original_name, kind, line_number)?,
        meta,
        span: SourceSpan {
            start_line: line_number,
            end_line: line_number,
        },
    })
}

fn parse_built_track(
    parameters: &str,
    kind: LanguageUiKind,
    line_number: usize,
) -> Result<(&str, LanguageUiMeta), LanguageUiExtractError> {
    let parts = parameters.split(',').collect::<Vec<_>>();
    if parts.len() < 4 {
        return Err(LanguageUiExtractError::InvalidBuiltSyntax { kind, line_number });
    }
    let zero_label = parts
        .get(5)
        .filter(|value| !value.is_empty())
        .map(|value| (*value).to_string());

    Ok((parts[0], LanguageUiMeta::Track { zero_label }))
}

fn parse_built_select(
    parameters: &str,
    line_number: usize,
) -> Result<(&str, LanguageUiMeta), LanguageUiExtractError> {
    let mut parts = parameters.split(',');
    let name_and_default = parts.next().unwrap_or_default();
    let name = name_and_default
        .split_once('=')
        .map(|(name, _)| name)
        .unwrap_or(name_and_default);
    let mut options = Vec::new();

    for (option_index, option) in parts.enumerate() {
        let (name, _) =
            option
                .split_once('=')
                .ok_or(LanguageUiExtractError::InvalidBuiltSyntax {
                    kind: LanguageUiKind::Select,
                    line_number,
                })?;
        if name.is_empty() {
            return Err(LanguageUiExtractError::EmptySelectOptionName {
                line_number,
                option_index,
            });
        }
        options.push(name.to_string());
    }

    Ok((name, LanguageUiMeta::Select { options }))
}

fn parse_group_or_separator(
    line: &str,
    line_number: usize,
) -> Result<Option<LanguageUiItem>, LanguageUiExtractError> {
    if line == "--group" {
        return Ok(None);
    }

    let (kind, original_name) = if let Some(parameters) = line.strip_prefix("--group:") {
        (
            LanguageUiKind::Group,
            parameters.split(',').next().unwrap_or_default(),
        )
    } else if let Some(name) = line.strip_prefix("--separator:") {
        (LanguageUiKind::Separator, name)
    } else {
        let identifier = built_directive_identifier(line);
        if matches!(identifier, Some("group" | "separator")) {
            return Err(LanguageUiExtractError::InvalidBuiltSyntax {
                kind: if identifier == Some("group") {
                    LanguageUiKind::Group
                } else {
                    LanguageUiKind::Separator
                },
                line_number,
            });
        }
        return Ok(None);
    };

    Ok(Some(LanguageUiItem {
        kind,
        name: resolve_language_ui_name(original_name, kind, line_number)?,
        meta: LanguageUiMeta::None,
        span: SourceSpan {
            start_line: line_number,
            end_line: line_number,
        },
    }))
}

fn resolve_language_ui_name(
    original_name: &str,
    kind: LanguageUiKind,
    line_number: usize,
) -> Result<LanguageUiName, LanguageUiExtractError> {
    if original_name.is_empty() {
        return Err(LanguageUiExtractError::EmptyUiName { kind, line_number });
    }
    let translation_key = original_name
        .rsplit_once("::")
        .map(|(_, suffix)| suffix)
        .unwrap_or(original_name);
    if translation_key.is_empty() {
        return Err(LanguageUiExtractError::EmptyTranslationKey {
            kind,
            original_name: original_name.to_string(),
            line_number,
        });
    }

    Ok(LanguageUiName {
        original: original_name.to_string(),
        translation_key: translation_key.to_string(),
    })
}

fn built_directive_kind(line: &str) -> Option<LanguageUiKind> {
    let identifier = built_directive_identifier(line)?;
    let kind = language_kind_from_name(identifier)?;
    (!matches!(kind, LanguageUiKind::Group | LanguageUiKind::Separator)).then_some(kind)
}

fn built_directive_identifier(line: &str) -> Option<&str> {
    line.strip_prefix("--")?
        .split(|ch: char| ch == '@' || ch == ':' || ch == ',' || ch.is_whitespace())
        .next()
}

fn is_ignored_built_ui(line: &str) -> bool {
    let Some(identifier) = built_directive_identifier(line) else {
        return false;
    };

    matches!(identifier, "trackgroup" | "dialog" | "track0" | "check0")
        || line.starts_with("--color:")
        || line.starts_with("--file:")
}

fn language_kind_from_name(name: &str) -> Option<LanguageUiKind> {
    match name {
        "track" => Some(LanguageUiKind::Track),
        "check" => Some(LanguageUiKind::Check),
        "checksection" => Some(LanguageUiKind::CheckSection),
        "color" => Some(LanguageUiKind::Color),
        "file" => Some(LanguageUiKind::File),
        "folder" => Some(LanguageUiKind::Folder),
        "font" => Some(LanguageUiKind::Font),
        "figure" => Some(LanguageUiKind::Figure),
        "select" => Some(LanguageUiKind::Select),
        "text" => Some(LanguageUiKind::Text),
        "string" => Some(LanguageUiKind::String),
        "value" => Some(LanguageUiKind::Value),
        "group" => Some(LanguageUiKind::Group),
        "separator" => Some(LanguageUiKind::Separator),
        _ => None,
    }
}

fn language_kind_name(kind: LanguageUiKind) -> &'static str {
    match kind {
        LanguageUiKind::Param => "param",
        LanguageUiKind::ParamCheck => "param/check",
        LanguageUiKind::ParamSelect => "param/select",
        LanguageUiKind::Track => "track",
        LanguageUiKind::Check => "check",
        LanguageUiKind::CheckSection => "checksection",
        LanguageUiKind::Color => "color",
        LanguageUiKind::File => "file",
        LanguageUiKind::Folder => "folder",
        LanguageUiKind::Font => "font",
        LanguageUiKind::Figure => "figure",
        LanguageUiKind::Select => "select",
        LanguageUiKind::Text => "text",
        LanguageUiKind::String => "string",
        LanguageUiKind::Value => "value",
        LanguageUiKind::Group => "group",
        LanguageUiKind::Separator => "separator",
    }
}

fn language_kind_from_ui_control(kind: &UiControlKind) -> LanguageUiKind {
    match kind {
        UiControlKind::Track => LanguageUiKind::Track,
        UiControlKind::Check => LanguageUiKind::Check,
        UiControlKind::CheckSection => LanguageUiKind::CheckSection,
        UiControlKind::Color => LanguageUiKind::Color,
        UiControlKind::File => LanguageUiKind::File,
        UiControlKind::Folder => LanguageUiKind::Folder,
        UiControlKind::Font => LanguageUiKind::Font,
        UiControlKind::Figure => LanguageUiKind::Figure,
        UiControlKind::Select => LanguageUiKind::Select,
        UiControlKind::Text => LanguageUiKind::Text,
        UiControlKind::String => LanguageUiKind::String,
        UiControlKind::Value => LanguageUiKind::Value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_source(body: &str) -> Result<Vec<LanguageUiItem>, LanguageUiExtractError> {
        extract_language_ui_items(body, LanguageScriptSyntax::AuluaSource)
    }

    fn extract_built(body: &str) -> Result<Vec<LanguageUiItem>, LanguageUiExtractError> {
        extract_language_ui_items(body, LanguageScriptSyntax::BuiltScript)
    }

    fn extract_tra2(body: &str) -> Vec<LanguageUiItem> {
        extract_tra2_language_ui_items(body)
    }

    #[test]
    fn extracts_supported_tra2_params_with_exact_translation_keys() {
        let body = concat!(
            "--param:周期,0.5\n",
            "--param:aaa::周期,0.5\n",
            "--param:aaa::加速/check,0\n",
            "--param:aaa::種類/select/直線=1/曲線=2/aaa::直線=3,1\n",
        );

        let items = extract_tra2(body);

        assert_eq!(items.len(), 4);
        assert_eq!(items[0].name.original, "周期");
        assert_eq!(items[0].name.translation_key, "周期");
        assert_eq!(items[1].name.original, "aaa::周期");
        assert_eq!(items[1].name.translation_key, "aaa::周期");
        assert_eq!(items[2].kind, LanguageUiKind::ParamCheck);
        assert_eq!(items[2].name.original, "aaa::加速");
        assert_eq!(items[2].name.translation_key, "aaa::加速");
        assert_eq!(items[3].kind, LanguageUiKind::ParamSelect);
        assert_eq!(items[3].name.original, "aaa::種類");
        assert_eq!(items[3].name.translation_key, "aaa::種類");
        assert_eq!(
            items[3].meta,
            LanguageUiMeta::Select {
                options: vec![
                    "直線".to_string(),
                    "曲線".to_string(),
                    "aaa::直線".to_string(),
                ]
            }
        );
        assert_eq!(
            items.iter().map(|item| item.span).collect::<Vec<_>>(),
            vec![
                SourceSpan {
                    start_line: 1,
                    end_line: 1,
                },
                SourceSpan {
                    start_line: 2,
                    end_line: 2,
                },
                SourceSpan {
                    start_line: 3,
                    end_line: 3,
                },
                SourceSpan {
                    start_line: 4,
                    end_line: 4,
                },
            ]
        );
    }

    #[test]
    fn tra2_select_ignores_value_semantics_and_preserves_duplicate_option_names() {
        let items = extract_tra2("--param:種類/select/同じ=not-a-number/同じ=/複数=x=y,-1\n");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, LanguageUiKind::ParamSelect);
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Select {
                options: vec!["同じ".to_string(), "同じ".to_string(), "複数".to_string()]
            }
        );
    }

    #[test]
    fn ignores_unsupported_invalid_and_standard_ui_in_tra2() {
        let body = concat!(
            "--param:0.5\n",
            "--param:,0.5\n",
            "--param:周期,\n",
            "--param:/check,1\n",
            "--param:名前/不明/check,1\n",
            "--param:種類/select,0\n",
            "--param:種類/select/,0\n",
            "--param:種類/select/A=0/B,0\n",
            "--param:種類/select/=0/B=1,0\n",
            "--param:種類/select/A=0/B=1,\n",
            "--param:未知/other,1\n",
            " --param:インデント,1\n",
            "--Param:大文字,1\n",
            "--track@vx:X速度,-10,10,0\n",
            "---$track:Y速度\n",
            "local y = 0\n",
            "--group:Group\n",
            "--separator:Separator\n",
        );

        assert!(extract_tra2(body).is_empty());
    }

    #[test]
    fn standard_built_ui_still_extracts_track_and_ignores_param() {
        let items = extract_built("--param:周期,0.5\n--track@vx:X速度,-10,10,0\n").unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, LanguageUiKind::Track);
        assert_eq!(items[0].name.translation_key, "X速度");
    }

    #[test]
    fn extracts_all_source_ui_kinds_in_order() {
        let body = concat!(
            "---$track:position::X, min=-10, max=10, zero_label=Zero\n",
            "local x = 0\n",
            "---$track:Y\n",
            "local y = 0\n",
            "---$check:Check\n",
            "local check = false\n",
            "---$checksection:Section\n",
            "local section = false\n",
            "---$color:Color\n",
            "local color = 0xffffff\n",
            "---$file:File\n",
            "local file = \"\"\n",
            "---$folder:Folder\n",
            "local folder = \"\"\n",
            "---$font:Font\n",
            "local font = \"Arial\"\n",
            "---$figure:Figure\n",
            "local figure = \"circle\"\n",
            "---$select:Select\n",
            "---First=1\n",
            "---Second=2\n",
            "local select = 1\n",
            "---$text:Text\n",
            "local text = \"\"\n",
            "---$string:String\n",
            "local string = \"\"\n",
            "---$value:Value\n",
            "local value = 0\n",
            "--group:Group\n",
            "--group:Nested Group,false\n",
            "--group\n",
            "--separator:Separator\n",
        );

        let items = extract_source(body).unwrap();

        assert_eq!(
            items.iter().map(|item| item.kind).collect::<Vec<_>>(),
            vec![
                LanguageUiKind::Track,
                LanguageUiKind::Track,
                LanguageUiKind::Check,
                LanguageUiKind::CheckSection,
                LanguageUiKind::Color,
                LanguageUiKind::File,
                LanguageUiKind::Folder,
                LanguageUiKind::Font,
                LanguageUiKind::Figure,
                LanguageUiKind::Select,
                LanguageUiKind::Text,
                LanguageUiKind::String,
                LanguageUiKind::Value,
                LanguageUiKind::Group,
                LanguageUiKind::Group,
                LanguageUiKind::Separator,
            ]
        );
        assert_eq!(items[0].name.original, "position::X");
        assert_eq!(items[0].name.translation_key, "X");
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Track {
                zero_label: Some("Zero".to_string())
            }
        );
        assert_eq!(items[1].meta, LanguageUiMeta::Track { zero_label: None });
        assert_eq!(
            items[9].meta,
            LanguageUiMeta::Select {
                options: vec!["First".to_string(), "Second".to_string()]
            }
        );
        assert_eq!(
            items[0].span,
            SourceSpan {
                start_line: 1,
                end_line: 2
            }
        );
        assert_eq!(
            items[9].span,
            SourceSpan {
                start_line: 19,
                end_line: 22
            }
        );
        assert_eq!(items[13].name.original, "Group");
        assert_eq!(items[14].name.original, "Nested Group");
        assert_eq!(items[15].name.original, "Separator");
        assert_eq!(
            items[15].span,
            SourceSpan {
                start_line: 32,
                end_line: 32
            }
        );
    }

    #[test]
    fn source_ui_requires_physical_line_start() {
        let body = " ---$track:X\nlocal x = 0\n\t--group:Group\n --separator:Separator\n";

        assert!(extract_source(body).unwrap().is_empty());
    }

    #[test]
    fn indented_incomplete_source_ui_does_not_consume_following_valid_ui() {
        let body = " ---$track:Ignored\n---$check:Valid\nlocal valid = false\n";

        let items = extract_source(body).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, LanguageUiKind::Check);
        assert_eq!(items[0].name.original, "Valid");
        assert_eq!(
            items[0].span,
            SourceSpan {
                start_line: 2,
                end_line: 4,
            }
        );
    }

    #[test]
    fn indented_multiline_like_source_ui_does_not_consume_following_valid_ui() {
        let body = concat!(
            " ---$select:Ignored\n",
            "---Ignored option=1\n",
            "local ignored = 1\n",
            "---$track:Valid\n",
            "---zero_label=Stop\n",
            "local valid = 0\n",
        );

        let items = extract_source(body).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, LanguageUiKind::Track);
        assert_eq!(items[0].name.original, "Valid");
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Track {
                zero_label: Some("Stop".to_string())
            }
        );
        assert_eq!(
            items[0].span,
            SourceSpan {
                start_line: 4,
                end_line: 7,
            }
        );
    }

    #[test]
    fn extracts_source_track_zero_label_from_option_line() {
        let body = "---$track:X\n---zero_label=Stop\nlocal x = 0\n";

        let items = extract_source(body).unwrap();

        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Track {
                zero_label: Some("Stop".to_string())
            }
        );
        assert_eq!(
            items[0].span,
            SourceSpan {
                start_line: 1,
                end_line: 4
            }
        );
    }

    #[test]
    fn invalid_source_ui_is_an_error() {
        assert_eq!(
            extract_source("---$track:X\nnot an assignment\n").unwrap_err(),
            LanguageUiExtractError::InvalidSourceSyntax {
                kind: LanguageUiKind::Track,
                line_number: 1,
            }
        );
        assert_eq!(
            extract_source("---$check\n").unwrap_err(),
            LanguageUiExtractError::InvalidSourceSyntax {
                kind: LanguageUiKind::Check,
                line_number: 1,
            }
        );
    }

    #[test]
    fn empty_source_ui_name_and_select_option_are_errors() {
        assert_eq!(
            extract_source("---$track:, min=-1, max=1\nlocal x = 0\n").unwrap_err(),
            LanguageUiExtractError::EmptyUiName {
                kind: LanguageUiKind::Track,
                line_number: 1,
            }
        );
        assert_eq!(
            extract_source("---$select:Select\n---=1\nlocal value = 1\n").unwrap_err(),
            LanguageUiExtractError::EmptySelectOptionName {
                line_number: 1,
                option_index: 0,
            }
        );
    }

    #[test]
    fn extracts_all_built_ui_kinds_in_order() {
        let body = concat!(
            "--track@x:X,-10,10,0,1,Zero,0.1\n",
            "--check@check:Check,false\n",
            "--checksection@section:Section,false,true\n",
            "--color@color:Color,0xffffff\n",
            "--file@file:File\n",
            "--folder@folder:Folder\n",
            "--font@font:Font,Arial\n",
            "--figure@figure:Figure,circle\n",
            "--select@select:Select=1,First=1,Second=2\n",
            "--text@text:Text,default\n",
            "--string@string:String,default\n",
            "--value@value:Value,{0,0,0}\n",
            "--group:Group,true\n",
            "--separator:Separator\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(
            items.iter().map(|item| item.kind).collect::<Vec<_>>(),
            vec![
                LanguageUiKind::Track,
                LanguageUiKind::Check,
                LanguageUiKind::CheckSection,
                LanguageUiKind::Color,
                LanguageUiKind::File,
                LanguageUiKind::Folder,
                LanguageUiKind::Font,
                LanguageUiKind::Figure,
                LanguageUiKind::Select,
                LanguageUiKind::Text,
                LanguageUiKind::String,
                LanguageUiKind::Value,
                LanguageUiKind::Group,
                LanguageUiKind::Separator,
            ]
        );
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Track {
                zero_label: Some("Zero".to_string())
            }
        );
        assert_eq!(
            items[8].meta,
            LanguageUiMeta::Select {
                options: vec!["First".to_string(), "Second".to_string()]
            }
        );
        for (index, item) in items.iter().enumerate() {
            assert_eq!(item.span.start_line, index + 1);
            assert_eq!(item.span.end_line, index + 1);
        }
    }

    #[test]
    fn extracts_optional_track_parameters() {
        let body = concat!(
            "--track@x:X,-10,10,0\n",
            "--track@y:Y,-10,10,0,,Zero\n",
            "--track@z:Z,-10,10,0,1,,0.1\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(items[0].meta, LanguageUiMeta::Track { zero_label: None });
        assert_eq!(
            items[1].meta,
            LanguageUiMeta::Track {
                zero_label: Some("Zero".to_string())
            }
        );
        assert_eq!(items[2].meta, LanguageUiMeta::Track { zero_label: None });
    }

    #[test]
    fn extracts_select_with_and_without_default_value() {
        let body = concat!(
            "--select@first:First=2,A=1,B=2\n",
            "--select@second:Second,C=3,D=4\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(items[0].name.original, "First");
        assert_eq!(items[1].name.original, "Second");
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Select {
                options: vec!["A".to_string(), "B".to_string()]
            }
        );
        assert_eq!(
            items[1].meta,
            LanguageUiMeta::Select {
                options: vec!["C".to_string(), "D".to_string()]
            }
        );
    }

    #[test]
    fn group_terminator_is_not_an_item() {
        let items = extract_built("--group:First\n--group\n--group:Second,false\n").unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name.original, "First");
        assert_eq!(items[1].name.original, "Second");
    }

    #[test]
    fn handles_lf_crlf_and_missing_trailing_newline() {
        let body = "--file@a:A\r\n--folder@b:B\n--separator:C";

        let items = extract_built(body).unwrap();

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name.original, "A");
        assert_eq!(items[1].name.original, "B");
        assert_eq!(items[2].name.original, "C");
        assert_eq!(items[2].span.start_line, 3);
    }

    #[test]
    fn does_not_treat_lone_cr_as_a_line_ending() {
        let items = extract_built("--file@file:File\r").unwrap();

        assert_eq!(items[0].name.original, "File\r");
        assert_eq!(items[0].name.translation_key, "File\r");
    }

    #[test]
    fn built_ui_requires_physical_line_start_and_lowercase_name() {
        let body = concat!(
            " --track@x:X,-1,1,0\n",
            "\t--check@x:Check,false\n",
            "--Track@x:Upper,-1,1,0\n",
            "--file@file:File\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, LanguageUiKind::File);
    }

    #[test]
    fn built_ui_detection_does_not_consider_lua_syntax() {
        let body = concat!(
            "local value = [[\n",
            "--check@inside:Inside,false\n",
            "]]\n",
            "--[[\n",
            "--file@comment:Comment File\n",
            "]]\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name.original, "Inside");
        assert_eq!(items[1].name.original, "Comment File");
    }

    #[test]
    fn ignores_known_out_of_scope_ui_forms() {
        let body = concat!(
            "--trackgroup@x,y:X Y\n",
            "--dialog:Dialog\n",
            "--track0:X,-1,1,0\n",
            "--check0:Check,0\n",
            "--color:0xffffff\n",
            "--file:\n",
        );

        assert!(extract_built(body).unwrap().is_empty());
    }

    #[test]
    fn invalid_built_ui_syntax_is_an_error() {
        for (body, kind) in [
            ("--track\n", LanguageUiKind::Track),
            ("--track@x\n", LanguageUiKind::Track),
            ("--track@:X,-1,1,0\n", LanguageUiKind::Track),
            ("--check@x:Check\n", LanguageUiKind::Check),
            ("--separator\n", LanguageUiKind::Separator),
        ] {
            assert_eq!(
                extract_built(body).unwrap_err(),
                LanguageUiExtractError::InvalidBuiltSyntax {
                    kind,
                    line_number: 1,
                }
            );
        }
    }

    #[test]
    fn empty_built_ui_name_and_select_option_are_errors() {
        assert_eq!(
            extract_built("--file@file:\n").unwrap_err(),
            LanguageUiExtractError::EmptyUiName {
                kind: LanguageUiKind::File,
                line_number: 1,
            }
        );
        assert_eq!(
            extract_built("--select@select:Select,=1\n").unwrap_err(),
            LanguageUiExtractError::EmptySelectOptionName {
                line_number: 1,
                option_index: 0,
            }
        );
        assert_eq!(
            extract_built("--select@select:Select,invalid\n").unwrap_err(),
            LanguageUiExtractError::InvalidBuiltSyntax {
                kind: LanguageUiKind::Select,
                line_number: 1,
            }
        );
    }

    #[test]
    fn resolves_translation_key_from_last_namespace_separator() {
        let body = concat!(
            "--file@a:X\n",
            "--folder@b:position::X\n",
            "--font@c:transform::position::X,Arial\n",
            "--figure@d:::X,circle\n",
            "--group:layout::Group\n",
            "--separator:layout::Separator\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(
            items
                .iter()
                .map(|item| item.name.translation_key.as_str())
                .collect::<Vec<_>>(),
            vec!["X", "X", "X", "X", "Group", "Separator"]
        );
        assert_eq!(items[3].name.original, "::X");
    }

    #[test]
    fn empty_translation_key_is_an_error() {
        for original_name in ["X::", "::", "position::X::"] {
            let body = format!("--file@file:{original_name}\n");
            assert_eq!(
                extract_built(&body).unwrap_err(),
                LanguageUiExtractError::EmptyTranslationKey {
                    kind: LanguageUiKind::File,
                    original_name: original_name.to_string(),
                    line_number: 1,
                }
            );
        }
    }

    #[test]
    fn namespaces_are_not_applied_to_zero_label_or_select_options() {
        let body = concat!(
            "--track@x:position::X,-1,1,0,1,zero::Stop\n",
            "--select@s:select::Mode,option::First=1,option::Second=2\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(items[0].name.translation_key, "X");
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Track {
                zero_label: Some("zero::Stop".to_string())
            }
        );
        assert_eq!(items[1].name.translation_key, "Mode");
        assert_eq!(
            items[1].meta,
            LanguageUiMeta::Select {
                options: vec!["option::First".to_string(), "option::Second".to_string()]
            }
        );
    }

    #[test]
    fn duplicate_translation_keys_are_preserved_in_order() {
        let items = extract_built("--file@a:aaa::name\n--folder@b:bbb::name\n").unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name.original, "aaa::name");
        assert_eq!(items[1].name.original, "bbb::name");
        assert_eq!(items[0].name.translation_key, "name");
        assert_eq!(items[1].name.translation_key, "name");
    }

    #[test]
    fn built_names_are_not_trimmed() {
        let body = concat!(
            "--track@x: X ,-1,1,0,1, Zero \n",
            "--select@s: Select , Option =1\n",
        );

        let items = extract_built(body).unwrap();

        assert_eq!(items[0].name.original, " X ");
        assert_eq!(
            items[0].meta,
            LanguageUiMeta::Track {
                zero_label: Some(" Zero ".to_string())
            }
        );
        assert_eq!(items[1].name.original, " Select ");
        assert_eq!(
            items[1].meta,
            LanguageUiMeta::Select {
                options: vec![" Option ".to_string()]
            }
        );
    }
}
