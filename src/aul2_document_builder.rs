use crate::aul2_document::{Aul2Blank, Aul2Document, Aul2Entry, Aul2Section, Aul2SectionNode};
use crate::aul2_serializer::encode_aul2_value;
use crate::language_file_plan::{LanguageFilePlan, LanguageSectionPlan};

pub(crate) fn build_new_aul2_document(plan: &LanguageFilePlan) -> Aul2Document {
    let mut sections = plan
        .sections
        .iter()
        .filter_map(build_section)
        .collect::<Vec<_>>();
    let section_count = sections.len();

    for section in sections.iter_mut().take(section_count.saturating_sub(1)) {
        section.body.push(Aul2SectionNode::Blank(Aul2Blank {
            raw_text: String::new(),
            source_line: None,
        }));
    }

    Aul2Document {
        preamble: Vec::new(),
        has_trailing_newline: !sections.is_empty(),
        sections,
    }
}

fn build_section(section: &LanguageSectionPlan) -> Option<Aul2Section> {
    let (name, entries) = match section {
        LanguageSectionPlan::Text { name, entries, .. } if !entries.is_empty() => (
            name,
            entries
                .iter()
                .map(|entry| build_entry(&entry.key, &entry.value))
                .collect(),
        ),
        LanguageSectionPlan::Tooltip { name, entries, .. } if !entries.is_empty() => (
            name,
            entries
                .iter()
                .map(|entry| build_entry(&entry.key, &entry.value))
                .collect(),
        ),
        _ => return None,
    };

    Some(Aul2Section {
        name: name.clone(),
        source_line: None,
        body: entries,
    })
}

fn build_entry(key: &str, value: &str) -> Aul2SectionNode {
    Aul2SectionNode::Entry(Aul2Entry {
        key: key.to_string(),
        value_text: encode_aul2_value(value),
        source_line: None,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
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

    #[test]
    fn empty_plan_builds_an_empty_document() {
        let document = build_new_aul2_document(&plan(false, []));

        assert!(document.preamble.is_empty());
        assert!(document.sections.is_empty());
        assert!(!document.has_trailing_newline);
        assert_eq!(serialize_aul2_document(&document), "");
    }

    #[test]
    fn single_section_preserves_entry_order_without_blank_and_sets_generated_lines() {
        let document = build_new_aul2_document(&plan(
            false,
            [text_section(
                " section ",
                [("first", ""), ("second", "two"), ("third", "")],
            )],
        ));

        assert!(document.preamble.is_empty());
        assert!(document.has_trailing_newline);
        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].name, " section ");
        assert_eq!(document.sections[0].source_line, None);
        assert!(document.sections[0].body.iter().all(|node| matches!(
            node,
            Aul2SectionNode::Entry(Aul2Entry {
                source_line: None,
                ..
            })
        )));
        assert_eq!(
            document.sections[0]
                .body
                .iter()
                .filter_map(|node| match node {
                    Aul2SectionNode::Entry(entry) => Some(entry.key.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec!["first", "second", "third"]
        );
        assert_eq!(
            serialize_aul2_document(&document),
            "[ section ]\nfirst=\nsecond=two\nthird=\n"
        );
    }

    #[test]
    fn multiple_sections_keep_plan_order_with_one_blank_only_between_sections() {
        let document = build_new_aul2_document(&plan(
            false,
            [
                text_section("first", [("a", "")]),
                tooltip_section("Tips.first", [("tip", "")]),
                text_section("second", [("b", "")]),
            ],
        ));

        assert_eq!(
            document
                .sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "Tips.first", "second"]
        );
        for section in &document.sections[..2] {
            assert_eq!(
                section.body.last(),
                Some(&Aul2SectionNode::Blank(Aul2Blank {
                    raw_text: String::new(),
                    source_line: None,
                }))
            );
        }
        assert!(matches!(
            document.sections[2].body.last(),
            Some(Aul2SectionNode::Entry(_))
        ));
        assert_eq!(
            serialize_aul2_document(&document),
            "[first]\na=\n\n[Tips.first]\ntip=\n\n[second]\nb=\n"
        );
    }

    #[test]
    fn text_and_tooltip_values_are_encoded_once_without_default_recalculation() {
        let normal_request_with_value = build_new_aul2_document(&plan(
            false,
            [text_section("text", [("key", "custom\nvalue")])],
        ));
        let default_request_with_empty_value = build_new_aul2_document(&plan(
            true,
            [tooltip_section("Tips.text", [("effect.name", "")])],
        ));

        let Aul2SectionNode::Entry(text_entry) = &normal_request_with_value.sections[0].body[0]
        else {
            panic!("Text entryのはずです");
        };
        assert_eq!(text_entry.value_text, "custom\\nvalue");
        assert_eq!(
            serialize_aul2_document(&normal_request_with_value),
            "[text]\nkey=custom\\nvalue\n"
        );
        assert_eq!(
            serialize_aul2_document(&default_request_with_empty_value),
            "[Tips.text]\neffect.name=\n"
        );
    }

    #[test]
    fn empty_text_and_tooltip_sections_are_omitted_before_blank_placement() {
        let document = build_new_aul2_document(&plan(
            false,
            [
                text_section("first", [("a", "")]),
                text_section("empty-text", []),
                tooltip_section("empty-tooltip", []),
                tooltip_section("Tips.second", [("b", "")]),
            ],
        ));

        assert_eq!(
            document
                .sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>(),
            vec!["first", "Tips.second"]
        );
        assert_eq!(
            document.sections[0]
                .body
                .iter()
                .filter(|node| matches!(node, Aul2SectionNode::Blank(_)))
                .count(),
            1
        );
        assert!(
            document.sections[1]
                .body
                .iter()
                .all(|node| !matches!(node, Aul2SectionNode::Blank(_)))
        );
        assert_eq!(
            serialize_aul2_document(&document),
            "[first]\na=\n\n[Tips.second]\nb=\n"
        );
    }

    #[test]
    fn basic_language_plans_serialize_to_existing_expected_fixtures() {
        let config = load_config(get_fixture_path("language/basic/input/aulua.yaml")).unwrap();
        let language_plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();
        let expected_paths = [
            "language/basic/expected/Default.basic.aul2",
            "language/basic/expected/English.basic.aul2",
        ];

        assert_eq!(language_plan.files.len(), expected_paths.len());
        for (file_plan, expected_path) in language_plan.files.iter().zip(expected_paths) {
            let document = build_new_aul2_document(file_plan);
            let expected = fs::read_to_string(get_fixture_path(expected_path)).unwrap();

            assert_eq!(serialize_aul2_document(&document), expected);
        }
    }
}
