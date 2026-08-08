use crate::aul2_document::{Aul2Document, Aul2PreambleNode, Aul2SectionNode};

pub(crate) fn encode_aul2_value(value: &str) -> String {
    value.replace('\n', "\\n")
}

pub(crate) fn serialize_aul2_document(document: &Aul2Document) -> String {
    let mut output = String::new();
    let mut has_physical_line = false;

    for node in &document.preamble {
        begin_physical_line(&mut output, &mut has_physical_line);
        match node {
            Aul2PreambleNode::Comment(comment) => output.push_str(&comment.raw_text),
            Aul2PreambleNode::Blank(blank) => output.push_str(&blank.raw_text),
        }
    }

    for section in &document.sections {
        begin_physical_line(&mut output, &mut has_physical_line);
        output.push('[');
        output.push_str(&section.name);
        output.push(']');

        for node in &section.body {
            begin_physical_line(&mut output, &mut has_physical_line);
            match node {
                Aul2SectionNode::Entry(entry) => {
                    output.push_str(&entry.key);
                    output.push('=');
                    output.push_str(&entry.value_text);
                }
                Aul2SectionNode::Comment(comment) => output.push_str(&comment.raw_text),
                Aul2SectionNode::Blank(blank) => output.push_str(&blank.raw_text),
            }
        }
    }

    if document.has_trailing_newline {
        output.push('\n');
    }

    output
}

fn begin_physical_line(output: &mut String, has_physical_line: &mut bool) {
    if *has_physical_line {
        output.push('\n');
    } else {
        *has_physical_line = true;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use crate::aul2_document::{Aul2Blank, Aul2Comment, Aul2Entry, Aul2Section, Aul2SectionNode};
    use crate::aul2_parser::parse_aul2_document;

    use super::*;

    fn document(
        preamble: Vec<Aul2PreambleNode>,
        sections: Vec<Aul2Section>,
        has_trailing_newline: bool,
    ) -> Aul2Document {
        Aul2Document {
            preamble,
            sections,
            has_trailing_newline,
        }
    }

    fn section(name: &str, body: Vec<Aul2SectionNode>) -> Aul2Section {
        Aul2Section {
            name: name.to_string(),
            source_line: None,
            body,
        }
    }

    fn entry(key: &str, value_text: &str, source_line: Option<usize>) -> Aul2SectionNode {
        Aul2SectionNode::Entry(Aul2Entry {
            key: key.to_string(),
            value_text: value_text.to_string(),
            source_line,
        })
    }

    fn blank(raw_text: &str, source_line: Option<usize>) -> Aul2Blank {
        Aul2Blank {
            raw_text: raw_text.to_string(),
            source_line,
        }
    }

    fn comment(raw_text: &str, source_line: Option<usize>) -> Aul2Comment {
        Aul2Comment {
            raw_text: raw_text.to_string(),
            source_line,
        }
    }

    #[test]
    fn encodes_only_actual_lf_in_aul2_values() {
        let cases = [
            ("", ""),
            ("plain", "plain"),
            ("a\nb", "a\\nb"),
            ("a\nb\nc", "a\\nb\\nc"),
            ("\na", "\\na"),
            ("a\n", "a\\n"),
            (r"a\nb", r"a\nb"),
            (r"a\b", r"a\b"),
            ("日本語\n説明", "日本語\\n説明"),
            ("a\rb", "a\rb"),
            ("a\r\nb", "a\r\\nb"),
        ];

        for (value, expected) in cases {
            assert_eq!(encode_aul2_value(value), expected, "value: {value:?}");
        }
    }

    #[test]
    fn empty_document_always_respects_trailing_newline_metadata() {
        assert_eq!(
            serialize_aul2_document(&document(Vec::new(), Vec::new(), false)),
            ""
        );
        assert_eq!(
            serialize_aul2_document(&document(Vec::new(), Vec::new(), true)),
            "\n"
        );
    }

    #[test]
    fn serializes_preamble_raw_text_in_order_and_ignores_source_lines() {
        let document = document(
            vec![
                Aul2PreambleNode::Comment(comment("; comment", Some(20))),
                Aul2PreambleNode::Blank(blank(" \t ", None)),
                Aul2PreambleNode::Comment(comment(";second", Some(1))),
            ],
            Vec::new(),
            false,
        );

        assert_eq!(
            serialize_aul2_document(&document),
            "; comment\n \t \n;second"
        );
    }

    #[test]
    fn serializes_section_entry_spacing_and_lexical_value_without_encoding_again() {
        let document = document(
            Vec::new(),
            vec![Aul2Section {
                name: " section ".to_string(),
                source_line: Some(30),
                body: vec![
                    entry("key ", " value", Some(31)),
                    entry("tips", r"first\nsecond", None),
                    entry("raw-cr", "a\rb", None),
                ],
            }],
            false,
        );

        assert_eq!(
            serialize_aul2_document(&document),
            "[ section ]\nkey = value\ntips=first\\nsecond\nraw-cr=a\rb"
        );
    }

    #[test]
    fn does_not_insert_blank_lines_between_sections() {
        let document = document(
            Vec::new(),
            vec![
                section("first", Vec::new()),
                section("second", vec![entry("key", "value", None)]),
            ],
            false,
        );

        assert_eq!(
            serialize_aul2_document(&document),
            "[first]\n[second]\nkey=value"
        );
    }

    #[test]
    fn trailing_blank_and_final_newline_each_produce_their_own_lf() {
        let without_blank = document(
            Vec::new(),
            vec![section("section", vec![entry("key", "value", None)])],
            true,
        );
        assert_eq!(
            serialize_aul2_document(&without_blank),
            "[section]\nkey=value\n"
        );

        let with_blank = document(
            Vec::new(),
            vec![section(
                "section",
                vec![
                    entry("key", "value", None),
                    Aul2SectionNode::Blank(blank("", Some(3))),
                ],
            )],
            true,
        );
        assert_eq!(
            serialize_aul2_document(&with_blank),
            "[section]\nkey=value\n\n"
        );
    }

    #[test]
    fn parser_round_trip_normalizes_only_bom_and_newline_separators() {
        let cases = [
            (
                "; preamble\n\t\n[ section ]\nkey = value\ntips=first\\nsecond",
                "; preamble\n\t\n[ section ]\nkey = value\ntips=first\\nsecond",
            ),
            (
                "; preamble\r\n\t\r\n[ section ]\r\nkey = value\r\n",
                "; preamble\n\t\n[ section ]\nkey = value\n",
            ),
            (
                "; preamble\r\t\r[ section ]\rkey = value\r",
                "; preamble\n\t\n[ section ]\nkey = value\n",
            ),
            (
                "\u{feff}; preamble\r\n\t\r[ section ]\nkey = value",
                "; preamble\n\t\n[ section ]\nkey = value",
            ),
        ];

        for (input, expected) in cases {
            let parsed = parse_aul2_document(input).unwrap();
            assert_eq!(serialize_aul2_document(&parsed), expected);
        }
    }

    #[test]
    fn existing_lf_fixtures_round_trip_exactly_without_double_encoding() {
        for relative_path in [
            "tests/fixtures/language/update/input/English.basic.aul2",
            "tests/fixtures/language/basic/expected/Default.basic.aul2",
        ] {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative_path);
            let input = fs::read_to_string(path).unwrap();
            let parsed = parse_aul2_document(&input).unwrap();

            assert_eq!(
                serialize_aul2_document(&parsed),
                input,
                "fixture: {relative_path}"
            );
        }
    }
}
