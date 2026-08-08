use std::collections::HashSet;

use crate::aul2_document::{Aul2Document, Aul2SectionNode};
use crate::language_file_plan::{LanguageFilePlan, LanguageSectionPlan};

pub(crate) fn prune_aul2_document(document: &mut Aul2Document, plan: &LanguageFilePlan) {
    for section_plan in &plan.sections {
        let plan_keys = section_entry_keys(section_plan);
        if plan_keys.is_empty() {
            continue;
        }

        let Some(section) = document.find_section_mut(section_plan.name()) else {
            continue;
        };

        section.body.retain(|node| match node {
            Aul2SectionNode::Entry(entry) => plan_keys.contains(entry.key.as_str()),
            Aul2SectionNode::Comment(_) | Aul2SectionNode::Blank(_) => true,
        });
    }
}

fn section_entry_keys(section: &LanguageSectionPlan) -> HashSet<&str> {
    match section {
        LanguageSectionPlan::Text { entries, .. } => {
            entries.iter().map(|entry| entry.key.as_str()).collect()
        }
        LanguageSectionPlan::Tooltip { entries, .. } => {
            entries.iter().map(|entry| entry.key.as_str()).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::aul2_check::{LanguageCheckFinding, check_aul2_document};
    use crate::aul2_document::{Aul2Blank, Aul2Comment, Aul2PreambleNode};
    use crate::aul2_parser::parse_aul2_document;
    use crate::language_file_plan::{LanguageTextEntryPlan, LanguageTooltipEntryPlan};
    use crate::language_file_request::{LanguageFileRequest, LanguageFileRequestOrigin};
    use crate::language_script_analysis::LogicalScriptOrigin;
    use crate::language_script_entries::{LanguageTextOrigin, LanguageTipsOrigin};
    use crate::language_ui::SourceSpan;

    fn plan(sections: Vec<LanguageSectionPlan>) -> LanguageFilePlan {
        LanguageFilePlan {
            request: LanguageFileRequest {
                path: PathBuf::from("Language/Test.aul2"),
                text: true,
                tooltip: true,
                is_default: false,
                origin: LanguageFileRequestOrigin::Override,
            },
            sections,
        }
    }

    fn origin() -> LogicalScriptOrigin {
        LogicalScriptOrigin::Direct {
            path: PathBuf::from("script.anm2"),
        }
    }

    fn text_section(name: &str, keys: &[&str]) -> LanguageSectionPlan {
        LanguageSectionPlan::Text {
            name: name.to_string(),
            logical_script_name: name.to_string(),
            origin: origin(),
            entries: keys
                .iter()
                .map(|key| LanguageTextEntryPlan {
                    key: (*key).to_string(),
                    value: String::new(),
                    origins: vec![LanguageTextOrigin::ScriptName],
                })
                .collect(),
        }
    }

    fn tooltip_section(name: &str, keys: &[&str]) -> LanguageSectionPlan {
        LanguageSectionPlan::Tooltip {
            name: name.to_string(),
            logical_script_name: name.to_string(),
            origin: origin(),
            entries: keys
                .iter()
                .map(|key| LanguageTooltipEntryPlan {
                    key: (*key).to_string(),
                    value: String::new(),
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

    fn parse(input: &str) -> Aul2Document {
        parse_aul2_document(input).unwrap()
    }

    fn entry_keys<'a>(document: &'a Aul2Document, section_name: &str) -> Vec<&'a str> {
        document
            .find_section(section_name)
            .unwrap()
            .body
            .iter()
            .filter_map(|node| match node {
                Aul2SectionNode::Entry(entry) => Some(entry.key.as_str()),
                Aul2SectionNode::Comment(_) | Aul2SectionNode::Blank(_) => None,
            })
            .collect()
    }

    #[test]
    fn removes_only_unused_entries_from_managed_section() {
        let mut document = parse("[managed]\nkeep=value\nold=value\nkeep-two=other\n");

        prune_aul2_document(
            &mut document,
            &plan(vec![text_section("managed", &["keep", "keep-two"])]),
        );

        assert_eq!(entry_keys(&document, "managed"), vec!["keep", "keep-two"]);
        assert_eq!(
            document.find_section("managed").unwrap().body[0],
            Aul2SectionNode::Entry(crate::aul2_document::Aul2Entry {
                key: "keep".to_string(),
                value_text: "value".to_string(),
                source_line: Some(2),
            })
        );
    }

    #[test]
    fn required_empty_entry_is_preserved() {
        let mut document = parse("[managed]\nrequired=\n");
        let original = document.clone();

        prune_aul2_document(
            &mut document,
            &plan(vec![text_section("managed", &["required"])]),
        );

        assert_eq!(document, original);
    }

    #[test]
    fn unknown_and_disabled_category_sections_are_unchanged() {
        let mut document = parse(
            "[managed]\nkeep=value\nold=value\n[Tips.managed]\ntip=\nold-tip=value\n[Unknown]\nold=value\n",
        );
        let unknown = document.find_section("Unknown").unwrap().clone();
        let disabled = document.find_section("Tips.managed").unwrap().clone();
        let mut file_plan = plan(vec![text_section("managed", &["keep"])]);
        file_plan.request.text = false;
        file_plan.request.tooltip = false;

        prune_aul2_document(&mut document, &file_plan);

        assert_eq!(entry_keys(&document, "managed"), vec!["keep"]);
        assert_eq!(document.find_section("Tips.managed"), Some(&disabled));
        assert_eq!(document.find_section("Unknown"), Some(&unknown));
    }

    #[test]
    fn empty_plan_leaves_the_entire_document_unchanged() {
        let mut document = parse("; preamble\n\n[first]\nold=value\n[second]\nkey=\n");
        let original = document.clone();

        prune_aul2_document(&mut document, &plan(Vec::new()));

        assert_eq!(document, original);
    }

    #[test]
    fn empty_plan_sections_do_not_manage_existing_sections() {
        let mut document = parse("[text]\nold=value\n[Tips.text]\nold-tip=value\n");
        let original = document.clone();

        prune_aul2_document(
            &mut document,
            &plan(vec![
                text_section("text", &[]),
                tooltip_section("Tips.text", &[]),
            ]),
        );

        assert_eq!(document, original);
    }

    #[test]
    fn comments_and_blanks_keep_raw_text_lines_and_order() {
        let mut document =
            parse("[managed]\nkeep=value\n; old note\nunused=value\n \t \nkeep-two=value\n");

        prune_aul2_document(
            &mut document,
            &plan(vec![text_section("managed", &["keep", "keep-two"])]),
        );

        assert_eq!(
            document.find_section("managed").unwrap().body,
            vec![
                Aul2SectionNode::Entry(crate::aul2_document::Aul2Entry {
                    key: "keep".to_string(),
                    value_text: "value".to_string(),
                    source_line: Some(2),
                }),
                Aul2SectionNode::Comment(Aul2Comment {
                    raw_text: "; old note".to_string(),
                    source_line: Some(3),
                }),
                Aul2SectionNode::Blank(Aul2Blank {
                    raw_text: " \t ".to_string(),
                    source_line: Some(5),
                }),
                Aul2SectionNode::Entry(crate::aul2_document::Aul2Entry {
                    key: "keep-two".to_string(),
                    value_text: "value".to_string(),
                    source_line: Some(6),
                }),
            ]
        );
    }

    #[test]
    fn section_header_and_non_entry_body_remain_when_all_entries_are_removed() {
        let mut document = parse("[managed]\n; keep comment\nold=value\n\t\n");

        prune_aul2_document(
            &mut document,
            &plan(vec![text_section("managed", &["missing"])]),
        );

        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].name, "managed");
        assert_eq!(document.sections[0].source_line, Some(1));
        assert_eq!(
            document.sections[0].body,
            vec![
                Aul2SectionNode::Comment(Aul2Comment {
                    raw_text: "; keep comment".to_string(),
                    source_line: Some(2),
                }),
                Aul2SectionNode::Blank(Aul2Blank {
                    raw_text: "\t".to_string(),
                    source_line: Some(4),
                }),
            ]
        );
    }

    #[test]
    fn multiple_sections_keep_existing_section_and_body_order() {
        let mut document = parse(
            "[second]\nold-second=value\nkeep-second=value\n[Unknown]\nx=value\n[first]\nkeep-first=value\nold-first=value\n",
        );

        prune_aul2_document(
            &mut document,
            &plan(vec![
                text_section("first", &["keep-first"]),
                text_section("second", &["keep-second"]),
            ]),
        );

        assert_eq!(
            document
                .sections
                .iter()
                .map(|section| section.name.as_str())
                .collect::<Vec<_>>(),
            vec!["second", "Unknown", "first"]
        );
        assert_eq!(entry_keys(&document, "second"), vec!["keep-second"]);
        assert_eq!(entry_keys(&document, "Unknown"), vec!["x"]);
        assert_eq!(entry_keys(&document, "first"), vec!["keep-first"]);
    }

    #[test]
    fn preamble_lines_source_lines_spacing_and_final_newline_are_preserved() {
        let mut document = parse("; preamble\n \n[managed]\nkeep=value\nold=value\n\n\t\n");
        let original_preamble = document.preamble.clone();

        prune_aul2_document(
            &mut document,
            &plan(vec![text_section("managed", &["keep"])]),
        );

        assert_eq!(document.preamble, original_preamble);
        assert!(document.has_trailing_newline);
        assert_eq!(
            document.find_section("managed").unwrap().body,
            vec![
                Aul2SectionNode::Entry(crate::aul2_document::Aul2Entry {
                    key: "keep".to_string(),
                    value_text: "value".to_string(),
                    source_line: Some(4),
                }),
                Aul2SectionNode::Blank(Aul2Blank {
                    raw_text: String::new(),
                    source_line: Some(6),
                }),
                Aul2SectionNode::Blank(Aul2Blank {
                    raw_text: "\t".to_string(),
                    source_line: Some(7),
                }),
            ]
        );
        assert_eq!(
            document.preamble,
            vec![
                Aul2PreambleNode::Comment(Aul2Comment {
                    raw_text: "; preamble".to_string(),
                    source_line: Some(1),
                }),
                Aul2PreambleNode::Blank(Aul2Blank {
                    raw_text: " ".to_string(),
                    source_line: Some(2),
                }),
            ]
        );
    }

    #[test]
    fn unused_findings_disappear_while_missing_and_empty_remain_and_prune_is_idempotent() {
        let mut document = parse("[managed]\nempty=\nold=value\n");
        let file_plan = plan(vec![text_section("managed", &["missing", "empty"])]);

        assert_eq!(
            check_aul2_document(&document, &file_plan),
            vec![
                LanguageCheckFinding::MissingKey {
                    section_name: "managed".to_string(),
                    key: "missing".to_string(),
                },
                LanguageCheckFinding::EmptyValue {
                    section_name: "managed".to_string(),
                    key: "empty".to_string(),
                    source_line: Some(2),
                },
                LanguageCheckFinding::UnusedKey {
                    section_name: "managed".to_string(),
                    key: "old".to_string(),
                    source_line: Some(3),
                },
            ]
        );

        prune_aul2_document(&mut document, &file_plan);
        assert_eq!(
            check_aul2_document(&document, &file_plan),
            vec![
                LanguageCheckFinding::MissingKey {
                    section_name: "managed".to_string(),
                    key: "missing".to_string(),
                },
                LanguageCheckFinding::EmptyValue {
                    section_name: "managed".to_string(),
                    key: "empty".to_string(),
                    source_line: Some(2),
                },
            ]
        );

        let once_pruned = document.clone();
        prune_aul2_document(&mut document, &file_plan);
        assert_eq!(document, once_pruned);
    }
}
