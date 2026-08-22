use std::collections::HashMap;

use thiserror::Error;

use crate::language_script_info::{LanguageScriptInfo, LanguageUiInfoMeta, TipsText};
use crate::language_ui::{LanguageUiKind, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageScriptEntries {
    pub text_entries: Vec<LanguageTextEntry>,
    pub tips_entries: Vec<LanguageTipsEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageTextEntry {
    pub key: String,
    pub origins: Vec<LanguageTextOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageTextOrigin {
    ScriptName,
    UiName {
        kind: LanguageUiKind,
        ui_span: SourceSpan,
    },
    TrackZeroLabel {
        ui_span: SourceSpan,
    },
    SelectOption {
        ui_span: SourceSpan,
        option_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageTipsEntry {
    pub key: String,
    pub value: String,
    pub origin: LanguageTipsOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageTipsOrigin {
    ScriptTips {
        span: SourceSpan,
    },
    UiTips {
        kind: LanguageUiKind,
        ui_span: SourceSpan,
        tips_span: SourceSpan,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectNameTipsConflictKind {
    ScriptTipsAffectsUi,
    UiTipsAffectsScript,
    ScriptAndUiTipsConflict,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LanguageScriptEntriesError {
    #[error(
        "値を持つUI項目の完全名が重複しています: {name}, {first_kind:?} {first_span:?}, {duplicate_kind:?} {duplicate_span:?}"
    )]
    DuplicateValueUiName {
        name: String,
        first_kind: LanguageUiKind,
        first_span: SourceSpan,
        duplicate_kind: LanguageUiKind,
        duplicate_span: SourceSpan,
    },
    #[error("groupの完全名が重複しています: {name}, {first_span:?}, {duplicate_span:?}")]
    DuplicateGroupName {
        name: String,
        first_span: SourceSpan,
        duplicate_span: SourceSpan,
    },
    #[error("groupとseparatorの完全名が競合しています: {name}, {group_span:?}, {separator_span:?}")]
    GroupSeparatorNameConflict {
        name: String,
        group_span: SourceSpan,
        separator_span: SourceSpan,
    },
    #[error(
        "groupの保存キーと値を持つUI項目の完全名が競合しています: {state_key}, group {group_name} {group_span:?}, {value_ui_kind:?} {value_ui_span:?}"
    )]
    GroupStateKeyConflict {
        state_key: String,
        group_name: String,
        group_span: SourceSpan,
        value_ui_kind: LanguageUiKind,
        value_ui_span: SourceSpan,
    },
    #[error(
        "同一select内で選択肢名が重複しています: {name}, {ui_span:?}, option {first_option_index}, option {duplicate_option_index}"
    )]
    DuplicateSelectOptionName {
        name: String,
        ui_span: SourceSpan,
        first_option_index: usize,
        duplicate_option_index: usize,
    },
    #[error(
        "UI項目の完全名 effect.name がTipsキーと競合しています: {kind:?} {ui_span:?}, {conflict_kind:?}"
    )]
    EffectNameTipsConflict {
        kind: LanguageUiKind,
        ui_span: SourceSpan,
        script_tips_span: Option<SourceSpan>,
        ui_tips_span: Option<SourceSpan>,
        conflict_kind: EffectNameTipsConflictKind,
    },
}

pub fn build_language_script_entries(
    info: &LanguageScriptInfo,
) -> Result<LanguageScriptEntries, LanguageScriptEntriesError> {
    validate_value_ui_names(info)?;
    validate_group_names(info)?;
    validate_group_separator_names(info)?;
    validate_group_state_keys(info)?;
    validate_select_option_names(info)?;
    validate_effect_name_tips(info)?;

    Ok(LanguageScriptEntries {
        text_entries: build_text_entries(info),
        tips_entries: build_tips_entries(info),
    })
}

fn validate_value_ui_names(info: &LanguageScriptInfo) -> Result<(), LanguageScriptEntriesError> {
    let mut names: HashMap<&str, (LanguageUiKind, SourceSpan)> = HashMap::new();

    for ui in &info.ui_items {
        if !is_standard_value_ui_kind(ui.kind) {
            continue;
        }
        let original = ui.name.name.original.as_str();
        if let Some(&(first_kind, first_span)) = names.get(original) {
            return Err(LanguageScriptEntriesError::DuplicateValueUiName {
                name: original.to_string(),
                first_kind,
                first_span,
                duplicate_kind: ui.kind,
                duplicate_span: ui.span,
            });
        }
        names.insert(original, (ui.kind, ui.span));
    }

    Ok(())
}

fn validate_group_names(info: &LanguageScriptInfo) -> Result<(), LanguageScriptEntriesError> {
    let mut names: HashMap<&str, SourceSpan> = HashMap::new();

    for ui in &info.ui_items {
        if ui.kind != LanguageUiKind::Group {
            continue;
        }
        let original = ui.name.name.original.as_str();
        if let Some(&first_span) = names.get(original) {
            return Err(LanguageScriptEntriesError::DuplicateGroupName {
                name: original.to_string(),
                first_span,
                duplicate_span: ui.span,
            });
        }
        names.insert(original, ui.span);
    }

    Ok(())
}

fn validate_group_separator_names(
    info: &LanguageScriptInfo,
) -> Result<(), LanguageScriptEntriesError> {
    let groups = info
        .ui_items
        .iter()
        .filter(|ui| ui.kind == LanguageUiKind::Group)
        .map(|ui| (ui.name.name.original.as_str(), ui.span))
        .collect::<HashMap<_, _>>();

    for separator in &info.ui_items {
        if separator.kind != LanguageUiKind::Separator {
            continue;
        }
        let original = separator.name.name.original.as_str();
        if let Some(&group_span) = groups.get(original) {
            return Err(LanguageScriptEntriesError::GroupSeparatorNameConflict {
                name: original.to_string(),
                group_span,
                separator_span: separator.span,
            });
        }
    }

    Ok(())
}

fn validate_group_state_keys(info: &LanguageScriptInfo) -> Result<(), LanguageScriptEntriesError> {
    for group in &info.ui_items {
        if group.kind != LanguageUiKind::Group {
            continue;
        }
        let group_name = group.name.name.original.as_str();
        let state_key = format!("{group_name}.hide");

        if let Some(value_ui) = info
            .ui_items
            .iter()
            .find(|ui| is_standard_value_ui_kind(ui.kind) && ui.name.name.original == state_key)
        {
            return Err(LanguageScriptEntriesError::GroupStateKeyConflict {
                state_key,
                group_name: group_name.to_string(),
                group_span: group.span,
                value_ui_kind: value_ui.kind,
                value_ui_span: value_ui.span,
            });
        }
    }

    Ok(())
}

fn validate_select_option_names(
    info: &LanguageScriptInfo,
) -> Result<(), LanguageScriptEntriesError> {
    for ui in &info.ui_items {
        if ui.kind.is_param() {
            continue;
        }
        let LanguageUiInfoMeta::Select { options } = &ui.meta else {
            continue;
        };
        let mut names: HashMap<&str, usize> = HashMap::new();

        for (option_index, option) in options.iter().enumerate() {
            let option_index = option_index + 1;
            if let Some(&first_option_index) = names.get(option.value.as_str()) {
                return Err(LanguageScriptEntriesError::DuplicateSelectOptionName {
                    name: option.value.clone(),
                    ui_span: ui.span,
                    first_option_index,
                    duplicate_option_index: option_index,
                });
            }
            names.insert(option.value.as_str(), option_index);
        }
    }

    Ok(())
}

fn validate_effect_name_tips(info: &LanguageScriptInfo) -> Result<(), LanguageScriptEntriesError> {
    for ui in &info.ui_items {
        if ui.kind.is_param() {
            continue;
        }
        if ui.name.name.original != "effect.name" {
            continue;
        }

        let script_tips_span = info.script_tips.as_ref().map(|tips| tips.span);
        let ui_tips_span = ui.tips.as_ref().map(|tips| tips.span);
        let conflict_kind = match (script_tips_span, ui_tips_span) {
            (Some(_), Some(_)) => EffectNameTipsConflictKind::ScriptAndUiTipsConflict,
            (Some(_), None) => EffectNameTipsConflictKind::ScriptTipsAffectsUi,
            (None, Some(_)) => EffectNameTipsConflictKind::UiTipsAffectsScript,
            (None, None) => continue,
        };

        return Err(LanguageScriptEntriesError::EffectNameTipsConflict {
            kind: ui.kind,
            ui_span: ui.span,
            script_tips_span,
            ui_tips_span,
            conflict_kind,
        });
    }

    Ok(())
}

fn build_text_entries(info: &LanguageScriptInfo) -> Vec<LanguageTextEntry> {
    let mut entries = Vec::new();
    let mut entry_indices = HashMap::new();

    if info.script_name.enabled {
        add_text_origin(
            &mut entries,
            &mut entry_indices,
            &info.script_name.value,
            LanguageTextOrigin::ScriptName,
        );
    }

    for ui in &info.ui_items {
        if ui.name.enabled {
            add_text_origin(
                &mut entries,
                &mut entry_indices,
                &ui.name.name.translation_key,
                LanguageTextOrigin::UiName {
                    kind: ui.kind,
                    ui_span: ui.span,
                },
            );
        }

        match &ui.meta {
            LanguageUiInfoMeta::Track {
                zero_label: Some(zero_label),
            } if zero_label.enabled => add_text_origin(
                &mut entries,
                &mut entry_indices,
                &zero_label.value,
                LanguageTextOrigin::TrackZeroLabel { ui_span: ui.span },
            ),
            LanguageUiInfoMeta::Select { options } => {
                for (option_index, option) in options.iter().enumerate() {
                    if option.enabled {
                        add_text_origin(
                            &mut entries,
                            &mut entry_indices,
                            &option.value,
                            LanguageTextOrigin::SelectOption {
                                ui_span: ui.span,
                                option_index: option_index + 1,
                            },
                        );
                    }
                }
            }
            _ => {}
        }
    }

    entries
}

fn add_text_origin(
    entries: &mut Vec<LanguageTextEntry>,
    entry_indices: &mut HashMap<String, usize>,
    key: &str,
    origin: LanguageTextOrigin,
) {
    if let Some(&entry_index) = entry_indices.get(key) {
        entries[entry_index].origins.push(origin);
    } else {
        let entry_index = entries.len();
        entries.push(LanguageTextEntry {
            key: key.to_string(),
            origins: vec![origin],
        });
        entry_indices.insert(key.to_string(), entry_index);
    }
}

fn build_tips_entries(info: &LanguageScriptInfo) -> Vec<LanguageTipsEntry> {
    let mut entries = Vec::new();

    if let Some(tips) = &info.script_tips {
        entries.push(LanguageTipsEntry {
            key: "effect.name".to_string(),
            value: tips.value.clone(),
            origin: LanguageTipsOrigin::ScriptTips { span: tips.span },
        });
    }

    for ui in &info.ui_items {
        if let Some(tips) = &ui.tips {
            entries.push(language_ui_tips_entry(
                ui.kind,
                &ui.name.name.original,
                ui.span,
                tips,
            ));
        }
    }

    entries
}

fn language_ui_tips_entry(
    kind: LanguageUiKind,
    original_name: &str,
    ui_span: SourceSpan,
    tips: &TipsText,
) -> LanguageTipsEntry {
    LanguageTipsEntry {
        key: original_name.to_string(),
        value: tips.value.clone(),
        origin: LanguageTipsOrigin::UiTips {
            kind,
            ui_span,
            tips_span: tips.span,
        },
    }
}

fn is_standard_value_ui_kind(kind: LanguageUiKind) -> bool {
    !kind.is_param() && !matches!(kind, LanguageUiKind::Group | LanguageUiKind::Separator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_script_info::{
        LanguageNamedText, LanguageText, LanguageUiInfo, LanguageUiInfoMeta,
    };
    use crate::language_ui::LanguageUiName;

    fn span(start_line: usize, end_line: usize) -> SourceSpan {
        SourceSpan {
            start_line,
            end_line,
        }
    }

    fn info(ui_items: Vec<LanguageUiInfo>) -> LanguageScriptInfo {
        LanguageScriptInfo {
            script_name: LanguageText {
                value: "script".to_string(),
                enabled: true,
            },
            script_tips: None,
            ui_items,
        }
    }

    fn ui(
        kind: LanguageUiKind,
        original: &str,
        translation_key: &str,
        line: usize,
    ) -> LanguageUiInfo {
        LanguageUiInfo {
            kind,
            name: LanguageNamedText {
                name: LanguageUiName {
                    original: original.to_string(),
                    translation_key: translation_key.to_string(),
                },
                enabled: true,
            },
            tips: None,
            meta: LanguageUiInfoMeta::None,
            span: span(line, line),
        }
    }

    fn tips(value: &str, line: usize) -> TipsText {
        TipsText {
            value: value.to_string(),
            span: span(line, line),
        }
    }

    fn build(
        info: &LanguageScriptInfo,
    ) -> Result<LanguageScriptEntries, LanguageScriptEntriesError> {
        build_language_script_entries(info)
    }

    #[test]
    fn rejects_duplicate_value_ui_names_with_same_or_different_kinds() {
        for (first_kind, duplicate_kind) in [
            (LanguageUiKind::Track, LanguageUiKind::Track),
            (LanguageUiKind::Track, LanguageUiKind::Check),
        ] {
            let model = info(vec![
                ui(first_kind, "Item", "Item", 1),
                ui(duplicate_kind, "Item", "Item", 3),
            ]);

            assert_eq!(
                build(&model).unwrap_err(),
                LanguageScriptEntriesError::DuplicateValueUiName {
                    name: "Item".to_string(),
                    first_kind,
                    first_span: span(1, 1),
                    duplicate_kind,
                    duplicate_span: span(3, 3),
                }
            );
        }
    }

    #[test]
    fn allows_duplicate_params_and_names_shared_with_standard_ui() {
        let model = info(vec![
            ui(LanguageUiKind::Param, "周期", "周期", 1),
            ui(LanguageUiKind::ParamCheck, "周期", "周期", 2),
            ui(LanguageUiKind::ParamSelect, "周期", "周期", 3),
            ui(LanguageUiKind::Track, "周期", "周期", 4),
        ]);

        let result = build(&model).unwrap();

        let entry = result
            .text_entries
            .iter()
            .find(|entry| entry.key == "周期")
            .unwrap();
        assert_eq!(entry.origins.len(), 4);
        assert!(
            entry
                .origins
                .iter()
                .all(|origin| matches!(origin, LanguageTextOrigin::UiName { .. }))
        );
    }

    #[test]
    fn param_kinds_do_not_conflict_with_group_state_key() {
        for kind in [
            LanguageUiKind::Param,
            LanguageUiKind::ParamCheck,
            LanguageUiKind::ParamSelect,
        ] {
            let model = info(vec![
                ui(LanguageUiKind::Group, "Settings", "Settings", 1),
                ui(kind, "Settings.hide", "Settings.hide", 2),
            ]);

            assert!(build(&model).is_ok());
        }
    }

    #[test]
    fn value_ui_structure_validation_ignores_enabled_but_uses_exact_original_name() {
        let mut disabled = ui(LanguageUiKind::Check, "Item", "Item", 2);
        disabled.name.enabled = false;
        assert!(matches!(
            build(&info(vec![
                ui(LanguageUiKind::Track, "Item", "Item", 1),
                disabled,
            ])),
            Err(LanguageScriptEntriesError::DuplicateValueUiName { .. })
        ));

        let allowed = info(vec![
            ui(LanguageUiKind::Track, "aaa::Item", "Item", 1),
            ui(LanguageUiKind::Check, "bbb::Item", "Item", 2),
            ui(LanguageUiKind::Color, "ITEM", "ITEM", 3),
            ui(LanguageUiKind::File, "é", "é", 4),
            ui(LanguageUiKind::Folder, "e\u{301}", "e\u{301}", 5),
        ]);
        assert!(build(&allowed).is_ok());
    }

    #[test]
    fn rejects_duplicate_group_names_even_when_disabled() {
        let mut duplicate = ui(LanguageUiKind::Group, "Group", "Group", 4);
        duplicate.name.enabled = false;
        assert_eq!(
            build(&info(vec![
                ui(LanguageUiKind::Group, "Group", "Group", 1),
                duplicate,
            ]))
            .unwrap_err(),
            LanguageScriptEntriesError::DuplicateGroupName {
                name: "Group".to_string(),
                first_span: span(1, 1),
                duplicate_span: span(4, 4),
            }
        );
    }

    #[test]
    fn allows_duplicate_separators_and_names_shared_with_value_ui() {
        let model = info(vec![
            ui(LanguageUiKind::Separator, "Shared", "Shared", 1),
            ui(LanguageUiKind::Separator, "Shared", "Shared", 2),
            ui(LanguageUiKind::Track, "Shared", "Shared", 3),
            ui(LanguageUiKind::Group, "Group", "Group", 4),
            ui(LanguageUiKind::Check, "Group", "Group", 5),
        ]);

        assert!(build(&model).is_ok());
    }

    #[test]
    fn rejects_group_separator_conflict_regardless_of_order() {
        for ui_items in [
            vec![
                ui(LanguageUiKind::Group, "Same", "Same", 1),
                ui(LanguageUiKind::Separator, "Same", "Same", 2),
            ],
            vec![
                ui(LanguageUiKind::Separator, "Same", "Same", 1),
                ui(LanguageUiKind::Group, "Same", "Same", 2),
            ],
        ] {
            assert!(matches!(
                build(&info(ui_items)),
                Err(LanguageScriptEntriesError::GroupSeparatorNameConflict {
                    name,
                    ..
                }) if name == "Same"
            ));
        }
    }

    #[test]
    fn group_separator_comparison_is_case_sensitive_and_does_not_normalize_unicode() {
        let model = info(vec![
            ui(LanguageUiKind::Group, "Name", "Name", 1),
            ui(LanguageUiKind::Separator, "name", "name", 2),
            ui(LanguageUiKind::Group, "é", "é", 3),
            ui(LanguageUiKind::Separator, "e\u{301}", "e\u{301}", 4),
        ]);

        assert!(build(&model).is_ok());
    }

    #[test]
    fn rejects_group_state_key_conflict_with_value_ui_name() {
        assert_eq!(
            build(&info(vec![
                ui(LanguageUiKind::Group, "Settings", "Settings", 1),
                ui(LanguageUiKind::Track, "Settings.hide", "Settings.hide", 3,),
            ]))
            .unwrap_err(),
            LanguageScriptEntriesError::GroupStateKeyConflict {
                state_key: "Settings.hide".to_string(),
                group_name: "Settings".to_string(),
                group_span: span(1, 1),
                value_ui_kind: LanguageUiKind::Track,
                value_ui_span: span(3, 3),
            }
        );
    }

    #[test]
    fn group_state_key_comparison_is_case_sensitive_and_does_not_normalize_unicode() {
        let model = info(vec![
            ui(LanguageUiKind::Group, "Settings", "Settings", 1),
            ui(LanguageUiKind::Track, "settings.hide", "settings.hide", 2),
            ui(LanguageUiKind::Group, "é", "é", 3),
            ui(LanguageUiKind::Check, "e\u{301}.hide", "e\u{301}.hide", 4),
        ]);

        assert!(build(&model).is_ok());
    }

    #[test]
    fn rejects_duplicate_select_options_with_one_based_indices_even_when_disabled() {
        let mut select = ui(LanguageUiKind::Select, "Select", "Select", 1);
        select.meta = LanguageUiInfoMeta::Select {
            options: vec![
                LanguageText {
                    value: "Same".to_string(),
                    enabled: false,
                },
                LanguageText {
                    value: "Other".to_string(),
                    enabled: true,
                },
                LanguageText {
                    value: "Same".to_string(),
                    enabled: false,
                },
            ],
        };

        assert_eq!(
            build(&info(vec![select])).unwrap_err(),
            LanguageScriptEntriesError::DuplicateSelectOptionName {
                name: "Same".to_string(),
                ui_span: span(1, 1),
                first_option_index: 1,
                duplicate_option_index: 3,
            }
        );
    }

    #[test]
    fn param_select_allows_and_aggregates_duplicate_option_names() {
        let mut select = ui(LanguageUiKind::ParamSelect, "種類", "種類", 1);
        select.meta = LanguageUiInfoMeta::Select {
            options: vec![
                LanguageText {
                    value: "aaa::直線".to_string(),
                    enabled: true,
                },
                LanguageText {
                    value: "aaa::直線".to_string(),
                    enabled: true,
                },
            ],
        };

        let entries = build(&info(vec![select])).unwrap().text_entries;
        let option = entries
            .iter()
            .find(|entry| entry.key == "aaa::直線")
            .unwrap();
        assert_eq!(option.origins.len(), 2);
        assert!(
            option
                .origins
                .iter()
                .all(|origin| matches!(origin, LanguageTextOrigin::SelectOption { .. }))
        );
    }

    #[test]
    fn allows_same_options_across_selects_and_compares_option_names_exactly() {
        let mut first = ui(LanguageUiKind::Select, "First", "First", 1);
        first.meta = LanguageUiInfoMeta::Select {
            options: ["Same", "aaa::Item", "Name", "é"]
                .into_iter()
                .map(|value| LanguageText {
                    value: value.to_string(),
                    enabled: true,
                })
                .collect(),
        };
        let mut second = ui(LanguageUiKind::Select, "Second", "Second", 2);
        second.meta = LanguageUiInfoMeta::Select {
            options: ["Same", "bbb::Item", "name", "e\u{301}"]
                .into_iter()
                .map(|value| LanguageText {
                    value: value.to_string(),
                    enabled: true,
                })
                .collect(),
        };

        assert!(build(&info(vec![first, second])).is_ok());
    }

    #[test]
    fn aggregates_text_entries_in_first_occurrence_and_origin_order() {
        let mut track = ui(LanguageUiKind::Track, "aaa::Shared", "Shared", 2);
        track.meta = LanguageUiInfoMeta::Track {
            zero_label: Some(LanguageText {
                value: "Shared".to_string(),
                enabled: true,
            }),
        };
        let mut select = ui(LanguageUiKind::Select, "List", "List", 4);
        select.meta = LanguageUiInfoMeta::Select {
            options: vec![
                LanguageText {
                    value: "Shared".to_string(),
                    enabled: true,
                },
                LanguageText {
                    value: "Unique".to_string(),
                    enabled: true,
                },
                LanguageText {
                    value: "Disabled".to_string(),
                    enabled: false,
                },
                LanguageText {
                    value: "aaa::Option".to_string(),
                    enabled: true,
                },
            ],
        };
        let mut model = info(vec![track, select]);
        model.script_name.value = "Shared".to_string();

        let entries = build(&model).unwrap().text_entries;

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["Shared", "List", "Unique", "aaa::Option"]
        );
        assert_eq!(
            entries[0].origins,
            vec![
                LanguageTextOrigin::ScriptName,
                LanguageTextOrigin::UiName {
                    kind: LanguageUiKind::Track,
                    ui_span: span(2, 2),
                },
                LanguageTextOrigin::TrackZeroLabel {
                    ui_span: span(2, 2),
                },
                LanguageTextOrigin::SelectOption {
                    ui_span: span(4, 4),
                    option_index: 1,
                },
            ]
        );
        assert!(!entries.iter().any(|entry| entry.key == "Disabled"));
    }

    #[test]
    fn aggregates_different_original_ui_names_by_translation_key() {
        let entries = build(&info(vec![
            ui(LanguageUiKind::Track, "aaa::Item", "Item", 1),
            ui(LanguageUiKind::Check, "bbb::Item", "Item", 2),
        ]))
        .unwrap()
        .text_entries;

        let item = entries.iter().find(|entry| entry.key == "Item").unwrap();
        assert_eq!(
            item.origins,
            vec![
                LanguageTextOrigin::UiName {
                    kind: LanguageUiKind::Track,
                    ui_span: span(1, 1),
                },
                LanguageTextOrigin::UiName {
                    kind: LanguageUiKind::Check,
                    ui_span: span(2, 2),
                },
            ]
        );
    }

    #[test]
    fn excludes_disabled_text_candidates_and_does_not_normalize_keys() {
        let mut first = ui(LanguageUiKind::Track, " Original ", " Key ", 1);
        first.name.enabled = false;
        first.meta = LanguageUiInfoMeta::Track {
            zero_label: Some(LanguageText {
                value: "Zero".to_string(),
                enabled: false,
            }),
        };
        let mut second = ui(LanguageUiKind::Select, "Select", "Select", 2);
        second.meta = LanguageUiInfoMeta::Select {
            options: ["Key", "key", "é", "e\u{301}", " a::b "]
                .into_iter()
                .map(|value| LanguageText {
                    value: value.to_string(),
                    enabled: true,
                })
                .collect(),
        };
        let mut model = info(vec![first, second]);
        model.script_name.enabled = false;

        let entries = build(&model).unwrap().text_entries;
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["Select", "Key", "key", "é", "e\u{301}", " a::b "]
        );
    }

    #[test]
    fn aggregates_shared_keys_from_different_zero_labels_and_selects() {
        let mut first_track = ui(LanguageUiKind::Track, "Track1", "Track1", 1);
        first_track.meta = LanguageUiInfoMeta::Track {
            zero_label: Some(LanguageText {
                value: "Shared".to_string(),
                enabled: true,
            }),
        };
        let mut second_track = ui(LanguageUiKind::Track, "Track2", "Track2", 2);
        second_track.meta = LanguageUiInfoMeta::Track {
            zero_label: Some(LanguageText {
                value: "Shared".to_string(),
                enabled: true,
            }),
        };
        let mut first_select = ui(LanguageUiKind::Select, "Select1", "Select1", 3);
        first_select.meta = LanguageUiInfoMeta::Select {
            options: vec![LanguageText {
                value: "Shared".to_string(),
                enabled: true,
            }],
        };
        let mut second_select = ui(LanguageUiKind::Select, "Select2", "Select2", 4);
        second_select.meta = LanguageUiInfoMeta::Select {
            options: vec![LanguageText {
                value: "Shared".to_string(),
                enabled: true,
            }],
        };

        let entries = build(&info(vec![
            first_track,
            second_track,
            first_select,
            second_select,
        ]))
        .unwrap()
        .text_entries;
        let shared = entries.iter().find(|entry| entry.key == "Shared").unwrap();
        assert_eq!(shared.origins.len(), 4);
    }

    #[test]
    fn empty_enabled_input_produces_empty_entries() {
        let mut model = info(vec![]);
        model.script_name.enabled = false;

        assert_eq!(
            build(&model).unwrap(),
            LanguageScriptEntries {
                text_entries: vec![],
                tips_entries: vec![],
            }
        );
    }

    #[test]
    fn builds_script_and_ui_tips_using_original_names_and_preserves_values() {
        let mut first = ui(LanguageUiKind::Track, "aaa::Item", "Item", 3);
        first.name.enabled = false;
        first.tips = Some(tips("UI\nTips", 1));
        let mut second = ui(LanguageUiKind::Check, "bbb::Item", "Item", 6);
        second.tips = Some(tips("Second", 5));
        let mut model = info(vec![first, second]);
        model.script_tips = Some(tips("Script\nTips", 2));

        let entries = build(&model).unwrap().tips_entries;

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["effect.name", "aaa::Item", "bbb::Item"]
        );
        assert_eq!(entries[0].value, "Script\nTips");
        assert_eq!(entries[1].value, "UI\nTips");
        assert_eq!(
            entries[1].origin,
            LanguageTipsOrigin::UiTips {
                kind: LanguageUiKind::Track,
                ui_span: span(3, 3),
                tips_span: span(1, 1),
            }
        );
    }

    #[test]
    fn classifies_all_effect_name_conflict_kinds() {
        for (script_tips, ui_tips, expected) in [
            (
                Some(tips("Script", 1)),
                None,
                EffectNameTipsConflictKind::ScriptTipsAffectsUi,
            ),
            (
                None,
                Some(tips("UI", 2)),
                EffectNameTipsConflictKind::UiTipsAffectsScript,
            ),
            (
                Some(tips("Script", 1)),
                Some(tips("UI", 2)),
                EffectNameTipsConflictKind::ScriptAndUiTipsConflict,
            ),
        ] {
            let mut effect_ui = ui(LanguageUiKind::Track, "effect.name", "effect.name", 3);
            effect_ui.name.enabled = false;
            effect_ui.tips = ui_tips.clone();
            let mut model = info(vec![effect_ui]);
            model.script_tips = script_tips.clone();

            assert_eq!(
                build(&model).unwrap_err(),
                LanguageScriptEntriesError::EffectNameTipsConflict {
                    kind: LanguageUiKind::Track,
                    ui_span: span(3, 3),
                    script_tips_span: script_tips.map(|tips| tips.span),
                    ui_tips_span: ui_tips.map(|tips| tips.span),
                    conflict_kind: expected,
                }
            );
        }
    }

    #[test]
    fn effect_name_check_includes_structure_ui_but_allows_no_tips_and_namespaced_name() {
        let no_tips = info(vec![ui(
            LanguageUiKind::Separator,
            "effect.name",
            "effect.name",
            1,
        )]);
        assert!(build(&no_tips).is_ok());

        let mut structure = info(vec![ui(
            LanguageUiKind::Group,
            "effect.name",
            "effect.name",
            2,
        )]);
        structure.script_tips = Some(tips("Script", 1));
        assert!(matches!(
            build(&structure),
            Err(LanguageScriptEntriesError::EffectNameTipsConflict {
                kind: LanguageUiKind::Group,
                ..
            })
        ));

        let mut namespaced = info(vec![ui(
            LanguageUiKind::Track,
            "aaa::effect.name",
            "effect.name",
            3,
        )]);
        namespaced.script_tips = Some(tips("Script", 1));
        assert!(build(&namespaced).is_ok());
    }

    #[test]
    fn validation_order_reports_value_ui_conflict_first() {
        let mut select = ui(LanguageUiKind::Select, "Duplicate", "Duplicate", 1);
        select.meta = LanguageUiInfoMeta::Select {
            options: vec![
                LanguageText {
                    value: "Option".to_string(),
                    enabled: true,
                },
                LanguageText {
                    value: "Option".to_string(),
                    enabled: true,
                },
            ],
        };
        let model = info(vec![
            select,
            ui(LanguageUiKind::Check, "Duplicate", "Duplicate", 2),
            ui(LanguageUiKind::Group, "Group", "Group", 3),
            ui(LanguageUiKind::Group, "Group", "Group", 4),
        ]);

        assert!(matches!(
            build(&model),
            Err(LanguageScriptEntriesError::DuplicateValueUiName { .. })
        ));
    }
}
