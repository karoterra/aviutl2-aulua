use crate::aul2_document::{
    Aul2Blank, Aul2Document, Aul2Entry, Aul2PreambleNode, Aul2Section, Aul2SectionNode,
};
use crate::aul2_serializer::encode_aul2_value;
use crate::language_file_plan::{LanguageFilePlan, LanguageSectionPlan};

pub(crate) fn update_aul2_document(document: &mut Aul2Document, plan: &LanguageFilePlan) {
    let mut missing_sections = Vec::new();

    for section_plan in plan
        .sections
        .iter()
        .filter(|section| !section_is_empty(section))
    {
        if let Some(section) = document.find_section_mut(section_plan.name()) {
            add_missing_entries(section, section_plan);
        } else {
            missing_sections.push(section_plan);
        }
    }

    if missing_sections.is_empty() {
        return;
    }

    ensure_blank_before_appended_sections(document);
    let missing_section_count = missing_sections.len();

    for (index, section_plan) in missing_sections.into_iter().enumerate() {
        let mut section = build_generated_section(section_plan);
        if index + 1 < missing_section_count {
            section.body.push(generated_section_blank());
        }
        document.push_section(section);
    }
}

fn section_is_empty(section: &LanguageSectionPlan) -> bool {
    match section {
        LanguageSectionPlan::Text { entries, .. } => entries.is_empty(),
        LanguageSectionPlan::Tooltip { entries, .. } => entries.is_empty(),
    }
}

fn add_missing_entries(section: &mut Aul2Section, section_plan: &LanguageSectionPlan) {
    match section_plan {
        LanguageSectionPlan::Text { entries, .. } => {
            for entry in entries {
                add_missing_entry(section, &entry.key, &entry.value);
            }
        }
        LanguageSectionPlan::Tooltip { entries, .. } => {
            for entry in entries {
                add_missing_entry(section, &entry.key, &entry.value);
            }
        }
    }
}

fn add_missing_entry(section: &mut Aul2Section, key: &str, value: &str) {
    if section.find_entry(key).is_none() {
        section.insert_entry_at_end(build_generated_entry(key, value));
    }
}

fn build_generated_section(section_plan: &LanguageSectionPlan) -> Aul2Section {
    let (name, body) = match section_plan {
        LanguageSectionPlan::Text { name, entries, .. } => (
            name,
            entries
                .iter()
                .map(|entry| build_generated_entry(&entry.key, &entry.value))
                .map(Aul2SectionNode::Entry)
                .collect(),
        ),
        LanguageSectionPlan::Tooltip { name, entries, .. } => (
            name,
            entries
                .iter()
                .map(|entry| build_generated_entry(&entry.key, &entry.value))
                .map(Aul2SectionNode::Entry)
                .collect(),
        ),
    };

    Aul2Section {
        name: name.clone(),
        source_line: None,
        body,
    }
}

fn build_generated_entry(key: &str, value: &str) -> Aul2Entry {
    Aul2Entry {
        key: key.to_string(),
        value_text: encode_aul2_value(value),
        source_line: None,
    }
}

fn generated_section_blank() -> Aul2SectionNode {
    Aul2SectionNode::Blank(Aul2Blank {
        raw_text: String::new(),
        source_line: None,
    })
}

fn ensure_blank_before_appended_sections(document: &mut Aul2Document) {
    if let Some(section) = document.sections.last_mut() {
        if !matches!(section.body.last(), Some(Aul2SectionNode::Blank(_))) {
            section.body.push(generated_section_blank());
        }
    } else if !document.preamble.is_empty()
        && !matches!(document.preamble.last(), Some(Aul2PreambleNode::Blank(_)))
    {
        document.preamble.push(Aul2PreambleNode::Blank(Aul2Blank {
            raw_text: String::new(),
            source_line: None,
        }));
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::aul2_parser::parse_aul2_document;
    use crate::aul2_serializer::serialize_aul2_document;
    use crate::common::get_fixture_path;
    use crate::config_loader::load_config;
    use crate::language_file_plan::{LanguageTextEntryPlan, LanguageTooltipEntryPlan};
    use crate::language_file_request::{
        LanguageFileRequest, LanguageFileRequestOrigin, LanguageFileSelection,
    };
    use crate::language_plan::build_language_plan;
    use crate::language_script_analysis::{LanguageScriptInput, LogicalScriptOrigin};
    use crate::language_script_entries::{LanguageTextOrigin, LanguageTipsOrigin};
    use crate::language_ui::SourceSpan;

    fn request(is_default: bool) -> LanguageFileRequest {
        LanguageFileRequest {
            path: PathBuf::from("Language/Test.aul2"),
            text: true,
            tooltip: true,
            is_default,
            origin: LanguageFileRequestOrigin::Override,
        }
    }

    fn origin(name: &str) -> LogicalScriptOrigin {
        LogicalScriptOrigin::Direct {
            path: PathBuf::from(name),
        }
    }

    fn text_section(
        name: &str,
        entries: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> LanguageSectionPlan {
        LanguageSectionPlan::Text {
            name: name.to_string(),
            logical_script_name: name.to_string(),
            origin: origin(name),
            entries: entries
                .into_iter()
                .map(|(key, value)| LanguageTextEntryPlan {
                    key: key.to_string(),
                    value: value.to_string(),
                    origins: vec![LanguageTextOrigin::ScriptName],
                })
                .collect(),
        }
    }

    fn tooltip_section(
        name: &str,
        entries: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> LanguageSectionPlan {
        LanguageSectionPlan::Tooltip {
            name: name.to_string(),
            logical_script_name: name.to_string(),
            origin: origin(name),
            entries: entries
                .into_iter()
                .map(|(key, value)| LanguageTooltipEntryPlan {
                    key: key.to_string(),
                    value: value.to_string(),
                    origin: LanguageTipsOrigin::ScriptTips {
                        span: SourceSpan {
                            start_line: 1,
                            end_line: 1,
                        },
                    },
                })
                .collect(),
        }
    }

    fn plan(
        is_default: bool,
        sections: impl IntoIterator<Item = LanguageSectionPlan>,
    ) -> LanguageFilePlan {
        LanguageFilePlan {
            request: request(is_default),
            sections: sections.into_iter().collect(),
        }
    }

    fn entry(key: &str, value_text: &str, source_line: Option<usize>) -> Aul2SectionNode {
        Aul2SectionNode::Entry(Aul2Entry {
            key: key.to_string(),
            value_text: value_text.to_string(),
            source_line,
        })
    }

    fn blank(raw_text: &str, source_line: Option<usize>) -> Aul2SectionNode {
        Aul2SectionNode::Blank(Aul2Blank {
            raw_text: raw_text.to_string(),
            source_line,
        })
    }

    fn comment(raw_text: &str, source_line: Option<usize>) -> Aul2SectionNode {
        Aul2SectionNode::Comment(crate::aul2_document::Aul2Comment {
            raw_text: raw_text.to_string(),
            source_line,
        })
    }

    fn section(name: &str, source_line: Option<usize>, body: Vec<Aul2SectionNode>) -> Aul2Section {
        Aul2Section {
            name: name.to_string(),
            source_line,
            body,
        }
    }

    fn document(sections: Vec<Aul2Section>, has_trailing_newline: bool) -> Aul2Document {
        Aul2Document {
            preamble: Vec::new(),
            sections,
            has_trailing_newline,
        }
    }

    #[test]
    fn existing_values_are_kept_even_for_default_requests() {
        let mut document = document(
            vec![section(
                "script",
                Some(1),
                vec![entry("key", "translated", Some(2))],
            )],
            false,
        );
        let original = document.clone();

        update_aul2_document(
            &mut document,
            &plan(true, [text_section("script", [("key", "expected")])]),
        );

        assert_eq!(document, original);
    }

    #[test]
    fn missing_keys_follow_plan_order_before_trailing_blanks_and_keep_existing_nodes() {
        let mut document = document(
            vec![section(
                "script",
                Some(1),
                vec![
                    entry("c", "existing", Some(2)),
                    comment("; keep", Some(3)),
                    blank("", Some(4)),
                    blank("\t", Some(5)),
                ],
            )],
            true,
        );

        update_aul2_document(
            &mut document,
            &plan(
                false,
                [text_section(
                    "script",
                    [("a", "first\nline"), ("b", "second"), ("c", "ignored")],
                )],
            ),
        );

        assert_eq!(
            document.sections[0].body,
            vec![
                entry("c", "existing", Some(2)),
                comment("; keep", Some(3)),
                entry("a", "first\\nline", None),
                entry("b", "second", None),
                blank("", Some(4)),
                blank("\t", Some(5)),
            ]
        );
        assert!(document.has_trailing_newline);
    }

    #[test]
    fn stale_entries_and_unknown_sections_are_preserved_completely() {
        let mut document = document(
            vec![
                section(
                    "managed",
                    Some(1),
                    vec![
                        entry("needed", "translation", Some(2)),
                        entry("stale", "keep", Some(3)),
                    ],
                ),
                section(
                    "Unknown",
                    Some(4),
                    vec![comment("; custom", Some(5)), entry("x", "y", Some(6))],
                ),
            ],
            true,
        );
        let unknown = document.sections[1].clone();

        update_aul2_document(
            &mut document,
            &plan(false, [text_section("managed", [("needed", "different")])]),
        );

        assert_eq!(
            document.sections[0].find_entry("stale").unwrap().value_text,
            "keep"
        );
        assert_eq!(document.sections[1], unknown);
    }

    #[test]
    fn missing_sections_are_appended_in_plan_order_without_moving_existing_sections() {
        let mut document = document(
            vec![
                section(
                    "C",
                    Some(1),
                    vec![entry("c", "keep", Some(2)), blank("", Some(3))],
                ),
                section("Unknown", Some(4), vec![entry("x", "keep", Some(5))]),
                section("A", Some(6), vec![entry("a", "keep", Some(7))]),
            ],
            false,
        );

        update_aul2_document(
            &mut document,
            &plan(
                false,
                [
                    text_section("A", [("a", "")]),
                    text_section("B", [("b", "new\nvalue")]),
                    text_section("C", [("c", "")]),
                    tooltip_section("D", [("tip", "description")]),
                ],
            ),
        );

        assert_eq!(
            document
                .sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "Unknown", "A", "B", "D"]
        );
        assert_eq!(document.sections[0].source_line, Some(1));
        assert_eq!(document.sections[1].source_line, Some(4));
        assert_eq!(document.sections[2].source_line, Some(6));
        assert_eq!(document.sections[3].source_line, None);
        assert_eq!(document.sections[4].source_line, None);
        assert!(matches!(
            document.sections[2].body.last(),
            Some(Aul2SectionNode::Blank(Aul2Blank {
                source_line: None,
                ..
            }))
        ));
        assert!(matches!(
            document.sections[3].body.last(),
            Some(Aul2SectionNode::Blank(Aul2Blank {
                source_line: None,
                ..
            }))
        ));
        assert!(matches!(
            document.sections[4].body.last(),
            Some(Aul2SectionNode::Entry(_))
        ));
        assert_eq!(
            document.sections[3].find_entry("b").unwrap().value_text,
            "new\\nvalue"
        );
        assert!(!document.has_trailing_newline);
    }

    #[test]
    fn existing_trailing_blank_count_is_only_increased_when_zero() {
        for (existing_blank_count, expected_blank_count) in [(0, 1), (1, 1), (2, 2)] {
            let mut body = vec![entry("existing", "value", Some(2))];
            body.extend((0..existing_blank_count).map(|index| blank("", Some(index + 3))));
            let mut document = document(vec![section("existing", Some(1), body)], true);

            update_aul2_document(
                &mut document,
                &plan(false, [text_section("new", [("key", "")])]),
            );

            let actual_blank_count = document.sections[0]
                .body
                .iter()
                .rev()
                .take_while(|node| matches!(node, Aul2SectionNode::Blank(_)))
                .count();
            assert_eq!(
                actual_blank_count, expected_blank_count,
                "existing blank count: {existing_blank_count}"
            );
        }

        let mut comment_ended = document(
            vec![section(
                "existing",
                Some(1),
                vec![
                    entry("existing", "value", Some(2)),
                    comment("; trailing comment", Some(3)),
                ],
            )],
            false,
        );
        update_aul2_document(
            &mut comment_ended,
            &plan(false, [text_section("new", [("key", "")])]),
        );
        assert_eq!(
            comment_ended.sections[0].body,
            vec![
                entry("existing", "value", Some(2)),
                comment("; trailing comment", Some(3)),
                blank("", None),
            ]
        );
    }

    #[test]
    fn empty_and_preamble_only_documents_add_spacing_without_changing_trailing_metadata() {
        let new_section_plan = plan(false, [text_section("new", [("key", "")])]);
        let mut empty = document(Vec::new(), false);
        update_aul2_document(&mut empty, &new_section_plan);
        assert!(empty.preamble.is_empty());
        assert_eq!(serialize_aul2_document(&empty), "[new]\nkey=");
        assert!(!empty.has_trailing_newline);

        let mut preamble_only = Aul2Document {
            preamble: vec![Aul2PreambleNode::Comment(
                crate::aul2_document::Aul2Comment {
                    raw_text: "; keep".to_string(),
                    source_line: Some(1),
                },
            )],
            sections: Vec::new(),
            has_trailing_newline: true,
        };
        update_aul2_document(&mut preamble_only, &new_section_plan);
        assert_eq!(
            preamble_only.preamble.last(),
            Some(&Aul2PreambleNode::Blank(Aul2Blank {
                raw_text: String::new(),
                source_line: None,
            }))
        );
        assert_eq!(
            serialize_aul2_document(&preamble_only),
            "; keep\n\n[new]\nkey=\n"
        );
        assert!(preamble_only.has_trailing_newline);
    }

    #[test]
    fn empty_plan_sections_do_not_manage_or_create_sections() {
        let mut document = document(
            vec![section(
                "empty-text",
                Some(1),
                vec![entry("custom", "keep", Some(2))],
            )],
            false,
        );
        let original = document.clone();

        update_aul2_document(
            &mut document,
            &plan(
                false,
                [
                    text_section("empty-text", []),
                    tooltip_section("empty-tooltip", []),
                ],
            ),
        );

        assert_eq!(document, original);
    }

    #[test]
    fn update_fixture_matches_expected_document_exactly() {
        let config = load_config(get_fixture_path("language/update/input/aulua.yaml")).unwrap();
        let language_plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();
        assert_eq!(language_plan.files.len(), 1);

        let input =
            fs::read_to_string(get_fixture_path("language/update/input/English.basic.aul2"))
                .unwrap();
        let expected = fs::read_to_string(get_fixture_path(
            "language/update/expected/English.basic.aul2",
        ))
        .unwrap();
        let mut document = parse_aul2_document(&input).unwrap();

        update_aul2_document(&mut document, &language_plan.files[0]);

        assert_eq!(serialize_aul2_document(&document), expected);
    }
}
