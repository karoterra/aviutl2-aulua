use thiserror::Error;

use crate::language_directive::{LanguageDirective, LanguageDirectiveKind, NolangTarget};
use crate::language_ui::{
    LanguageUiItem, LanguageUiKind, LanguageUiMeta, LanguageUiName, SourceSpan,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageScriptInfo {
    pub script_name: LanguageText,
    pub script_tips: Option<TipsText>,
    pub ui_items: Vec<LanguageUiInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageText {
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageNamedText {
    pub name: LanguageUiName,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TipsText {
    pub value: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageUiInfo {
    pub kind: LanguageUiKind,
    pub name: LanguageNamedText,
    pub tips: Option<TipsText>,
    pub meta: LanguageUiInfoMeta,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageUiInfoMeta {
    None,
    Track { zero_label: Option<LanguageText> },
    Select { options: Vec<LanguageText> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageInfoEventKind {
    Ui(LanguageUiKind),
    Tips,
    ScriptTips,
    Nolang,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LanguageScriptInfoError {
    #[error("languageイベントのspanが不正です: {event:?} {span:?}")]
    InvalidSpan {
        event: LanguageInfoEventKind,
        span: SourceSpan,
    },
    #[error("UI項目の順序が不正です: {previous_span:?}, {current_span:?}")]
    UnorderedUiItems {
        previous_span: SourceSpan,
        current_span: SourceSpan,
    },
    #[error("languageディレクティブの順序が不正です: {previous_span:?}, {current_span:?}")]
    UnorderedDirectives {
        previous_span: SourceSpan,
        current_span: SourceSpan,
    },
    #[error("UI項目のspanが重複しています: {first_span:?}, {second_span:?}")]
    OverlappingUiItems {
        first_kind: LanguageUiKind,
        first_span: SourceSpan,
        second_kind: LanguageUiKind,
        second_span: SourceSpan,
    },
    #[error("languageディレクティブのspanが重複しています: {first_span:?}, {second_span:?}")]
    OverlappingDirectives {
        first_kind: LanguageInfoEventKind,
        first_span: SourceSpan,
        second_kind: LanguageInfoEventKind,
        second_span: SourceSpan,
    },
    #[error(
        "UI項目とlanguageディレクティブのspanが重複しています: {ui_span:?}, {directive_span:?}"
    )]
    OverlappingUiAndDirective {
        ui_kind: LanguageUiKind,
        ui_span: SourceSpan,
        directive_kind: LanguageInfoEventKind,
        directive_span: SourceSpan,
    },
    #[error("同じUI項目に複数のTipsが指定されています: {first_span:?}, {duplicate_span:?}")]
    DuplicateTips {
        first_span: SourceSpan,
        duplicate_span: SourceSpan,
    },
    #[error("このUI項目にはTipsを指定できません: {kind:?} {ui_span:?}, {tips_span:?}")]
    TipsNotSupported {
        kind: LanguageUiKind,
        ui_span: SourceSpan,
        tips_span: SourceSpan,
    },
    #[error("script_tipsが複数指定されています: {first_span:?}, {duplicate_span:?}")]
    DuplicateScriptTips {
        first_span: SourceSpan,
        duplicate_span: SourceSpan,
    },
    #[error("nolangのscript_nameが複数指定されています: {first_span:?}, {duplicate_span:?}")]
    DuplicateScriptNameTarget {
        first_span: SourceSpan,
        duplicate_span: SourceSpan,
    },
    #[error(
        "このUI項目にはnolang対象を適用できません: {target:?}, {kind:?} {ui_span:?}, {directive_span:?}"
    )]
    NolangTargetNotSupported {
        target: NolangTarget,
        directive_span: SourceSpan,
        kind: LanguageUiKind,
        ui_span: SourceSpan,
    },
    #[error("trackにzero labelがありません: {ui_span:?}, {directive_span:?}")]
    MissingZeroLabel {
        directive_span: SourceSpan,
        ui_span: SourceSpan,
    },
    #[error("selectに指定された選択肢がありません: {option_name}, {ui_span:?}, {directive_span:?}")]
    MissingSelectOption {
        option_name: String,
        directive_span: SourceSpan,
        ui_span: SourceSpan,
    },
    #[error("後続UI項目のないTipsがあります: {span:?}")]
    UnconsumedTips { span: SourceSpan },
    #[error("後続UI項目のないnolang指定があります: {span:?}")]
    UnconsumedNolang { span: SourceSpan },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingNolangTarget {
    target: NolangTarget,
    span: SourceSpan,
}

pub fn build_language_script_info(
    script_name: &str,
    ui_items: Vec<LanguageUiItem>,
    directives: Vec<LanguageDirective>,
) -> Result<LanguageScriptInfo, LanguageScriptInfoError> {
    validate_inputs(&ui_items, &directives)?;

    let mut script_name = LanguageText {
        value: script_name.to_string(),
        enabled: true,
    };
    let mut script_tips: Option<TipsText> = None;
    let mut script_name_target_span: Option<SourceSpan> = None;
    let mut pending_tips: Option<TipsText> = None;
    let mut pending_nolang: Vec<PendingNolangTarget> = Vec::new();
    let mut resolved_ui_items = Vec::with_capacity(ui_items.len());
    let mut ui_items = ui_items.into_iter().peekable();
    let mut directives = directives.into_iter().peekable();

    while ui_items.peek().is_some() || directives.peek().is_some() {
        let take_ui = match (ui_items.peek(), directives.peek()) {
            (Some(ui), Some(directive)) => ui.span.start_line < directive.span.start_line,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => unreachable!(),
        };

        if take_ui {
            let ui = ui_items.next().unwrap();
            let tips = pending_tips.take();
            let nolang = std::mem::take(&mut pending_nolang);
            resolved_ui_items.push(apply_pending_to_ui(ui, tips, nolang)?);
        } else {
            let directive = directives.next().unwrap();
            match directive.kind {
                LanguageDirectiveKind::Tips { text } => {
                    if let Some(first) = &pending_tips {
                        return Err(LanguageScriptInfoError::DuplicateTips {
                            first_span: first.span,
                            duplicate_span: directive.span,
                        });
                    }
                    pending_tips = Some(TipsText {
                        value: text,
                        span: directive.span,
                    });
                }
                LanguageDirectiveKind::ScriptTips { text } => {
                    if let Some(first) = &script_tips {
                        return Err(LanguageScriptInfoError::DuplicateScriptTips {
                            first_span: first.span,
                            duplicate_span: directive.span,
                        });
                    }
                    script_tips = Some(TipsText {
                        value: text,
                        span: directive.span,
                    });
                }
                LanguageDirectiveKind::Nolang { targets } => {
                    for target in targets {
                        if target == NolangTarget::ScriptName {
                            if let Some(first_span) = script_name_target_span {
                                return Err(LanguageScriptInfoError::DuplicateScriptNameTarget {
                                    first_span,
                                    duplicate_span: directive.span,
                                });
                            }
                            script_name.enabled = false;
                            script_name_target_span = Some(directive.span);
                        } else if !pending_nolang
                            .iter()
                            .any(|pending| pending.target == target)
                        {
                            pending_nolang.push(PendingNolangTarget {
                                target,
                                span: directive.span,
                            });
                        }
                    }
                }
            }
        }
    }

    if let Some(tips) = pending_tips {
        return Err(LanguageScriptInfoError::UnconsumedTips { span: tips.span });
    }
    if let Some(pending) = pending_nolang.first() {
        return Err(LanguageScriptInfoError::UnconsumedNolang { span: pending.span });
    }

    Ok(LanguageScriptInfo {
        script_name,
        script_tips,
        ui_items: resolved_ui_items,
    })
}

fn apply_pending_to_ui(
    ui: LanguageUiItem,
    tips: Option<TipsText>,
    nolang: Vec<PendingNolangTarget>,
) -> Result<LanguageUiInfo, LanguageScriptInfoError> {
    if matches!(
        ui.kind,
        LanguageUiKind::Param | LanguageUiKind::Group | LanguageUiKind::Separator
    ) && let Some(tips) = &tips
    {
        return Err(LanguageScriptInfoError::TipsNotSupported {
            kind: ui.kind,
            ui_span: ui.span,
            tips_span: tips.span,
        });
    }

    let mut result = LanguageUiInfo {
        kind: ui.kind,
        name: LanguageNamedText {
            name: ui.name,
            enabled: true,
        },
        tips,
        meta: match ui.meta {
            LanguageUiMeta::None => LanguageUiInfoMeta::None,
            LanguageUiMeta::Track { zero_label } => LanguageUiInfoMeta::Track {
                zero_label: zero_label.filter(|value| !value.is_empty()).map(|value| {
                    LanguageText {
                        value,
                        enabled: true,
                    }
                }),
            },
            LanguageUiMeta::Select { options } => LanguageUiInfoMeta::Select {
                options: options
                    .into_iter()
                    .map(|value| LanguageText {
                        value,
                        enabled: true,
                    })
                    .collect(),
            },
        },
        span: ui.span,
    };

    for pending in nolang {
        match pending.target {
            NolangTarget::Name => result.name.enabled = false,
            NolangTarget::ZeroLabel => match &mut result.meta {
                LanguageUiInfoMeta::Track { zero_label } => {
                    let Some(zero_label) = zero_label else {
                        return Err(LanguageScriptInfoError::MissingZeroLabel {
                            directive_span: pending.span,
                            ui_span: result.span,
                        });
                    };
                    zero_label.enabled = false;
                }
                _ => {
                    return Err(LanguageScriptInfoError::NolangTargetNotSupported {
                        target: NolangTarget::ZeroLabel,
                        directive_span: pending.span,
                        kind: result.kind,
                        ui_span: result.span,
                    });
                }
            },
            NolangTarget::Options => match &mut result.meta {
                LanguageUiInfoMeta::Select { options } => {
                    for option in options {
                        option.enabled = false;
                    }
                }
                _ => {
                    return Err(LanguageScriptInfoError::NolangTargetNotSupported {
                        target: NolangTarget::Options,
                        directive_span: pending.span,
                        kind: result.kind,
                        ui_span: result.span,
                    });
                }
            },
            NolangTarget::Option(option_name) => match &mut result.meta {
                LanguageUiInfoMeta::Select { options } => {
                    let mut found = false;
                    for option in options {
                        if option.value == option_name {
                            option.enabled = false;
                            found = true;
                        }
                    }
                    if !found {
                        return Err(LanguageScriptInfoError::MissingSelectOption {
                            option_name,
                            directive_span: pending.span,
                            ui_span: result.span,
                        });
                    }
                }
                _ => {
                    return Err(LanguageScriptInfoError::NolangTargetNotSupported {
                        target: NolangTarget::Option(option_name),
                        directive_span: pending.span,
                        kind: result.kind,
                        ui_span: result.span,
                    });
                }
            },
            NolangTarget::ScriptName => unreachable!(),
        }
    }

    Ok(result)
}

fn validate_inputs(
    ui_items: &[LanguageUiItem],
    directives: &[LanguageDirective],
) -> Result<(), LanguageScriptInfoError> {
    for ui in ui_items {
        validate_span(LanguageInfoEventKind::Ui(ui.kind), ui.span)?;
    }
    for directive in directives {
        validate_span(directive_event_kind(directive), directive.span)?;
    }

    for pair in ui_items.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        if current.span.start_line < previous.span.start_line {
            return Err(LanguageScriptInfoError::UnorderedUiItems {
                previous_span: previous.span,
                current_span: current.span,
            });
        }
        if current.span.start_line <= previous.span.end_line {
            return Err(LanguageScriptInfoError::OverlappingUiItems {
                first_kind: previous.kind,
                first_span: previous.span,
                second_kind: current.kind,
                second_span: current.span,
            });
        }
    }

    for pair in directives.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        if current.span.start_line < previous.span.start_line {
            return Err(LanguageScriptInfoError::UnorderedDirectives {
                previous_span: previous.span,
                current_span: current.span,
            });
        }
        if current.span.start_line <= previous.span.end_line {
            return Err(LanguageScriptInfoError::OverlappingDirectives {
                first_kind: directive_event_kind(previous),
                first_span: previous.span,
                second_kind: directive_event_kind(current),
                second_span: current.span,
            });
        }
    }

    let mut ui_index = 0;
    let mut directive_index = 0;
    while let (Some(ui), Some(directive)) =
        (ui_items.get(ui_index), directives.get(directive_index))
    {
        if spans_overlap(ui.span, directive.span) {
            return Err(LanguageScriptInfoError::OverlappingUiAndDirective {
                ui_kind: ui.kind,
                ui_span: ui.span,
                directive_kind: directive_event_kind(directive),
                directive_span: directive.span,
            });
        }
        if ui.span.end_line < directive.span.start_line {
            ui_index += 1;
        } else {
            directive_index += 1;
        }
    }

    Ok(())
}

fn validate_span(
    event: LanguageInfoEventKind,
    span: SourceSpan,
) -> Result<(), LanguageScriptInfoError> {
    if span.start_line == 0 || span.end_line == 0 || span.start_line > span.end_line {
        return Err(LanguageScriptInfoError::InvalidSpan { event, span });
    }
    Ok(())
}

fn spans_overlap(first: SourceSpan, second: SourceSpan) -> bool {
    first.start_line <= second.end_line && second.start_line <= first.end_line
}

fn directive_event_kind(directive: &LanguageDirective) -> LanguageInfoEventKind {
    match directive.kind {
        LanguageDirectiveKind::Tips { .. } => LanguageInfoEventKind::Tips,
        LanguageDirectiveKind::ScriptTips { .. } => LanguageInfoEventKind::ScriptTips,
        LanguageDirectiveKind::Nolang { .. } => LanguageInfoEventKind::Nolang,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start_line: usize, end_line: usize) -> SourceSpan {
        SourceSpan {
            start_line,
            end_line,
        }
    }

    fn ui(
        kind: LanguageUiKind,
        start_line: usize,
        end_line: usize,
        meta: LanguageUiMeta,
    ) -> LanguageUiItem {
        LanguageUiItem {
            kind,
            name: LanguageUiName {
                original: format!("{kind:?}"),
                translation_key: format!("{kind:?}"),
            },
            meta,
            span: span(start_line, end_line),
        }
    }

    fn directive(kind: LanguageDirectiveKind, line: usize) -> LanguageDirective {
        LanguageDirective {
            kind,
            span: span(line, line),
        }
    }

    fn tips(text: &str, line: usize) -> LanguageDirective {
        directive(
            LanguageDirectiveKind::Tips {
                text: text.to_string(),
            },
            line,
        )
    }

    fn script_tips(text: &str, line: usize) -> LanguageDirective {
        directive(
            LanguageDirectiveKind::ScriptTips {
                text: text.to_string(),
            },
            line,
        )
    }

    fn nolang(targets: Vec<NolangTarget>, line: usize) -> LanguageDirective {
        directive(LanguageDirectiveKind::Nolang { targets }, line)
    }

    fn build(
        ui_items: Vec<LanguageUiItem>,
        directives: Vec<LanguageDirective>,
    ) -> Result<LanguageScriptInfo, LanguageScriptInfoError> {
        build_language_script_info("script", ui_items, directives)
    }

    #[test]
    fn builds_default_info_without_directives_and_preserves_ui_order_and_spans() {
        let result = build(
            vec![
                ui(LanguageUiKind::Check, 2, 3, LanguageUiMeta::None),
                ui(LanguageUiKind::Group, 8, 8, LanguageUiMeta::None),
            ],
            vec![],
        )
        .unwrap();

        assert_eq!(
            result.script_name,
            LanguageText {
                value: "script".to_string(),
                enabled: true
            }
        );
        assert!(result.script_tips.is_none());
        assert_eq!(result.ui_items.len(), 2);
        assert_eq!(result.ui_items[0].kind, LanguageUiKind::Check);
        assert_eq!(result.ui_items[0].span, span(2, 3));
        assert_eq!(result.ui_items[1].kind, LanguageUiKind::Group);
        assert_eq!(result.ui_items[1].span, span(8, 8));
        assert!(result.ui_items.iter().all(|item| item.name.enabled));
    }

    #[test]
    fn applies_tips_and_nolang_in_either_order_to_the_next_ui() {
        for (directives, tips_span) in [
            (
                vec![tips("description", 1), nolang(vec![NolangTarget::Name], 2)],
                span(1, 1),
            ),
            (
                vec![nolang(vec![NolangTarget::Name], 1), tips("description", 2)],
                span(2, 2),
            ),
        ] {
            let result = build(
                vec![ui(LanguageUiKind::Check, 10, 11, LanguageUiMeta::None)],
                directives,
            )
            .unwrap();

            assert!(!result.ui_items[0].name.enabled);
            assert_eq!(
                result.ui_items[0].tips,
                Some(TipsText {
                    value: "description".to_string(),
                    span: tips_span,
                })
            );
        }
    }

    #[test]
    fn script_events_do_not_clear_ui_pending_and_are_independent() {
        let result = build(
            vec![ui(
                LanguageUiKind::Track,
                20,
                25,
                LanguageUiMeta::Track {
                    zero_label: Some("Zero".to_string()),
                },
            )],
            vec![
                tips("UI tips", 1),
                script_tips("Script tips", 3),
                nolang(vec![NolangTarget::ScriptName], 5),
                nolang(vec![NolangTarget::Name], 7),
            ],
        )
        .unwrap();

        assert!(!result.script_name.enabled);
        assert_eq!(result.script_tips.unwrap().value, "Script tips");
        assert!(!result.ui_items[0].name.enabled);
        assert_eq!(result.ui_items[0].tips.as_ref().unwrap().value, "UI tips");
    }

    #[test]
    fn splits_mixed_script_name_and_ui_targets_from_one_nolang_event() {
        let result = build(
            vec![ui(LanguageUiKind::Check, 3, 4, LanguageUiMeta::None)],
            vec![nolang(
                vec![NolangTarget::ScriptName, NolangTarget::Name],
                1,
            )],
        )
        .unwrap();

        assert!(!result.script_name.enabled);
        assert!(!result.ui_items[0].name.enabled);
    }

    #[test]
    fn combines_multiple_nolang_events_and_treats_ui_targets_idempotently() {
        let result = build(
            vec![ui(
                LanguageUiKind::Track,
                5,
                8,
                LanguageUiMeta::Track {
                    zero_label: Some("Zero".to_string()),
                },
            )],
            vec![
                nolang(vec![NolangTarget::Name, NolangTarget::ZeroLabel], 1),
                nolang(
                    vec![
                        NolangTarget::Name,
                        NolangTarget::ZeroLabel,
                        NolangTarget::Name,
                    ],
                    3,
                ),
            ],
        )
        .unwrap();

        assert!(!result.ui_items[0].name.enabled);
        let LanguageUiInfoMeta::Track {
            zero_label: Some(zero_label),
        } = &result.ui_items[0].meta
        else {
            panic!("expected track metadata");
        };
        assert!(!zero_label.enabled);
    }

    #[test]
    fn name_can_disable_group_and_separator() {
        let result = build(
            vec![
                ui(LanguageUiKind::Group, 2, 2, LanguageUiMeta::None),
                ui(LanguageUiKind::Separator, 4, 4, LanguageUiMeta::None),
            ],
            vec![
                nolang(vec![NolangTarget::Name], 1),
                nolang(vec![NolangTarget::Name], 3),
            ],
        )
        .unwrap();

        assert!(result.ui_items.iter().all(|item| !item.name.enabled));
    }

    #[test]
    fn normalizes_empty_zero_label_to_none() {
        let result = build(
            vec![ui(
                LanguageUiKind::Track,
                1,
                2,
                LanguageUiMeta::Track {
                    zero_label: Some(String::new()),
                },
            )],
            vec![],
        )
        .unwrap();

        assert_eq!(
            result.ui_items[0].meta,
            LanguageUiInfoMeta::Track { zero_label: None }
        );
    }

    #[test]
    fn applies_options_and_option_targets_including_duplicate_names() {
        let result = build(
            vec![
                ui(
                    LanguageUiKind::Select,
                    4,
                    5,
                    LanguageUiMeta::Select {
                        options: vec!["same".to_string(), "same".to_string(), "other".to_string()],
                    },
                ),
                ui(
                    LanguageUiKind::Select,
                    8,
                    9,
                    LanguageUiMeta::Select { options: vec![] },
                ),
            ],
            vec![
                nolang(
                    vec![
                        NolangTarget::Options,
                        NolangTarget::Option("same".to_string()),
                    ],
                    1,
                ),
                nolang(vec![NolangTarget::Options], 7),
            ],
        )
        .unwrap();

        let LanguageUiInfoMeta::Select { options } = &result.ui_items[0].meta else {
            panic!("expected select metadata");
        };
        assert!(options.iter().all(|option| !option.enabled));
        assert_eq!(
            result.ui_items[1].meta,
            LanguageUiInfoMeta::Select { options: vec![] }
        );
    }

    #[test]
    fn option_matching_is_case_sensitive_and_does_not_normalize_unicode() {
        let result = build(
            vec![ui(
                LanguageUiKind::Select,
                3,
                4,
                LanguageUiMeta::Select {
                    options: vec![
                        "A".to_string(),
                        "a".to_string(),
                        "é".to_string(),
                        "e\u{301}".to_string(),
                    ],
                },
            )],
            vec![nolang(
                vec![
                    NolangTarget::Option("a".to_string()),
                    NolangTarget::Option("e\u{301}".to_string()),
                ],
                1,
            )],
        )
        .unwrap();

        let LanguageUiInfoMeta::Select { options } = &result.ui_items[0].meta else {
            panic!("expected select metadata");
        };
        assert_eq!(
            options
                .iter()
                .map(|option| option.enabled)
                .collect::<Vec<_>>(),
            vec![true, false, true, false]
        );
    }

    #[test]
    fn rejects_invalid_spans_and_unordered_inputs() {
        let invalid_ui = ui(LanguageUiKind::Check, 0, 0, LanguageUiMeta::None);
        assert!(matches!(
            build(vec![invalid_ui], vec![]),
            Err(LanguageScriptInfoError::InvalidSpan { .. })
        ));

        let mut invalid_directive = tips("invalid", 3);
        invalid_directive.span.end_line = 2;
        assert!(matches!(
            build(vec![], vec![invalid_directive]),
            Err(LanguageScriptInfoError::InvalidSpan { .. })
        ));

        assert!(matches!(
            build(
                vec![
                    ui(LanguageUiKind::Check, 5, 5, LanguageUiMeta::None),
                    ui(LanguageUiKind::Color, 3, 3, LanguageUiMeta::None),
                ],
                vec![],
            ),
            Err(LanguageScriptInfoError::UnorderedUiItems { .. })
        ));

        assert!(matches!(
            build(vec![], vec![tips("later", 5), script_tips("earlier", 3)],),
            Err(LanguageScriptInfoError::UnorderedDirectives { .. })
        ));
    }

    #[test]
    fn rejects_overlaps_within_and_between_event_lists() {
        assert!(matches!(
            build(
                vec![
                    ui(LanguageUiKind::Check, 1, 3, LanguageUiMeta::None),
                    ui(LanguageUiKind::Color, 3, 4, LanguageUiMeta::None),
                ],
                vec![],
            ),
            Err(LanguageScriptInfoError::OverlappingUiItems { .. })
        ));

        let mut first = tips("first", 1);
        first.span.end_line = 3;
        assert!(matches!(
            build(vec![], vec![first, script_tips("second", 3)]),
            Err(LanguageScriptInfoError::OverlappingDirectives { .. })
        ));

        for directive_line in [1, 2] {
            assert!(matches!(
                build(
                    vec![ui(
                        LanguageUiKind::Track,
                        1,
                        3,
                        LanguageUiMeta::Track { zero_label: None }
                    )],
                    vec![tips("overlap", directive_line)],
                ),
                Err(LanguageScriptInfoError::OverlappingUiAndDirective { .. })
            ));
        }

        let mut enclosing_directive = tips("overlap", 1);
        enclosing_directive.span.end_line = 3;
        assert!(matches!(
            build(
                vec![ui(LanguageUiKind::Check, 2, 2, LanguageUiMeta::None)],
                vec![enclosing_directive],
            ),
            Err(LanguageScriptInfoError::OverlappingUiAndDirective { .. })
        ));
    }

    #[test]
    fn duplicate_tips_and_script_tips_are_errors_with_both_spans() {
        assert_eq!(
            build(
                vec![ui(LanguageUiKind::Check, 5, 6, LanguageUiMeta::None)],
                vec![tips("first", 1), tips("second", 3)],
            )
            .unwrap_err(),
            LanguageScriptInfoError::DuplicateTips {
                first_span: span(1, 1),
                duplicate_span: span(3, 3),
            }
        );

        assert_eq!(
            build(
                vec![],
                vec![script_tips("first", 1), script_tips("second", 3)],
            )
            .unwrap_err(),
            LanguageScriptInfoError::DuplicateScriptTips {
                first_span: span(1, 1),
                duplicate_span: span(3, 3),
            }
        );
    }

    #[test]
    fn unsupported_ui_kinds_reject_tips() {
        for kind in [
            LanguageUiKind::Param,
            LanguageUiKind::Group,
            LanguageUiKind::Separator,
        ] {
            assert!(matches!(
                build(
                    vec![ui(kind, 2, 2, LanguageUiMeta::None)],
                    vec![tips("unsupported", 1)],
                ),
                Err(LanguageScriptInfoError::TipsNotSupported { kind: actual, .. })
                    if actual == kind
            ));
        }
    }

    #[test]
    fn duplicate_script_name_targets_are_errors_within_or_across_events() {
        for directives in [
            vec![nolang(
                vec![NolangTarget::ScriptName, NolangTarget::ScriptName],
                1,
            )],
            vec![
                nolang(vec![NolangTarget::ScriptName], 1),
                nolang(vec![NolangTarget::ScriptName], 3),
            ],
        ] {
            assert!(matches!(
                build(vec![], directives),
                Err(LanguageScriptInfoError::DuplicateScriptNameTarget { .. })
            ));
        }
    }

    #[test]
    fn rejects_nolang_targets_for_unsupported_ui_kinds() {
        for target in [
            NolangTarget::ZeroLabel,
            NolangTarget::Options,
            NolangTarget::Option("value".to_string()),
        ] {
            let error = build(
                vec![ui(LanguageUiKind::Check, 2, 3, LanguageUiMeta::None)],
                vec![nolang(vec![target.clone()], 1)],
            )
            .unwrap_err();
            assert!(matches!(
                error,
                LanguageScriptInfoError::NolangTargetNotSupported {
                    target: actual,
                    directive_span,
                    ui_span,
                    ..
                } if actual == target
                    && directive_span == span(1, 1)
                    && ui_span == span(2, 3)
            ));
        }
    }

    #[test]
    fn nolang_application_errors_use_the_first_span_for_duplicate_ui_targets() {
        assert_eq!(
            build(
                vec![ui(LanguageUiKind::Check, 5, 6, LanguageUiMeta::None)],
                vec![
                    nolang(vec![NolangTarget::ZeroLabel], 1),
                    nolang(vec![NolangTarget::ZeroLabel], 3),
                ],
            )
            .unwrap_err(),
            LanguageScriptInfoError::NolangTargetNotSupported {
                target: NolangTarget::ZeroLabel,
                directive_span: span(1, 1),
                kind: LanguageUiKind::Check,
                ui_span: span(5, 6),
            }
        );
    }

    #[test]
    fn missing_or_empty_zero_label_is_an_error() {
        for zero_label in [None, Some(String::new())] {
            assert!(matches!(
                build(
                    vec![ui(
                        LanguageUiKind::Track,
                        2,
                        3,
                        LanguageUiMeta::Track { zero_label }
                    )],
                    vec![nolang(vec![NolangTarget::ZeroLabel], 1)],
                ),
                Err(LanguageScriptInfoError::MissingZeroLabel { .. })
            ));
        }
    }

    #[test]
    fn missing_option_is_an_error_even_when_options_is_also_present() {
        for targets in [
            vec![NolangTarget::Option("missing".to_string())],
            vec![
                NolangTarget::Options,
                NolangTarget::Option("missing".to_string()),
            ],
        ] {
            assert_eq!(
                build(
                    vec![ui(
                        LanguageUiKind::Select,
                        2,
                        3,
                        LanguageUiMeta::Select {
                            options: vec!["present".to_string()],
                        }
                    )],
                    vec![nolang(targets, 1)],
                )
                .unwrap_err(),
                LanguageScriptInfoError::MissingSelectOption {
                    option_name: "missing".to_string(),
                    directive_span: span(1, 1),
                    ui_span: span(2, 3),
                }
            );
        }
    }

    #[test]
    fn reports_unconsumed_tips_and_nolang() {
        assert_eq!(
            build(vec![], vec![tips("orphan", 2)]).unwrap_err(),
            LanguageScriptInfoError::UnconsumedTips { span: span(2, 2) }
        );
        assert_eq!(
            build(vec![], vec![nolang(vec![NolangTarget::Name], 4)],).unwrap_err(),
            LanguageScriptInfoError::UnconsumedNolang { span: span(4, 4) }
        );
    }
}
