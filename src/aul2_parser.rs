use std::collections::HashMap;

use thiserror::Error;

use crate::aul2_document::{
    Aul2Blank, Aul2Comment, Aul2Document, Aul2Entry, Aul2PreambleNode, Aul2Section, Aul2SectionNode,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ParseAul2DocumentError {
    #[error("{line_number}行目のsection headerが不正です: {line}")]
    MalformedSection { line_number: usize, line: String },

    #[error("{line_number}行目のsection名が空です")]
    EmptySectionName { line_number: usize },

    #[error("{line_number}行目にsection外のentryがあります: {line}")]
    EntryOutsideSection { line_number: usize, line: String },

    #[error("{line_number}行目のentry keyが空です: {line}")]
    EmptyKey { line_number: usize, line: String },

    #[error("{line_number}行目の構文が不正です: {line}")]
    MalformedLine { line_number: usize, line: String },

    #[error("section名 {name} が重複しています: {first_line}行目 / {duplicate_line}行目")]
    DuplicateSection {
        name: String,
        first_line: usize,
        duplicate_line: usize,
    },

    #[error(
        "section {section_name} のentry key {key} が重複しています: {first_line}行目 / {duplicate_line}行目"
    )]
    DuplicateKey {
        section_name: String,
        key: String,
        first_line: usize,
        duplicate_line: usize,
    },
}

pub(crate) fn parse_aul2_document(input: &str) -> Result<Aul2Document, ParseAul2DocumentError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let PhysicalLines {
        lines,
        has_trailing_newline,
    } = split_physical_lines(input);
    let mut document = Aul2Document {
        preamble: Vec::new(),
        sections: Vec::new(),
        has_trailing_newline,
    };
    let mut current_section_index: Option<usize> = None;
    let mut seen_sections = HashMap::<String, usize>::new();
    let mut seen_keys = HashMap::<String, usize>::new();

    for (index, line) in lines.into_iter().enumerate() {
        let line_number = index + 1;

        if is_blank(line) {
            let blank = Aul2Blank {
                raw_text: line.to_string(),
                source_line: Some(line_number),
            };
            if let Some(section_index) = current_section_index {
                document.sections[section_index]
                    .body
                    .push(Aul2SectionNode::Blank(blank));
            } else {
                document.preamble.push(Aul2PreambleNode::Blank(blank));
            }
            continue;
        }

        if line.starts_with(';') {
            let comment = Aul2Comment {
                raw_text: line.to_string(),
                source_line: Some(line_number),
            };
            if let Some(section_index) = current_section_index {
                document.sections[section_index]
                    .body
                    .push(Aul2SectionNode::Comment(comment));
            } else {
                document.preamble.push(Aul2PreambleNode::Comment(comment));
            }
            continue;
        }

        if line.starts_with('[') {
            let Some(name) = line
                .strip_prefix('[')
                .and_then(|line| line.strip_suffix(']'))
            else {
                return Err(ParseAul2DocumentError::MalformedSection {
                    line_number,
                    line: line.to_string(),
                });
            };
            if name.is_empty() {
                return Err(ParseAul2DocumentError::EmptySectionName { line_number });
            }
            if let Some(&first_line) = seen_sections.get(name) {
                return Err(ParseAul2DocumentError::DuplicateSection {
                    name: name.to_string(),
                    first_line,
                    duplicate_line: line_number,
                });
            }

            seen_sections.insert(name.to_string(), line_number);
            document.sections.push(Aul2Section {
                name: name.to_string(),
                source_line: Some(line_number),
                body: Vec::new(),
            });
            current_section_index = Some(document.sections.len() - 1);
            seen_keys.clear();
            continue;
        }

        if let Some((key, value_text)) = line.split_once('=') {
            if key.is_empty() {
                return Err(ParseAul2DocumentError::EmptyKey {
                    line_number,
                    line: line.to_string(),
                });
            }
            let Some(section_index) = current_section_index else {
                return Err(ParseAul2DocumentError::EntryOutsideSection {
                    line_number,
                    line: line.to_string(),
                });
            };
            if let Some(&first_line) = seen_keys.get(key) {
                return Err(ParseAul2DocumentError::DuplicateKey {
                    section_name: document.sections[section_index].name.clone(),
                    key: key.to_string(),
                    first_line,
                    duplicate_line: line_number,
                });
            }

            seen_keys.insert(key.to_string(), line_number);
            document.sections[section_index]
                .body
                .push(Aul2SectionNode::Entry(Aul2Entry {
                    key: key.to_string(),
                    value_text: value_text.to_string(),
                    source_line: Some(line_number),
                }));
            continue;
        }

        return Err(ParseAul2DocumentError::MalformedLine {
            line_number,
            line: line.to_string(),
        });
    }

    Ok(document)
}

fn is_blank(line: &str) -> bool {
    line.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

#[derive(Debug, PartialEq, Eq)]
struct PhysicalLines<'a> {
    lines: Vec<&'a str>,
    has_trailing_newline: bool,
}

fn split_physical_lines(input: &str) -> PhysicalLines<'_> {
    let bytes = input.as_bytes();
    let mut lines = Vec::new();
    let mut line_start = 0;
    let mut index = 0;
    let mut has_trailing_newline = false;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                lines.push(&input[line_start..index]);
                index += 1;
                line_start = index;
                has_trailing_newline = true;
            }
            b'\r' => {
                lines.push(&input[line_start..index]);
                index += 1;
                if index < bytes.len() && bytes[index] == b'\n' {
                    index += 1;
                }
                line_start = index;
                has_trailing_newline = true;
            }
            _ => {
                index += 1;
                has_trailing_newline = false;
            }
        }
    }

    if line_start < input.len() {
        lines.push(&input[line_start..]);
    }

    PhysicalLines {
        lines,
        has_trailing_newline,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn physical_line_tokenizer_handles_all_newlines_and_trailing_state() {
        let cases = [
            ("", vec![], false),
            ("abc", vec!["abc"], false),
            ("abc\n", vec!["abc"], true),
            ("abc\r", vec!["abc"], true),
            ("abc\r\n", vec!["abc"], true),
            ("\n", vec![""], true),
            ("\r", vec![""], true),
            ("\r\n", vec![""], true),
            ("a\n\n", vec!["a", ""], true),
            ("a\r\nb\rc\n", vec!["a", "b", "c"], true),
            ("a\nb", vec!["a", "b"], false),
        ];

        for (input, expected_lines, expected_trailing) in cases {
            let actual = split_physical_lines(input);
            assert_eq!(actual.lines, expected_lines, "input: {input:?}");
            assert_eq!(
                actual.has_trailing_newline, expected_trailing,
                "input: {input:?}"
            );
        }
    }

    #[test]
    fn empty_and_bom_inputs_are_parsed_without_bom_metadata() {
        for input in ["", "\u{feff}"] {
            assert_eq!(
                parse_aul2_document(input).unwrap(),
                Aul2Document {
                    preamble: Vec::new(),
                    sections: Vec::new(),
                    has_trailing_newline: false,
                }
            );
        }

        let comment = parse_aul2_document("\u{feff};comment").unwrap();
        assert_eq!(
            comment.preamble,
            vec![Aul2PreambleNode::Comment(Aul2Comment {
                raw_text: ";comment".to_string(),
                source_line: Some(1),
            })]
        );

        let newline = parse_aul2_document("\u{feff}\r\n").unwrap();
        assert_eq!(
            newline.preamble,
            vec![Aul2PreambleNode::Blank(Aul2Blank {
                raw_text: String::new(),
                source_line: Some(1),
            })]
        );
        assert!(newline.has_trailing_newline);
    }

    #[test]
    fn preamble_and_section_nodes_preserve_raw_text_order_and_lines() {
        let document =
            parse_aul2_document("; preamble\r\n\t\r[ section ]\nkey = value\r; body\n \t \n[next]")
                .unwrap();

        assert_eq!(
            document.preamble,
            vec![
                Aul2PreambleNode::Comment(Aul2Comment {
                    raw_text: "; preamble".to_string(),
                    source_line: Some(1),
                }),
                Aul2PreambleNode::Blank(Aul2Blank {
                    raw_text: "\t".to_string(),
                    source_line: Some(2),
                }),
            ]
        );
        assert_eq!(document.sections[0].name, " section ");
        assert_eq!(document.sections[0].source_line, Some(3));
        assert_eq!(
            document.sections[0].body,
            vec![
                Aul2SectionNode::Entry(Aul2Entry {
                    key: "key ".to_string(),
                    value_text: " value".to_string(),
                    source_line: Some(4),
                }),
                Aul2SectionNode::Comment(Aul2Comment {
                    raw_text: "; body".to_string(),
                    source_line: Some(5),
                }),
                Aul2SectionNode::Blank(Aul2Blank {
                    raw_text: " \t ".to_string(),
                    source_line: Some(6),
                }),
            ]
        );
        assert_eq!(document.sections[1].name, "next");
        assert_eq!(document.sections[1].source_line, Some(7));
        assert!(!document.has_trailing_newline);
    }

    #[test]
    fn section_syntax_is_validated_without_trimming_or_fallback() {
        assert_eq!(
            parse_aul2_document("[]"),
            Err(ParseAul2DocumentError::EmptySectionName { line_number: 1 })
        );

        for line in ["[foo", "[", "[foo]bar", "[foo]\t"] {
            assert_eq!(
                parse_aul2_document(line),
                Err(ParseAul2DocumentError::MalformedSection {
                    line_number: 1,
                    line: line.to_string(),
                })
            );
        }

        assert_eq!(
            parse_aul2_document("foo]"),
            Err(ParseAul2DocumentError::MalformedLine {
                line_number: 1,
                line: "foo]".to_string(),
            })
        );
    }

    #[test]
    fn entries_split_only_first_equals_and_preserve_whitespace() {
        let document = parse_aul2_document(
            "[section]\nkey=value\nempty=\nkey = value\n key=a=b=c\ntext=literal\\nvalue",
        )
        .unwrap();
        let entries = document.sections[0]
            .body
            .iter()
            .filter_map(|node| match node {
                Aul2SectionNode::Entry(entry) => Some(entry),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            entries
                .iter()
                .map(|entry| (
                    entry.key.as_str(),
                    entry.value_text.as_str(),
                    entry.source_line
                ))
                .collect::<Vec<_>>(),
            vec![
                ("key", "value", Some(2)),
                ("empty", "", Some(3)),
                ("key ", " value", Some(4)),
                (" key", "a=b=c", Some(5)),
                ("text", "literal\\nvalue", Some(6)),
            ]
        );
    }

    #[test]
    fn empty_key_and_entry_outside_section_have_context() {
        assert_eq!(
            parse_aul2_document("=value"),
            Err(ParseAul2DocumentError::EmptyKey {
                line_number: 1,
                line: "=value".to_string(),
            })
        );
        assert_eq!(
            parse_aul2_document("key=value"),
            Err(ParseAul2DocumentError::EntryOutsideSection {
                line_number: 1,
                line: "key=value".to_string(),
            })
        );
    }

    #[test]
    fn comment_and_blank_classification_is_ascii_and_position_sensitive() {
        let document = parse_aul2_document(";key=value\n[section]\n#key=value\n \t ").unwrap();
        assert!(matches!(document.preamble[0], Aul2PreambleNode::Comment(_)));
        assert_eq!(
            document.sections[0].find_entry("#key").unwrap().value_text,
            "value"
        );
        assert!(matches!(
            document.sections[0].body[1],
            Aul2SectionNode::Blank(_)
        ));

        for line in [" ;comment", "\t;comment", "#comment", "\u{a0}", "\u{3000}"] {
            assert_eq!(
                parse_aul2_document(line),
                Err(ParseAul2DocumentError::MalformedLine {
                    line_number: 1,
                    line: line.to_string(),
                })
            );
        }
    }

    #[test]
    fn duplicate_sections_use_exact_names_and_report_both_lines() {
        assert_eq!(
            parse_aul2_document("[foo]\n\n[foo]"),
            Err(ParseAul2DocumentError::DuplicateSection {
                name: "foo".to_string(),
                first_line: 1,
                duplicate_line: 3,
            })
        );

        let composed = "é";
        let decomposed = "e\u{301}";
        let input = format!("[foo]\n[Foo]\n[ foo ]\n[{composed}]\n[{decomposed}]");
        let document = parse_aul2_document(&input).unwrap();
        assert_eq!(
            document
                .sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>(),
            vec!["foo", "Foo", " foo ", composed, decomposed]
        );
    }

    #[test]
    fn duplicate_keys_are_scoped_to_exact_section_and_report_both_lines() {
        assert_eq!(
            parse_aul2_document("[first]\nkey=1\n; comment\nkey=2"),
            Err(ParseAul2DocumentError::DuplicateKey {
                section_name: "first".to_string(),
                key: "key".to_string(),
                first_line: 2,
                duplicate_line: 4,
            })
        );

        let composed = "é";
        let decomposed = "e\u{301}";
        let input =
            format!("[first]\nkey=1\nKey=2\n key=3\n{composed}=4\n{decomposed}=5\n[second]\nkey=6");
        let document = parse_aul2_document(&input).unwrap();
        assert_eq!(document.sections.len(), 2);
        assert_eq!(document.sections[0].body.len(), 5);
        assert_eq!(
            document.sections[1].find_entry("key").unwrap().value_text,
            "6"
        );
    }

    #[test]
    fn update_fixture_is_parsed_as_lossless_structure_without_ownership() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/language/update/input/English.basic.aul2");
        let input = fs::read_to_string(path).unwrap();
        let document = parse_aul2_document(&input).unwrap();

        assert_eq!(
            document.preamble,
            vec![
                Aul2PreambleNode::Comment(Aul2Comment {
                    raw_text: "; ファイル先頭コメント".to_string(),
                    source_line: Some(1),
                }),
                Aul2PreambleNode::Blank(Aul2Blank {
                    raw_text: String::new(),
                    source_line: Some(2),
                }),
            ]
        );
        assert_eq!(
            document
                .sections
                .iter()
                .map(|section| (section.name.as_str(), section.source_line))
                .collect::<Vec<_>>(),
            vec![
                ("single", Some(3)),
                ("Tips.single", Some(10)),
                ("Unknown", Some(14))
            ]
        );
        assert!(matches!(
            document.sections[0].body[3],
            Aul2SectionNode::Comment(Aul2Comment {
                source_line: Some(7),
                ..
            })
        ));
        assert_eq!(
            document.sections[0]
                .find_entry("独自キー")
                .unwrap()
                .value_text,
            "Keep this"
        );
        assert!(document.sections[2].find_entry("foo").is_some());
        assert!(document.has_trailing_newline);
    }
}
