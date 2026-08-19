use crate::language_file_request::LanguageFileRequest;
use crate::language_script_analysis::LogicalScriptOrigin;
use crate::language_script_entries::{LanguageTextOrigin, LanguageTipsOrigin};
use crate::language_section_catalog::{
    LanguageSectionCandidate, LanguageSectionCatalog, LanguageSectionCategory,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageFilePlan {
    pub request: LanguageFileRequest,
    pub sections: Vec<LanguageSectionPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LanguageSectionPlan {
    Text {
        name: String,
        logical_script_name: String,
        origin: LogicalScriptOrigin,
        entries: Vec<LanguageTextEntryPlan>,
    },
    Tooltip {
        name: String,
        logical_script_name: String,
        origin: LogicalScriptOrigin,
        entries: Vec<LanguageTooltipEntryPlan>,
    },
}

impl LanguageSectionPlan {
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
pub(crate) struct LanguageTextEntryPlan {
    pub key: String,
    pub value: String,
    pub origins: Vec<LanguageTextOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageTooltipEntryPlan {
    pub key: String,
    pub value: String,
    pub origin: LanguageTipsOrigin,
}

pub(crate) fn build_language_file_plans(
    requests: &[LanguageFileRequest],
    catalog: &LanguageSectionCatalog,
) -> Vec<LanguageFilePlan> {
    requests
        .iter()
        .map(|request| build_language_file_plan(request, catalog))
        .collect()
}

pub(crate) fn build_language_file_plan(
    request: &LanguageFileRequest,
    catalog: &LanguageSectionCatalog,
) -> LanguageFilePlan {
    LanguageFilePlan {
        request: request.clone(),
        sections: catalog
            .sections
            .iter()
            .filter_map(|candidate| build_section_plan(request, candidate))
            .collect(),
    }
}

fn build_section_plan(
    request: &LanguageFileRequest,
    candidate: &LanguageSectionCandidate,
) -> Option<LanguageSectionPlan> {
    match candidate {
        LanguageSectionCandidate::Text {
            name,
            logical_script_name,
            origin,
            entries,
        } if request.text && !entries.is_empty() => Some(LanguageSectionPlan::Text {
            name: name.clone(),
            logical_script_name: logical_script_name.clone(),
            origin: origin.clone(),
            entries: entries
                .iter()
                .map(|entry| LanguageTextEntryPlan {
                    key: entry.key.clone(),
                    value: if request.is_default {
                        entry.key.clone()
                    } else {
                        String::new()
                    },
                    origins: entry.origins.clone(),
                })
                .collect(),
        }),
        LanguageSectionCandidate::Tooltip {
            name,
            logical_script_name,
            origin,
            entries,
        } if request.tooltip && !entries.is_empty() => Some(LanguageSectionPlan::Tooltip {
            name: name.clone(),
            logical_script_name: logical_script_name.clone(),
            origin: origin.clone(),
            entries: entries
                .iter()
                .map(|entry| LanguageTooltipEntryPlan {
                    key: entry.key.clone(),
                    value: if request.is_default {
                        entry.value.clone()
                    } else {
                        String::new()
                    },
                    origin: entry.origin.clone(),
                })
                .collect(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::language_file_request::LanguageFileRequestOrigin;
    use crate::language_script_entries::{
        LanguageTextEntry, LanguageTipsEntry, LanguageTipsOrigin,
    };
    use crate::language_ui::{LanguageUiKind, SourceSpan};

    fn span(start_line: usize, end_line: usize) -> SourceSpan {
        SourceSpan {
            start_line,
            end_line,
        }
    }

    fn configured_origin(paths: &[&str]) -> LogicalScriptOrigin {
        LogicalScriptOrigin::Configured {
            source_paths: paths.iter().map(PathBuf::from).collect(),
        }
    }

    fn direct_origin(path: &str) -> LogicalScriptOrigin {
        LogicalScriptOrigin::Direct {
            path: PathBuf::from(path),
        }
    }

    fn text_entry(key: &str, origins: Vec<LanguageTextOrigin>) -> LanguageTextEntry {
        LanguageTextEntry {
            key: key.to_string(),
            origins,
        }
    }

    fn tooltip_entry(key: &str, value: &str, origin: LanguageTipsOrigin) -> LanguageTipsEntry {
        LanguageTipsEntry {
            key: key.to_string(),
            value: value.to_string(),
            origin,
        }
    }

    fn text_candidate(
        name: &str,
        logical_script_name: &str,
        origin: LogicalScriptOrigin,
        entries: Vec<LanguageTextEntry>,
    ) -> LanguageSectionCandidate {
        LanguageSectionCandidate::Text {
            name: name.to_string(),
            logical_script_name: logical_script_name.to_string(),
            origin,
            entries,
        }
    }

    fn tooltip_candidate(
        name: &str,
        logical_script_name: &str,
        origin: LogicalScriptOrigin,
        entries: Vec<LanguageTipsEntry>,
    ) -> LanguageSectionCandidate {
        LanguageSectionCandidate::Tooltip {
            name: name.to_string(),
            logical_script_name: logical_script_name.to_string(),
            origin,
            entries,
        }
    }

    fn request(
        path: &str,
        text: bool,
        tooltip: bool,
        is_default: bool,
        origin: LanguageFileRequestOrigin,
    ) -> LanguageFileRequest {
        LanguageFileRequest {
            path: PathBuf::from(path),
            text,
            tooltip,
            is_default,
            origin,
        }
    }

    fn catalog_with_all_entry_origins() -> LanguageSectionCatalog {
        let ui_span = span(3, 4);
        LanguageSectionCatalog {
            sections: vec![
                text_candidate(
                    "script",
                    "script",
                    configured_origin(&["first.lua", "second.lua"]),
                    vec![
                        text_entry(
                            "shared",
                            vec![
                                LanguageTextOrigin::ScriptName,
                                LanguageTextOrigin::UiName {
                                    kind: LanguageUiKind::Check,
                                    ui_span,
                                },
                            ],
                        ),
                        text_entry(
                            "option",
                            vec![LanguageTextOrigin::SelectOption {
                                ui_span: span(6, 8),
                                option_index: 2,
                            }],
                        ),
                    ],
                ),
                tooltip_candidate(
                    "Tips.script",
                    "script",
                    direct_origin("script.anm2"),
                    vec![tooltip_entry(
                        "full::name",
                        "first line\nsecond line",
                        LanguageTipsOrigin::UiTips {
                            kind: LanguageUiKind::Check,
                            ui_span,
                            tips_span: span(1, 2),
                        },
                    )],
                ),
            ],
        }
    }

    #[test]
    fn normal_file_uses_empty_values_and_preserves_entries_and_origins() {
        let catalog = catalog_with_all_entry_origins();
        let request = request(
            "English.aul2",
            true,
            true,
            false,
            LanguageFileRequestOrigin::Configured { index: 1 },
        );

        let plans = build_language_file_plans(std::slice::from_ref(&request), &catalog);

        assert_eq!(plans[0].request, request);
        assert_eq!(
            plans[0].sections,
            vec![
                LanguageSectionPlan::Text {
                    name: "script".to_string(),
                    logical_script_name: "script".to_string(),
                    origin: configured_origin(&["first.lua", "second.lua"]),
                    entries: vec![
                        LanguageTextEntryPlan {
                            key: "shared".to_string(),
                            value: String::new(),
                            origins: vec![
                                LanguageTextOrigin::ScriptName,
                                LanguageTextOrigin::UiName {
                                    kind: LanguageUiKind::Check,
                                    ui_span: span(3, 4),
                                },
                            ],
                        },
                        LanguageTextEntryPlan {
                            key: "option".to_string(),
                            value: String::new(),
                            origins: vec![LanguageTextOrigin::SelectOption {
                                ui_span: span(6, 8),
                                option_index: 2,
                            }],
                        },
                    ],
                },
                LanguageSectionPlan::Tooltip {
                    name: "Tips.script".to_string(),
                    logical_script_name: "script".to_string(),
                    origin: direct_origin("script.anm2"),
                    entries: vec![LanguageTooltipEntryPlan {
                        key: "full::name".to_string(),
                        value: String::new(),
                        origin: LanguageTipsOrigin::UiTips {
                            kind: LanguageUiKind::Check,
                            ui_span: span(3, 4),
                            tips_span: span(1, 2),
                        },
                    }],
                },
            ]
        );
    }

    #[test]
    fn default_file_uses_keys_and_raw_multiline_tips_as_values() {
        let catalog = catalog_with_all_entry_origins();
        let request = request(
            "Default.aul2",
            true,
            true,
            true,
            LanguageFileRequestOrigin::Override,
        );

        let plans = build_language_file_plans(&[request], &catalog);

        let LanguageSectionPlan::Text { entries, .. } = &plans[0].sections[0] else {
            panic!("Text sectionではありません");
        };
        assert_eq!(
            entries
                .iter()
                .map(|entry| (entry.key.as_str(), entry.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("shared", "shared"), ("option", "option")]
        );

        let LanguageSectionPlan::Tooltip { entries, .. } = &plans[0].sections[1] else {
            panic!("Tooltip sectionではありません");
        };
        assert_eq!(entries[0].key, "full::name");
        assert_eq!(entries[0].value, "first line\nsecond line");
        assert!(!entries[0].value.contains("\\n"));
        assert_eq!(
            entries[0].origin,
            LanguageTipsOrigin::UiTips {
                kind: LanguageUiKind::Check,
                ui_span: span(3, 4),
                tips_span: span(1, 2),
            }
        );
    }

    #[test]
    fn filters_categories_and_preserves_request_and_section_order() {
        let catalog = LanguageSectionCatalog {
            sections: vec![
                text_candidate(
                    "first",
                    "first",
                    direct_origin("first.anm2"),
                    vec![text_entry("first", vec![LanguageTextOrigin::ScriptName])],
                ),
                tooltip_candidate(
                    "Tips.first",
                    "first",
                    direct_origin("first.anm2"),
                    vec![tooltip_entry(
                        "effect.name",
                        "First tips",
                        LanguageTipsOrigin::ScriptTips { span: span(1, 1) },
                    )],
                ),
                text_candidate(
                    "second",
                    "second",
                    direct_origin("second.anm2"),
                    vec![text_entry("second", vec![LanguageTextOrigin::ScriptName])],
                ),
                tooltip_candidate(
                    "Tips.second",
                    "second",
                    direct_origin("second.anm2"),
                    vec![tooltip_entry(
                        "effect.name",
                        "Second tips",
                        LanguageTipsOrigin::ScriptTips { span: span(2, 2) },
                    )],
                ),
            ],
        };
        let requests = vec![
            request(
                "English.aul2",
                true,
                true,
                false,
                LanguageFileRequestOrigin::Configured { index: 0 },
            ),
            request(
                "Text.aul2",
                true,
                false,
                false,
                LanguageFileRequestOrigin::Configured { index: 1 },
            ),
            request(
                "Default.tips.aul2",
                false,
                true,
                true,
                LanguageFileRequestOrigin::Configured { index: 2 },
            ),
        ];

        let plans = build_language_file_plans(&requests, &catalog);

        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.request.path.as_path())
                .collect::<Vec<_>>(),
            requests
                .iter()
                .map(|request| request.path.as_path())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            plans[0]
                .sections
                .iter()
                .map(|section| (section.name(), section.category()))
                .collect::<Vec<_>>(),
            vec![
                ("first", LanguageSectionCategory::Text),
                ("Tips.first", LanguageSectionCategory::Tooltip),
                ("second", LanguageSectionCategory::Text),
                ("Tips.second", LanguageSectionCategory::Tooltip),
            ]
        );
        assert_eq!(
            plans[1]
                .sections
                .iter()
                .map(LanguageSectionPlan::name)
                .collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        assert_eq!(
            plans[2]
                .sections
                .iter()
                .map(LanguageSectionPlan::name)
                .collect::<Vec<_>>(),
            vec!["Tips.first", "Tips.second"]
        );

        let LanguageSectionPlan::Text { entries, .. } = &plans[0].sections[0] else {
            panic!("Text sectionではありません");
        };
        assert_eq!(entries[0].value, "");
        let LanguageSectionPlan::Tooltip { entries, .. } = &plans[2].sections[0] else {
            panic!("Tooltip sectionではありません");
        };
        assert_eq!(entries[0].value, "First tips");
    }

    #[test]
    fn empty_requests_produce_no_file_plans() {
        let catalog = catalog_with_all_entry_origins();

        assert!(build_language_file_plans(&[], &catalog).is_empty());
    }

    #[test]
    fn empty_catalog_preserves_file_plans_with_requests() {
        let requests = vec![
            request(
                "First.aul2",
                true,
                true,
                false,
                LanguageFileRequestOrigin::Configured { index: 0 },
            ),
            request(
                "Default.aul2",
                true,
                true,
                true,
                LanguageFileRequestOrigin::Override,
            ),
        ];
        let catalog = LanguageSectionCatalog {
            sections: Vec::new(),
        };

        let plans = build_language_file_plans(&requests, &catalog);

        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].request, requests[0]);
        assert_eq!(plans[1].request, requests[1]);
        assert!(plans.iter().all(|plan| plan.sections.is_empty()));
    }

    #[test]
    fn filtered_or_empty_candidates_leave_the_file_plan_empty() {
        let request = request(
            "Text.aul2",
            true,
            false,
            false,
            LanguageFileRequestOrigin::Configured { index: 0 },
        );
        let catalog = LanguageSectionCatalog {
            sections: vec![
                tooltip_candidate(
                    "Tips.script",
                    "script",
                    direct_origin("script.anm2"),
                    vec![tooltip_entry(
                        "effect.name",
                        "tips",
                        LanguageTipsOrigin::ScriptTips { span: span(1, 1) },
                    )],
                ),
                text_candidate("empty", "empty", direct_origin("empty.anm2"), Vec::new()),
            ],
        };

        let plans = build_language_file_plans(&[request], &catalog);

        assert_eq!(plans.len(), 1);
        assert!(plans[0].sections.is_empty());
    }

    #[test]
    fn section_accessors_return_variant_metadata() {
        let catalog = catalog_with_all_entry_origins();
        let plans = build_language_file_plans(
            &[request(
                "English.aul2",
                true,
                true,
                false,
                LanguageFileRequestOrigin::Override,
            )],
            &catalog,
        );

        assert_eq!(plans[0].sections[0].logical_script_name(), "script");
        assert_eq!(
            plans[0].sections[0].origin(),
            &configured_origin(&["first.lua", "second.lua"])
        );
        assert_eq!(plans[0].sections[1].logical_script_name(), "script");
        assert_eq!(plans[0].sections[1].origin(), &direct_origin("script.anm2"));
    }
}
