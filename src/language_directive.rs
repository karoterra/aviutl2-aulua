use thiserror::Error;

use crate::language_ui::{SourceSpan, physical_lines};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageDirectiveName {
    Tips,
    ScriptTips,
    Nolang,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDirective {
    pub kind: LanguageDirectiveKind,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageDirectiveKind {
    Tips { text: String },
    ScriptTips { text: String },
    Nolang { targets: Vec<NolangTarget> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NolangTarget {
    Name,
    ZeroLabel,
    Options,
    Option(String),
    ScriptName,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LanguageDirectiveExtractError {
    #[error("languageディレクティブにコロンがありません: {directive:?} line {line_number}")]
    MissingColon {
        directive: LanguageDirectiveName,
        line_number: usize,
    },
    #[error("Tips本文が空です: {directive:?} line {line_number}")]
    EmptyTips {
        directive: LanguageDirectiveName,
        line_number: usize,
    },
    #[error("nolangの対象が空です: line {line_number}, target {target_index}")]
    EmptyNolangTarget {
        line_number: usize,
        target_index: usize,
    },
    #[error("未知のnolang対象です: line {line_number}, target {target_index}: {target}")]
    UnknownNolangTarget {
        target: String,
        line_number: usize,
        target_index: usize,
    },
    #[error("nolangのoption対象名が空です: line {line_number}, target {target_index}")]
    EmptyNolangOptionName {
        line_number: usize,
        target_index: usize,
    },
}

pub fn extract_language_directives(
    body: &str,
) -> Result<Vec<LanguageDirective>, LanguageDirectiveExtractError> {
    let lines = physical_lines(body).collect::<Vec<_>>();
    let mut directives = Vec::new();
    let mut line_index = 0;

    while line_index < lines.len() {
        let line = lines[line_index];
        let line_number = line_index + 1;

        if let Some(text) = line.strip_prefix("---$tips:") {
            let (directive, next_line_index) =
                parse_tips_block(&lines, line_index, text, LanguageDirectiveName::Tips)?;
            directives.push(directive);
            line_index = next_line_index;
            continue;
        }
        if line == "---$tips" {
            return Err(LanguageDirectiveExtractError::MissingColon {
                directive: LanguageDirectiveName::Tips,
                line_number,
            });
        }

        if let Some(text) = line.strip_prefix("---$script_tips:") {
            let (directive, next_line_index) =
                parse_tips_block(&lines, line_index, text, LanguageDirectiveName::ScriptTips)?;
            directives.push(directive);
            line_index = next_line_index;
            continue;
        }
        if line == "---$script_tips" {
            return Err(LanguageDirectiveExtractError::MissingColon {
                directive: LanguageDirectiveName::ScriptTips,
                line_number,
            });
        }

        if let Some(targets) = line.strip_prefix("---$nolang:") {
            directives.push(LanguageDirective {
                kind: LanguageDirectiveKind::Nolang {
                    targets: parse_nolang_targets(targets, line_number)?,
                },
                span: SourceSpan {
                    start_line: line_number,
                    end_line: line_number,
                },
            });
        } else if line == "---$nolang" {
            return Err(LanguageDirectiveExtractError::MissingColon {
                directive: LanguageDirectiveName::Nolang,
                line_number,
            });
        }

        line_index += 1;
    }

    Ok(directives)
}

pub(crate) fn remove_language_directives(
    source: &str,
) -> Result<String, LanguageDirectiveExtractError> {
    let directives = extract_language_directives(source)?;
    if directives.is_empty() {
        return Ok(source.to_string());
    }

    let mut spans = directives.iter().map(|directive| directive.span).peekable();
    let mut output = String::with_capacity(source.len());

    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        let line_number = line_index + 1;
        while spans.peek().is_some_and(|span| span.end_line < line_number) {
            spans.next();
        }

        let should_remove = spans
            .peek()
            .is_some_and(|span| span.start_line <= line_number && line_number <= span.end_line);
        if !should_remove {
            output.push_str(line);
        }
    }

    Ok(output)
}

fn parse_tips_block(
    lines: &[&str],
    start_line_index: usize,
    first_text: &str,
    directive: LanguageDirectiveName,
) -> Result<(LanguageDirective, usize), LanguageDirectiveExtractError> {
    let mut text = first_text.to_string();
    let mut next_line_index = start_line_index + 1;

    while let Some(line) = lines.get(next_line_index) {
        let Some(continuation) = tips_continuation_text(line) else {
            break;
        };
        text.push('\n');
        text.push_str(continuation);
        next_line_index += 1;
    }

    let start_line = start_line_index + 1;
    if text.is_empty() {
        return Err(LanguageDirectiveExtractError::EmptyTips {
            directive,
            line_number: start_line,
        });
    }

    let kind = match directive {
        LanguageDirectiveName::Tips => LanguageDirectiveKind::Tips { text },
        LanguageDirectiveName::ScriptTips => LanguageDirectiveKind::ScriptTips { text },
        LanguageDirectiveName::Nolang => unreachable!(),
    };

    Ok((
        LanguageDirective {
            kind,
            span: SourceSpan {
                start_line,
                end_line: next_line_index,
            },
        },
        next_line_index,
    ))
}

fn tips_continuation_text(line: &str) -> Option<&str> {
    line.strip_prefix("---")?
        .trim_start_matches(' ')
        .strip_prefix(':')
}

fn parse_nolang_targets(
    text: &str,
    line_number: usize,
) -> Result<Vec<NolangTarget>, LanguageDirectiveExtractError> {
    text.split(',')
        .enumerate()
        .map(|(target_index, raw_target)| {
            let target_index = target_index + 1;
            let target = raw_target.trim_matches(' ');
            if target.is_empty() {
                return Err(LanguageDirectiveExtractError::EmptyNolangTarget {
                    line_number,
                    target_index,
                });
            }

            match target {
                "name" => Ok(NolangTarget::Name),
                "zero_label" => Ok(NolangTarget::ZeroLabel),
                "options" => Ok(NolangTarget::Options),
                "script_name" => Ok(NolangTarget::ScriptName),
                _ => {
                    if let Some(option_name) = target.strip_prefix("option:") {
                        if option_name.is_empty() {
                            Err(LanguageDirectiveExtractError::EmptyNolangOptionName {
                                line_number,
                                target_index,
                            })
                        } else {
                            Ok(NolangTarget::Option(option_name.to_string()))
                        }
                    } else {
                        Err(LanguageDirectiveExtractError::UnknownNolangTarget {
                            target: target.to_string(),
                            line_number,
                            target_index,
                        })
                    }
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_all_language_directive_types_and_multiline_blocks() {
        let source = concat!(
            "---$script_tips:Script tips\n",
            "---:continued\n",
            "first\n",
            "---$tips:UI tips\n",
            "---   :continued\n",
            "second\n",
            "---$nolang: name, zero_label, options, option:選択肢, script_name\n",
        );

        assert_eq!(
            remove_language_directives(source).unwrap(),
            "first\nsecond\n"
        );
    }

    #[test]
    fn removes_directives_at_beginning_middle_and_end_with_or_without_final_lf() {
        for (source, expected) in [
            (
                "---$nolang:name\nA\n---$tips:middle\nB\n---$script_tips:end\n",
                "A\nB\n",
            ),
            (
                "---$nolang:name\nA\n---$tips:middle\nB\n---$script_tips:end",
                "A\nB\n",
            ),
        ] {
            assert_eq!(remove_language_directives(source).unwrap(), expected);
        }

        assert_eq!(
            remove_language_directives("---$tips:first\nretained").unwrap(),
            "retained"
        );
    }

    #[test]
    fn preserves_blank_lines_non_targets_ui_directives_unknowns_and_labels() {
        let source = concat!(
            "ordinary\n",
            "\n",
            "-- ---$tips:comment\n",
            " ---$tips:indented\n",
            "---$foo:unknown\n",
            "---$Tips:uppercase\n",
            "---$nolang_extra:name\n",
            "---$track:Track\n",
            "@Label\n",
        );

        assert_eq!(remove_language_directives(source).unwrap(), source);
    }

    #[test]
    fn propagates_existing_extractor_errors() {
        for source in ["---$nolang\n", "---$tips:\n", "---$nolang:unknown\n"] {
            assert!(remove_language_directives(source).is_err());
        }
    }

    #[test]
    fn extracts_single_line_tips_and_script_tips_in_order() {
        let body = concat!(
            "local value = 0\n",
            "---$tips: UI tips \n",
            "---$script_tips:Script tips",
        );

        let directives = extract_language_directives(body).unwrap();

        assert_eq!(
            directives,
            vec![
                LanguageDirective {
                    kind: LanguageDirectiveKind::Tips {
                        text: " UI tips ".to_string(),
                    },
                    span: SourceSpan {
                        start_line: 2,
                        end_line: 2,
                    },
                },
                LanguageDirective {
                    kind: LanguageDirectiveKind::ScriptTips {
                        text: "Script tips".to_string(),
                    },
                    span: SourceSpan {
                        start_line: 3,
                        end_line: 3,
                    },
                },
            ]
        );
    }

    #[test]
    fn extracts_multiline_tips_with_empty_lines_and_preserves_whitespace() {
        let body = concat!(
            "---$tips:\n",
            "---:2行目\n",
            "---:\n",
            "---    : 4行目 \n",
            "---$nolang:name\n",
        );

        let directives = extract_language_directives(body).unwrap();

        assert_eq!(
            directives[0],
            LanguageDirective {
                kind: LanguageDirectiveKind::Tips {
                    text: "\n2行目\n\n 4行目 ".to_string(),
                },
                span: SourceSpan {
                    start_line: 1,
                    end_line: 4,
                },
            }
        );
        assert!(matches!(
            directives[1].kind,
            LanguageDirectiveKind::Nolang { .. }
        ));
    }

    #[test]
    fn empty_start_and_empty_continuation_represent_a_blank_line() {
        let directives = extract_language_directives("---$script_tips:\n---:\n").unwrap();

        assert_eq!(
            directives[0].kind,
            LanguageDirectiveKind::ScriptTips {
                text: "\n".to_string()
            }
        );
        assert_eq!(directives[0].span.end_line, 2);
    }

    #[test]
    fn handles_lf_crlf_mixed_endings_and_missing_trailing_newline() {
        let body = "---$tips:first\r\n---:second\n---  :third\r\n---$script_tips:last";

        let directives = extract_language_directives(body).unwrap();

        assert_eq!(
            directives[0].kind,
            LanguageDirectiveKind::Tips {
                text: "first\nsecond\nthird".to_string()
            }
        );
        assert_eq!(
            directives[1].kind,
            LanguageDirectiveKind::ScriptTips {
                text: "last".to_string()
            }
        );
        assert_eq!(directives[1].span.start_line, 4);
    }

    #[test]
    fn preserves_lone_carriage_return_as_tips_text() {
        let directives = extract_language_directives("---$tips:text\r").unwrap();

        assert_eq!(
            directives[0].kind,
            LanguageDirectiveKind::Tips {
                text: "text\r".to_string()
            }
        );
    }

    #[test]
    fn recognizes_directives_at_physical_line_start_without_considering_lua_syntax() {
        let body = concat!(
            "local text = [[\n",
            "---$tips:Inside string\n",
            "]]\n",
            "--[[\n",
            "---$script_tips:Inside comment\n",
            "]]\n",
        );

        let directives = extract_language_directives(body).unwrap();

        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].span.start_line, 2);
        assert_eq!(directives[1].span.start_line, 5);
    }

    #[test]
    fn ignores_indented_uppercase_and_similarly_named_directives() {
        let body = concat!(
            " ---$tips:Indented\n",
            "\t---$nolang:name\n",
            "---$Tips:Uppercase\n",
            "---$tip:text\n",
            "---$nolang_extra:name\n",
        );

        assert!(extract_language_directives(body).unwrap().is_empty());
    }

    #[test]
    fn tabbed_continuation_is_not_consumed_and_lone_continuations_are_ignored() {
        let body = concat!(
            "---$tips:first\n",
            "---\t:not continuation\n",
            "---$nolang:name\n",
            "---:lone continuation\n",
        );

        let directives = extract_language_directives(body).unwrap();

        assert_eq!(directives.len(), 2);
        assert_eq!(
            directives[0],
            LanguageDirective {
                kind: LanguageDirectiveKind::Tips {
                    text: "first".to_string()
                },
                span: SourceSpan {
                    start_line: 1,
                    end_line: 1,
                },
            }
        );
        assert_eq!(directives[1].span.start_line, 3);
    }

    #[test]
    fn extracts_all_nolang_targets_in_order_without_removing_duplicates() {
        let body = concat!(
            "---$nolang: name, zero_label, options, option: First, option:選択肢, script_name, name\n",
            "---$nolang: option:Ä, option:ä\n",
        );

        let directives = extract_language_directives(body).unwrap();

        assert_eq!(directives.len(), 2);
        assert_eq!(
            directives[0].kind,
            LanguageDirectiveKind::Nolang {
                targets: vec![
                    NolangTarget::Name,
                    NolangTarget::ZeroLabel,
                    NolangTarget::Options,
                    NolangTarget::Option(" First".to_string()),
                    NolangTarget::Option("選択肢".to_string()),
                    NolangTarget::ScriptName,
                    NolangTarget::Name,
                ]
            }
        );
        assert_eq!(
            directives[1].kind,
            LanguageDirectiveKind::Nolang {
                targets: vec![
                    NolangTarget::Option("Ä".to_string()),
                    NolangTarget::Option("ä".to_string()),
                ]
            }
        );
    }

    #[test]
    fn trims_only_outer_ascii_spaces_from_nolang_targets() {
        let directives =
            extract_language_directives("---$nolang:  name  , option: value  \n").unwrap();

        assert_eq!(
            directives[0].kind,
            LanguageDirectiveKind::Nolang {
                targets: vec![
                    NolangTarget::Name,
                    NolangTarget::Option(" value".to_string()),
                ]
            }
        );

        assert_eq!(
            extract_language_directives("---$nolang:\tname\n").unwrap_err(),
            LanguageDirectiveExtractError::UnknownNolangTarget {
                target: "\tname".to_string(),
                line_number: 1,
                target_index: 1,
            }
        );
    }

    #[test]
    fn missing_colon_is_an_error_for_each_known_directive() {
        for (body, directive) in [
            ("---$tips\n", LanguageDirectiveName::Tips),
            ("---$script_tips\n", LanguageDirectiveName::ScriptTips),
            ("---$nolang\n", LanguageDirectiveName::Nolang),
        ] {
            assert_eq!(
                extract_language_directives(body).unwrap_err(),
                LanguageDirectiveExtractError::MissingColon {
                    directive,
                    line_number: 1,
                }
            );
        }
    }

    #[test]
    fn entirely_empty_tips_block_is_an_error() {
        for (body, directive) in [
            ("---$tips:\n", LanguageDirectiveName::Tips),
            ("---$script_tips:", LanguageDirectiveName::ScriptTips),
        ] {
            assert_eq!(
                extract_language_directives(body).unwrap_err(),
                LanguageDirectiveExtractError::EmptyTips {
                    directive,
                    line_number: 1,
                }
            );
        }
    }

    #[test]
    fn empty_nolang_elements_are_errors() {
        for (body, target_index) in [
            ("---$nolang:\n", 1),
            ("---$nolang:,name\n", 1),
            ("---$nolang:name,,options\n", 2),
            ("---$nolang:name,\n", 2),
        ] {
            assert_eq!(
                extract_language_directives(body).unwrap_err(),
                LanguageDirectiveExtractError::EmptyNolangTarget {
                    line_number: 1,
                    target_index,
                }
            );
        }
    }

    #[test]
    fn unknown_nolang_target_is_an_error() {
        assert_eq!(
            extract_language_directives("---$nolang:name, unknown\n").unwrap_err(),
            LanguageDirectiveExtractError::UnknownNolangTarget {
                target: "unknown".to_string(),
                line_number: 1,
                target_index: 2,
            }
        );
    }

    #[test]
    fn empty_nolang_option_name_is_an_error() {
        assert_eq!(
            extract_language_directives("---$nolang:name, option: \n").unwrap_err(),
            LanguageDirectiveExtractError::EmptyNolangOptionName {
                line_number: 1,
                target_index: 2,
            }
        );
    }
}
