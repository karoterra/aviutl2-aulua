use std::collections::HashMap;

use thiserror::Error;

use crate::language_script_analysis::{AnalyzedLogicalScript, LogicalScriptOrigin};
use crate::language_script_entries::{LanguageTextEntry, LanguageTipsEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguageSectionCategory {
    Text,
    Tooltip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LanguageSectionCandidate {
    Text {
        name: String,
        logical_script_name: String,
        origin: LogicalScriptOrigin,
        entries: Vec<LanguageTextEntry>,
    },
    Tooltip {
        name: String,
        logical_script_name: String,
        origin: LogicalScriptOrigin,
        entries: Vec<LanguageTipsEntry>,
    },
}

impl LanguageSectionCandidate {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Text { name, .. } | Self::Tooltip { name, .. } => name,
        }
    }

    pub(crate) fn logical_script_name(&self) -> &str {
        match self {
            Self::Text {
                logical_script_name,
                ..
            }
            | Self::Tooltip {
                logical_script_name,
                ..
            } => logical_script_name,
        }
    }

    pub(crate) fn origin(&self) -> &LogicalScriptOrigin {
        match self {
            Self::Text { origin, .. } | Self::Tooltip { origin, .. } => origin,
        }
    }

    pub(crate) fn category(&self) -> LanguageSectionCategory {
        match self {
            Self::Text { .. } => LanguageSectionCategory::Text,
            Self::Tooltip { .. } => LanguageSectionCategory::Tooltip,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageSectionCatalog {
    pub sections: Vec<LanguageSectionCandidate>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum BuildLanguageSectionCatalogError {
    #[error(
        "language section名 {section_name} が重複しています: {first_logical_script_name} ({first_category:?}) / {duplicate_logical_script_name} ({duplicate_category:?})"
    )]
    DuplicateSectionName {
        section_name: String,
        first_logical_script_name: String,
        first_category: LanguageSectionCategory,
        first_origin: Box<LogicalScriptOrigin>,
        duplicate_logical_script_name: String,
        duplicate_category: LanguageSectionCategory,
        duplicate_origin: Box<LogicalScriptOrigin>,
    },
}

pub(crate) fn build_language_section_catalog(
    scripts: &[AnalyzedLogicalScript],
) -> Result<LanguageSectionCatalog, BuildLanguageSectionCatalogError> {
    let mut sections = Vec::new();
    let mut section_indices = HashMap::new();

    for script in scripts {
        let script_name = &script.prepared.name;
        let origin = &script.prepared.origin;

        if !script.language.entries.text_entries.is_empty() {
            push_unique_section(
                &mut sections,
                &mut section_indices,
                LanguageSectionCandidate::Text {
                    name: script_name.clone(),
                    logical_script_name: script_name.clone(),
                    origin: origin.clone(),
                    entries: script.language.entries.text_entries.clone(),
                },
            )?;
        }

        if !script.language.entries.tips_entries.is_empty() {
            push_unique_section(
                &mut sections,
                &mut section_indices,
                LanguageSectionCandidate::Tooltip {
                    name: format!("Tips.{script_name}"),
                    logical_script_name: script_name.clone(),
                    origin: origin.clone(),
                    entries: script.language.entries.tips_entries.clone(),
                },
            )?;
        }
    }

    Ok(LanguageSectionCatalog { sections })
}

fn push_unique_section(
    sections: &mut Vec<LanguageSectionCandidate>,
    section_indices: &mut HashMap<String, usize>,
    candidate: LanguageSectionCandidate,
) -> Result<(), BuildLanguageSectionCatalogError> {
    if let Some(&first_index) = section_indices.get(candidate.name()) {
        let first = &sections[first_index];
        return Err(BuildLanguageSectionCatalogError::DuplicateSectionName {
            section_name: candidate.name().to_string(),
            first_logical_script_name: first.logical_script_name().to_string(),
            first_category: first.category(),
            first_origin: Box::new(first.origin().clone()),
            duplicate_logical_script_name: candidate.logical_script_name().to_string(),
            duplicate_category: candidate.category(),
            duplicate_origin: Box::new(candidate.origin().clone()),
        });
    }

    let index = sections.len();
    section_indices.insert(candidate.name().to_string(), index);
    sections.push(candidate);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::language_script_analysis::{LanguageAnalysisWarning, PreparedLogicalScript};
    use crate::language_script_entries::{
        LanguageScriptEntries, LanguageTextOrigin, LanguageTipsOrigin,
    };
    use crate::language_script_info::{LanguageScriptInfo, LanguageText};
    use crate::language_ui::{LanguageUiKind, SourceSpan};
    use crate::logical_script_language::LogicalScriptLanguage;

    fn direct(path: &str) -> LogicalScriptOrigin {
        LogicalScriptOrigin::Direct {
            path: PathBuf::from(path),
        }
    }

    fn configured(paths: &[&str]) -> LogicalScriptOrigin {
        LogicalScriptOrigin::Configured {
            source_paths: paths.iter().map(PathBuf::from).collect(),
        }
    }

    fn text_entry(key: &str, origins: Vec<LanguageTextOrigin>) -> LanguageTextEntry {
        LanguageTextEntry {
            key: key.to_string(),
            origins,
        }
    }

    fn tips_entry(key: &str, value: &str, origin: LanguageTipsOrigin) -> LanguageTipsEntry {
        LanguageTipsEntry {
            key: key.to_string(),
            value: value.to_string(),
            origin,
        }
    }

    fn analyzed(
        name: &str,
        origin: LogicalScriptOrigin,
        text_entries: Vec<LanguageTextEntry>,
        tips_entries: Vec<LanguageTipsEntry>,
    ) -> AnalyzedLogicalScript {
        AnalyzedLogicalScript {
            prepared: PreparedLogicalScript {
                name: name.to_string(),
                body: format!("body of {name}"),
                origin,
                warnings: vec![LanguageAnalysisWarning::UndefinedVariable {
                    source_path: PathBuf::from("warning.lua"),
                    variable_name: "MISSING".to_string(),
                }],
            },
            language: LogicalScriptLanguage {
                info: LanguageScriptInfo {
                    script_name: LanguageText {
                        value: name.to_string(),
                        enabled: false,
                    },
                    script_tips: None,
                    ui_items: Vec::new(),
                },
                entries: LanguageScriptEntries {
                    text_entries,
                    tips_entries,
                },
            },
        }
    }

    fn script_name_text(key: &str) -> LanguageTextEntry {
        text_entry(key, vec![LanguageTextOrigin::ScriptName])
    }

    fn script_tips(value: &str, line: usize) -> LanguageTipsEntry {
        tips_entry(
            "effect.name",
            value,
            LanguageTipsOrigin::ScriptTips {
                span: SourceSpan {
                    start_line: line,
                    end_line: line,
                },
            },
        )
    }

    #[test]
    fn empty_input_builds_an_empty_catalog() {
        assert_eq!(
            build_language_section_catalog(&[]).unwrap(),
            LanguageSectionCatalog {
                sections: Vec::new(),
            }
        );
    }

    #[test]
    fn builds_non_empty_sections_in_script_text_tooltip_order() {
        let scripts = vec![
            analyzed(
                "both",
                direct("both.anm2"),
                vec![script_name_text("both")],
                vec![script_tips("Both tips", 1)],
            ),
            analyzed(
                "text-only",
                direct("text.anm2"),
                vec![script_name_text("text-only")],
                Vec::new(),
            ),
            analyzed(
                "tooltip-only",
                direct("tooltip.anm2"),
                Vec::new(),
                vec![script_tips("Tooltip only", 2)],
            ),
            analyzed("empty", direct("empty.anm2"), Vec::new(), Vec::new()),
        ];

        let catalog = build_language_section_catalog(&scripts).unwrap();

        assert_eq!(
            catalog
                .sections
                .iter()
                .map(|section| (section.name(), section.category()))
                .collect::<Vec<_>>(),
            vec![
                ("both", LanguageSectionCategory::Text),
                ("Tips.both", LanguageSectionCategory::Tooltip),
                ("text-only", LanguageSectionCategory::Text),
                ("Tips.tooltip-only", LanguageSectionCategory::Tooltip),
            ]
        );
    }

    #[test]
    fn uses_completed_entry_lists_without_rechecking_language_info() {
        let ui_span = SourceSpan {
            start_line: 3,
            end_line: 4,
        };
        let script = analyzed(
            "disabled-info",
            direct("disabled.anm2"),
            vec![text_entry(
                "UI text",
                vec![LanguageTextOrigin::UiName {
                    kind: LanguageUiKind::Check,
                    ui_span,
                }],
            )],
            vec![tips_entry(
                "Full UI name",
                "UI tips",
                LanguageTipsOrigin::UiTips {
                    kind: LanguageUiKind::Check,
                    ui_span,
                    tips_span: SourceSpan {
                        start_line: 1,
                        end_line: 1,
                    },
                },
            )],
        );

        let catalog = build_language_section_catalog(&[script]).unwrap();

        assert_eq!(catalog.sections.len(), 2);
        assert_eq!(catalog.sections[0].name(), "disabled-info");
        assert_eq!(catalog.sections[1].name(), "Tips.disabled-info");
    }

    #[test]
    fn preserves_entries_origins_and_unescaped_multiline_tips() {
        let first_span = SourceSpan {
            start_line: 2,
            end_line: 3,
        };
        let second_span = SourceSpan {
            start_line: 5,
            end_line: 6,
        };
        let text_entries = vec![
            text_entry(
                "shared",
                vec![
                    LanguageTextOrigin::ScriptName,
                    LanguageTextOrigin::UiName {
                        kind: LanguageUiKind::Track,
                        ui_span: first_span,
                    },
                ],
            ),
            text_entry(
                "option",
                vec![LanguageTextOrigin::SelectOption {
                    ui_span: second_span,
                    option_index: 2,
                }],
            ),
        ];
        let tips_entries = vec![tips_entry(
            "full::name",
            "first line\nsecond line",
            LanguageTipsOrigin::UiTips {
                kind: LanguageUiKind::Select,
                ui_span: second_span,
                tips_span: SourceSpan {
                    start_line: 4,
                    end_line: 4,
                },
            },
        )];
        let origin = configured(&["first.lua", "second.lua"]);
        let script = analyzed(
            "catalog",
            origin.clone(),
            text_entries.clone(),
            tips_entries.clone(),
        );

        let catalog = build_language_section_catalog(&[script]).unwrap();

        assert_eq!(
            catalog.sections,
            vec![
                LanguageSectionCandidate::Text {
                    name: "catalog".to_string(),
                    logical_script_name: "catalog".to_string(),
                    origin: origin.clone(),
                    entries: text_entries,
                },
                LanguageSectionCandidate::Tooltip {
                    name: "Tips.catalog".to_string(),
                    logical_script_name: "catalog".to_string(),
                    origin,
                    entries: tips_entries,
                },
            ]
        );
    }

    #[test]
    fn reports_text_after_tooltip_section_name_collision() {
        let first_origin = configured(&["foo.lua"]);
        let duplicate_origin = direct("tips_foo.anm2");
        let scripts = vec![
            analyzed(
                "foo",
                first_origin.clone(),
                Vec::new(),
                vec![script_tips("tips", 1)],
            ),
            analyzed(
                "Tips.foo",
                duplicate_origin.clone(),
                vec![script_name_text("Tips.foo")],
                Vec::new(),
            ),
        ];

        assert_eq!(
            build_language_section_catalog(&scripts).unwrap_err(),
            BuildLanguageSectionCatalogError::DuplicateSectionName {
                section_name: "Tips.foo".to_string(),
                first_logical_script_name: "foo".to_string(),
                first_category: LanguageSectionCategory::Tooltip,
                first_origin: Box::new(first_origin),
                duplicate_logical_script_name: "Tips.foo".to_string(),
                duplicate_category: LanguageSectionCategory::Text,
                duplicate_origin: Box::new(duplicate_origin),
            }
        );
    }

    #[test]
    fn reports_tooltip_after_text_section_name_collision() {
        let first_origin = direct("tips_foo.anm2");
        let duplicate_origin = configured(&["foo.lua"]);
        let scripts = vec![
            analyzed(
                "Tips.foo",
                first_origin.clone(),
                vec![script_name_text("Tips.foo")],
                Vec::new(),
            ),
            analyzed(
                "foo",
                duplicate_origin.clone(),
                Vec::new(),
                vec![script_tips("tips", 1)],
            ),
        ];

        assert_eq!(
            build_language_section_catalog(&scripts).unwrap_err(),
            BuildLanguageSectionCatalogError::DuplicateSectionName {
                section_name: "Tips.foo".to_string(),
                first_logical_script_name: "Tips.foo".to_string(),
                first_category: LanguageSectionCategory::Text,
                first_origin: Box::new(first_origin),
                duplicate_logical_script_name: "foo".to_string(),
                duplicate_category: LanguageSectionCategory::Tooltip,
                duplicate_origin: Box::new(duplicate_origin),
            }
        );
    }

    #[test]
    fn section_name_comparison_is_exact_without_case_trim_or_unicode_normalization() {
        let scripts = vec![
            analyzed(
                "Name",
                direct("upper.anm2"),
                vec![script_name_text("Name")],
                Vec::new(),
            ),
            analyzed(
                "name",
                direct("lower.anm2"),
                vec![script_name_text("name")],
                Vec::new(),
            ),
            analyzed(
                " name",
                direct("space.anm2"),
                vec![script_name_text(" name")],
                Vec::new(),
            ),
            analyzed(
                "\u{00e9}",
                direct("composed.anm2"),
                vec![script_name_text("\u{00e9}")],
                Vec::new(),
            ),
            analyzed(
                "e\u{0301}",
                direct("decomposed.anm2"),
                vec![script_name_text("e\u{0301}")],
                Vec::new(),
            ),
        ];

        let catalog = build_language_section_catalog(&scripts).unwrap();

        assert_eq!(catalog.sections.len(), scripts.len());
    }

    #[test]
    fn fails_on_the_first_collision_in_generation_order() {
        let scripts = vec![
            analyzed(
                "foo",
                direct("foo.anm2"),
                Vec::new(),
                vec![script_tips("foo tips", 1)],
            ),
            analyzed(
                "Tips.foo",
                direct("tips_foo.anm2"),
                vec![script_name_text("Tips.foo")],
                Vec::new(),
            ),
            analyzed(
                "bar",
                direct("bar.anm2"),
                Vec::new(),
                vec![script_tips("bar tips", 1)],
            ),
            analyzed(
                "Tips.bar",
                direct("tips_bar.anm2"),
                vec![script_name_text("Tips.bar")],
                Vec::new(),
            ),
        ];

        let error = build_language_section_catalog(&scripts).unwrap_err();

        assert!(matches!(
            error,
            BuildLanguageSectionCatalogError::DuplicateSectionName { section_name, .. }
                if section_name == "Tips.foo"
        ));
    }

    #[test]
    fn defensively_detects_same_category_collisions() {
        let text_scripts = vec![
            analyzed(
                "same",
                direct("first.anm2"),
                vec![script_name_text("same")],
                Vec::new(),
            ),
            analyzed(
                "same",
                direct("second.anm2"),
                vec![script_name_text("same")],
                Vec::new(),
            ),
        ];
        let tooltip_scripts = vec![
            analyzed(
                "same",
                direct("first.anm2"),
                Vec::new(),
                vec![script_tips("first", 1)],
            ),
            analyzed(
                "same",
                direct("second.anm2"),
                Vec::new(),
                vec![script_tips("second", 1)],
            ),
        ];

        assert!(matches!(
            build_language_section_catalog(&text_scripts).unwrap_err(),
            BuildLanguageSectionCatalogError::DuplicateSectionName {
                first_category: LanguageSectionCategory::Text,
                duplicate_category: LanguageSectionCategory::Text,
                ..
            }
        ));
        assert!(matches!(
            build_language_section_catalog(&tooltip_scripts).unwrap_err(),
            BuildLanguageSectionCatalogError::DuplicateSectionName {
                first_category: LanguageSectionCategory::Tooltip,
                duplicate_category: LanguageSectionCategory::Tooltip,
                ..
            }
        ));
    }
}
