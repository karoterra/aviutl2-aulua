use std::collections::HashSet;

use crate::aul2_document::{Aul2Document, Aul2SectionNode};
use crate::language_file_plan::{LanguageFilePlan, LanguageSectionPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LanguageCheckFinding {
    MissingSection {
        section_name: String,
    },
    MissingKey {
        section_name: String,
        key: String,
    },
    UnusedKey {
        section_name: String,
        key: String,
        source_line: Option<usize>,
    },
    EmptyValue {
        section_name: String,
        key: String,
        source_line: Option<usize>,
    },
}

pub(crate) fn check_aul2_document(
    document: &Aul2Document,
    plan: &LanguageFilePlan,
) -> Vec<LanguageCheckFinding> {
    let mut findings = Vec::new();

    for section_plan in &plan.sections {
        let plan_keys = section_entry_keys(section_plan);
        if plan_keys.is_empty() {
            continue;
        }

        let section_name = section_plan.name();
        let Some(section) = document.find_section(section_name) else {
            findings.push(LanguageCheckFinding::MissingSection {
                section_name: section_name.to_string(),
            });
            continue;
        };

        for key in &plan_keys {
            match section.find_entry(key) {
                None => findings.push(LanguageCheckFinding::MissingKey {
                    section_name: section_name.to_string(),
                    key: (*key).to_string(),
                }),
                Some(entry) if entry.value_text.is_empty() => {
                    findings.push(LanguageCheckFinding::EmptyValue {
                        section_name: section_name.to_string(),
                        key: (*key).to_string(),
                        source_line: entry.source_line,
                    });
                }
                Some(_) => {}
            }
        }

        let plan_key_set = plan_keys.iter().copied().collect::<HashSet<_>>();
        for node in &section.body {
            let Aul2SectionNode::Entry(entry) = node else {
                continue;
            };
            if !plan_key_set.contains(entry.key.as_str()) {
                findings.push(LanguageCheckFinding::UnusedKey {
                    section_name: section_name.to_string(),
                    key: entry.key.clone(),
                    source_line: entry.source_line,
                });
            }
        }
    }

    findings
}

fn section_entry_keys(section: &LanguageSectionPlan) -> Vec<&str> {
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
    use crate::aul2_document::{Aul2Entry, Aul2Section};
    use crate::aul2_parser::parse_aul2_document;
    use crate::language_file_plan::{LanguageTextEntryPlan, LanguageTooltipEntryPlan};
    use crate::language_file_request::{LanguageFileRequest, LanguageFileRequestOrigin};
    use crate::language_script_analysis::LogicalScriptOrigin;
    use crate::language_script_entries::{LanguageTextOrigin, LanguageTipsOrigin};
    use crate::language_ui::SourceSpan;

    fn plan(is_default: bool, sections: Vec<LanguageSectionPlan>) -> LanguageFilePlan {
        LanguageFilePlan {
            request: LanguageFileRequest {
                path: PathBuf::from("Language/Test.aul2"),
                text: true,
                tooltip: true,
                is_default,
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
                    value: format!("expected-{key}"),
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
                    value: format!("expected-{key}"),
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

    #[test]
    fn matching_managed_sections_have_no_findings() {
        let document = parse("[text]\na=value\nb=value\n[Tips.text]\ntip=value\n");
        let plan = plan(
            false,
            vec![
                text_section("text", &["a", "b"]),
                tooltip_section("Tips.text", &["tip"]),
            ],
        );

        assert!(check_aul2_document(&document, &plan).is_empty());
    }

    #[test]
    fn missing_section_is_reported_once_without_missing_keys() {
        let findings = check_aul2_document(
            &parse(""),
            &plan(false, vec![text_section("missing", &["a", "b"])]),
        );

        assert_eq!(
            findings,
            vec![LanguageCheckFinding::MissingSection {
                section_name: "missing".to_string(),
            }]
        );
    }

    #[test]
    fn missing_key_is_reported_without_empty_value() {
        let findings = check_aul2_document(
            &parse("[section]\npresent=value\n"),
            &plan(
                false,
                vec![text_section("section", &["present", "missing"])],
            ),
        );

        assert_eq!(
            findings,
            vec![LanguageCheckFinding::MissingKey {
                section_name: "section".to_string(),
                key: "missing".to_string(),
            }]
        );
    }

    #[test]
    fn required_empty_value_preserves_existing_source_line() {
        let findings = check_aul2_document(
            &parse("; preamble\n[section]\nkey=\n"),
            &plan(false, vec![text_section("section", &["key"])]),
        );

        assert_eq!(
            findings,
            vec![LanguageCheckFinding::EmptyValue {
                section_name: "section".to_string(),
                key: "key".to_string(),
                source_line: Some(3),
            }]
        );
    }

    #[test]
    fn spaces_tabs_and_literal_newline_escape_are_not_empty() {
        let document = parse("[section]\nspace= \ntab=\t\nescape=literal\\nvalue\n");
        let file_plan = plan(
            false,
            vec![text_section("section", &["space", "tab", "escape"])],
        );

        assert!(check_aul2_document(&document, &file_plan).is_empty());
    }

    #[test]
    fn unused_key_preserves_body_order_and_source_lines() {
        let findings = check_aul2_document(
            &parse("[section]\nrequired=value\n; keep\nold-a=value\n\nold-b=value\n"),
            &plan(false, vec![text_section("section", &["required"])]),
        );

        assert_eq!(
            findings,
            vec![
                LanguageCheckFinding::UnusedKey {
                    section_name: "section".to_string(),
                    key: "old-a".to_string(),
                    source_line: Some(4),
                },
                LanguageCheckFinding::UnusedKey {
                    section_name: "section".to_string(),
                    key: "old-b".to_string(),
                    source_line: Some(6),
                },
            ]
        );
    }

    #[test]
    fn unused_empty_key_only_reports_unused() {
        let findings = check_aul2_document(
            &parse("[section]\nrequired=value\nold=\n"),
            &plan(false, vec![text_section("section", &["required"])]),
        );

        assert_eq!(
            findings,
            vec![LanguageCheckFinding::UnusedKey {
                section_name: "section".to_string(),
                key: "old".to_string(),
                source_line: Some(3),
            }]
        );
    }

    #[test]
    fn unknown_and_disabled_category_sections_are_ignored() {
        let document = parse("[text]\nkey=value\n[Tips.text]\nempty=\nold=unused\n[Unknown]\nx=\n");
        let mut file_plan = plan(false, vec![text_section("text", &["key"])]);
        file_plan.request.tooltip = false;

        assert!(check_aul2_document(&document, &file_plan).is_empty());
    }

    #[test]
    fn empty_plan_and_empty_plan_sections_manage_nothing() {
        let document = parse("[existing]\nkey=\n[empty]\nold=\n");

        assert!(check_aul2_document(&document, &plan(false, Vec::new())).is_empty());
        assert!(
            check_aul2_document(
                &document,
                &plan(
                    false,
                    vec![text_section("empty", &[]), tooltip_section("tips", &[])],
                ),
            )
            .is_empty()
        );
    }

    #[test]
    fn default_nonempty_value_mismatch_is_not_checked() {
        let document = parse("[section]\nkey=existing-different-value\n");
        let file_plan = plan(true, vec![text_section("section", &["key"])]);

        assert!(check_aul2_document(&document, &file_plan).is_empty());
    }

    #[test]
    fn findings_follow_plan_sections_then_required_and_existing_entry_order() {
        let document =
            parse("[C]\nrequired=value\nold-c=value\n[A]\nempty=\nold-a=value\n[Unknown]\nx=\n");
        let file_plan = plan(
            false,
            vec![
                text_section("A", &["missing", "empty"]),
                text_section("B", &["b1", "b2"]),
                text_section("C", &["required"]),
            ],
        );

        assert_eq!(
            check_aul2_document(&document, &file_plan),
            vec![
                LanguageCheckFinding::MissingKey {
                    section_name: "A".to_string(),
                    key: "missing".to_string(),
                },
                LanguageCheckFinding::EmptyValue {
                    section_name: "A".to_string(),
                    key: "empty".to_string(),
                    source_line: Some(5),
                },
                LanguageCheckFinding::UnusedKey {
                    section_name: "A".to_string(),
                    key: "old-a".to_string(),
                    source_line: Some(6),
                },
                LanguageCheckFinding::MissingSection {
                    section_name: "B".to_string(),
                },
                LanguageCheckFinding::UnusedKey {
                    section_name: "C".to_string(),
                    key: "old-c".to_string(),
                    source_line: Some(3),
                },
            ]
        );
    }

    #[test]
    fn generated_existing_entries_preserve_none_source_line() {
        let document = Aul2Document {
            preamble: Vec::new(),
            sections: vec![Aul2Section {
                name: "section".to_string(),
                source_line: None,
                body: vec![
                    Aul2SectionNode::Entry(Aul2Entry {
                        key: "empty".to_string(),
                        value_text: String::new(),
                        source_line: None,
                    }),
                    Aul2SectionNode::Entry(Aul2Entry {
                        key: "old".to_string(),
                        value_text: "value".to_string(),
                        source_line: None,
                    }),
                ],
            }],
            has_trailing_newline: true,
        };
        let file_plan = plan(false, vec![text_section("section", &["empty"])]);

        assert_eq!(
            check_aul2_document(&document, &file_plan),
            vec![
                LanguageCheckFinding::EmptyValue {
                    section_name: "section".to_string(),
                    key: "empty".to_string(),
                    source_line: None,
                },
                LanguageCheckFinding::UnusedKey {
                    section_name: "section".to_string(),
                    key: "old".to_string(),
                    source_line: None,
                },
            ]
        );
    }
}
