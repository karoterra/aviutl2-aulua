use thiserror::Error;

use crate::language_directive::{
    LanguageDirective, LanguageDirectiveExtractError, LanguageDirectiveKind, LanguageDirectiveName,
    extract_language_directives,
};
use crate::language_script_entries::{
    LanguageScriptEntries, LanguageScriptEntriesError, build_language_script_entries,
};
use crate::language_script_info::{
    LanguageScriptInfo, LanguageScriptInfoError, build_language_script_info,
};
use crate::language_ui::{
    LanguageScriptSyntax, LanguageUiExtractError, SourceSpan, extract_language_ui_items,
    extract_tra2_language_ui_items,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguageScriptFormat {
    Standard(LanguageScriptSyntax),
    Tra2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalScriptLanguage {
    pub info: LanguageScriptInfo,
    pub entries: LanguageScriptEntries,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LogicalScriptLanguageError {
    #[error(transparent)]
    UiExtract(#[from] LanguageUiExtractError),
    #[error(transparent)]
    DirectiveExtract(#[from] LanguageDirectiveExtractError),
    #[error(".tra2ではlanguage Tipsを使用できません: {directive:?} {span:?}")]
    Tra2TipsNotSupported {
        directive: LanguageDirectiveName,
        span: SourceSpan,
    },
    #[error(transparent)]
    ScriptInfo(#[from] LanguageScriptInfoError),
    #[error(transparent)]
    ScriptEntries(#[from] LanguageScriptEntriesError),
}

pub fn analyze_logical_script_language(
    script_name: &str,
    body: &str,
    syntax: LanguageScriptSyntax,
) -> Result<LogicalScriptLanguage, LogicalScriptLanguageError> {
    analyze_logical_script_language_with_format(
        script_name,
        body,
        LanguageScriptFormat::Standard(syntax),
    )
}

pub(crate) fn analyze_logical_script_language_with_format(
    script_name: &str,
    body: &str,
    format: LanguageScriptFormat,
) -> Result<LogicalScriptLanguage, LogicalScriptLanguageError> {
    let ui_items = match format {
        LanguageScriptFormat::Standard(syntax) => extract_language_ui_items(body, syntax)?,
        LanguageScriptFormat::Tra2 => extract_tra2_language_ui_items(body),
    };
    let directives = extract_language_directives(body)?;
    if format == LanguageScriptFormat::Tra2 {
        validate_tra2_directives(&directives)?;
    }
    let info = build_language_script_info(script_name, ui_items, directives)?;
    let entries = build_language_script_entries(&info)?;

    Ok(LogicalScriptLanguage { info, entries })
}

fn validate_tra2_directives(
    directives: &[LanguageDirective],
) -> Result<(), LogicalScriptLanguageError> {
    for directive in directives {
        let directive_name = match directive.kind {
            LanguageDirectiveKind::Tips { .. } => Some(LanguageDirectiveName::Tips),
            LanguageDirectiveKind::ScriptTips { .. } => Some(LanguageDirectiveName::ScriptTips),
            LanguageDirectiveKind::Nolang { .. } => None,
        };

        if let Some(directive_name) = directive_name {
            return Err(LogicalScriptLanguageError::Tra2TipsNotSupported {
                directive: directive_name,
                span: directive.span,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_directive::LanguageDirectiveExtractError;
    use crate::language_script_entries::{
        LanguageScriptEntriesError, LanguageTextOrigin, LanguageTipsOrigin,
    };
    use crate::language_script_info::{LanguageScriptInfoError, LanguageUiInfoMeta};
    use crate::language_ui::{LanguageUiExtractError, LanguageUiKind, SourceSpan};

    #[test]
    fn analyzes_tra2_params_exclusively_and_aggregates_duplicate_keys() {
        let body = concat!(
            "--track@vx:X速度,-10,10,0\n",
            "---$nolang:name\n",
            "--param:非翻訳,1\n",
            "--param:aaa::周期,0.5\n",
            "--param:周期,0.5\n",
            "--param:周期,1.0\n",
            "--param:空,\n",
            "--param:チェック/check,0\n",
            "--param:選択/select/A=0/B=1,0\n",
        );

        let result = analyze_logical_script_language_with_format(
            "Transition",
            body,
            LanguageScriptFormat::Tra2,
        )
        .unwrap();

        assert_eq!(
            result
                .info
                .ui_items
                .iter()
                .map(|item| (
                    item.kind,
                    item.name.name.original.as_str(),
                    item.name.enabled
                ))
                .collect::<Vec<_>>(),
            vec![
                (LanguageUiKind::Param, "非翻訳", false),
                (LanguageUiKind::Param, "aaa::周期", true),
                (LanguageUiKind::Param, "周期", true),
                (LanguageUiKind::Param, "周期", true),
            ]
        );
        assert_eq!(
            result
                .entries
                .text_entries
                .iter()
                .map(|entry| (entry.key.as_str(), entry.origins.len()))
                .collect::<Vec<_>>(),
            vec![("Transition", 1), ("aaa::周期", 1), ("周期", 2)]
        );
    }

    #[test]
    fn rejects_each_tips_directive_in_tra2() {
        for (body, expected) in [
            (
                "---$tips:Param tips\n--param:周期,0.5\n",
                LanguageDirectiveName::Tips,
            ),
            (
                "---$script_tips:Script tips\n",
                LanguageDirectiveName::ScriptTips,
            ),
        ] {
            assert!(matches!(
                analyze_logical_script_language_with_format(
                    "Transition",
                    body,
                    LanguageScriptFormat::Tra2,
                ),
                Err(LogicalScriptLanguageError::Tra2TipsNotSupported {
                    directive,
                    ..
                }) if directive == expected
            ));
        }
    }

    #[test]
    fn analyzes_source_language_end_to_end() {
        let body = concat!(
            "---$script_tips:Script tips\n",
            "---$tips:Track tips\n",
            "---$track:namespace::Shared\n",
            "---zero_label=Shared\n",
            "local track = 0\n",
            "\n",
            "---$nolang: name, option:Disabled\n",
            "---$select:Shared\n",
            "---Shared=0\n",
            "---Disabled=1\n",
            "local select = 0\n",
        );

        let result =
            analyze_logical_script_language("Shared", body, LanguageScriptSyntax::AuluaSource)
                .unwrap();

        assert_eq!(result.info.script_name.value, "Shared");
        assert!(result.info.script_name.enabled);
        assert_eq!(
            result.info.script_tips.as_ref().unwrap().value,
            "Script tips"
        );
        assert_eq!(result.info.ui_items.len(), 2);

        let track = &result.info.ui_items[0];
        assert_eq!(track.name.name.original, "namespace::Shared");
        assert_eq!(track.name.name.translation_key, "Shared");
        assert!(track.name.enabled);
        assert_eq!(track.tips.as_ref().unwrap().value, "Track tips");
        assert!(matches!(
            &track.meta,
            LanguageUiInfoMeta::Track {
                zero_label: Some(zero_label)
            } if zero_label.value == "Shared" && zero_label.enabled
        ));

        let select = &result.info.ui_items[1];
        assert!(!select.name.enabled);
        assert!(matches!(
            &select.meta,
            LanguageUiInfoMeta::Select { options }
                if options[0].value == "Shared"
                    && options[0].enabled
                    && options[1].value == "Disabled"
                    && !options[1].enabled
        ));

        assert_eq!(result.entries.text_entries.len(), 1);
        let shared = &result.entries.text_entries[0];
        assert_eq!(shared.key, "Shared");
        assert_eq!(shared.origins.len(), 4);
        assert_eq!(shared.origins[0], LanguageTextOrigin::ScriptName);
        assert!(matches!(
            shared.origins[1],
            LanguageTextOrigin::UiName {
                kind: LanguageUiKind::Track,
                ..
            }
        ));
        assert!(matches!(
            shared.origins[2],
            LanguageTextOrigin::TrackZeroLabel { .. }
        ));
        assert!(matches!(
            shared.origins[3],
            LanguageTextOrigin::SelectOption {
                option_index: 1,
                ..
            }
        ));

        assert_eq!(result.entries.tips_entries.len(), 2);
        assert_eq!(result.entries.tips_entries[0].key, "effect.name");
        assert_eq!(result.entries.tips_entries[1].key, "namespace::Shared");
        assert!(matches!(
            result.entries.tips_entries[0].origin,
            LanguageTipsOrigin::ScriptTips { .. }
        ));
        assert!(matches!(
            result.entries.tips_entries[1].origin,
            LanguageTipsOrigin::UiTips {
                kind: LanguageUiKind::Track,
                ..
            }
        ));
    }

    #[test]
    fn analyzes_built_language_directives_and_ui() {
        let body = concat!(
            "---$tips:Track tips\n",
            "--track@track:namespace::Track,-10,10,0,1,Zero,0.1\n",
            "---$nolang: option:B\n",
            "--select@select:Select=0,A=0,B=1\n",
            "--group:Group\n",
            "--separator:Separator\n",
        );

        let result =
            analyze_logical_script_language("Built", body, LanguageScriptSyntax::BuiltScript)
                .unwrap();

        assert_eq!(
            result
                .info
                .ui_items
                .iter()
                .map(|item| item.kind)
                .collect::<Vec<_>>(),
            vec![
                LanguageUiKind::Track,
                LanguageUiKind::Select,
                LanguageUiKind::Group,
                LanguageUiKind::Separator,
            ]
        );
        assert_eq!(result.entries.tips_entries[0].key, "namespace::Track");
        assert_eq!(result.entries.tips_entries[0].value, "Track tips");
        assert_eq!(
            result
                .entries
                .text_entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec![
                "Built",
                "Track",
                "Zero",
                "Select",
                "A",
                "Group",
                "Separator"
            ]
        );
    }

    #[test]
    fn analyzes_empty_body() {
        let result =
            analyze_logical_script_language("Empty", "", LanguageScriptSyntax::AuluaSource)
                .unwrap();

        assert!(result.info.ui_items.is_empty());
        assert!(result.info.script_tips.is_none());
        assert_eq!(result.entries.text_entries.len(), 1);
        assert_eq!(result.entries.text_entries[0].key, "Empty");
        assert_eq!(
            result.entries.text_entries[0].origins,
            vec![LanguageTextOrigin::ScriptName]
        );
        assert!(result.entries.tips_entries.is_empty());
    }

    #[test]
    fn nolang_script_name_can_leave_text_entries_empty() {
        let result = analyze_logical_script_language(
            "Hidden",
            "---$nolang: script_name",
            LanguageScriptSyntax::BuiltScript,
        )
        .unwrap();

        assert!(!result.info.script_name.enabled);
        assert!(result.entries.text_entries.is_empty());
        assert!(result.entries.tips_entries.is_empty());
    }

    #[test]
    fn analyzes_ui_without_language_directives() {
        let result = analyze_logical_script_language(
            "Only UI",
            "---$check:Enabled\nlocal enabled = false",
            LanguageScriptSyntax::AuluaSource,
        )
        .unwrap();

        assert_eq!(result.info.ui_items.len(), 1);
        assert_eq!(result.info.ui_items[0].kind, LanguageUiKind::Check);
        assert_eq!(
            result
                .entries
                .text_entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["Only UI", "Enabled"]
        );
        assert!(result.entries.tips_entries.is_empty());
    }

    #[test]
    fn preserves_crlf_line_basis_without_trailing_newline() {
        let body = "---$tips:First\r\n---:Second\r\n---$check:Check\r\nlocal check = false";

        let result =
            analyze_logical_script_language("CRLF", body, LanguageScriptSyntax::AuluaSource)
                .unwrap();

        let ui = &result.info.ui_items[0];
        let tips = ui.tips.as_ref().unwrap();
        assert_eq!(tips.value, "First\nSecond");
        assert_eq!(
            tips.span,
            SourceSpan {
                start_line: 1,
                end_line: 2,
            }
        );
        assert_eq!(ui.span.start_line, 3);
        assert_eq!(ui.span.end_line, 4);
    }

    #[test]
    fn wraps_ui_extract_error() {
        let error = analyze_logical_script_language(
            "Script",
            "---$track:X\nnot an assignment",
            LanguageScriptSyntax::AuluaSource,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LogicalScriptLanguageError::UiExtract(
                LanguageUiExtractError::InvalidSourceSyntax { .. }
            )
        ));
    }

    #[test]
    fn wraps_directive_extract_error() {
        let error = analyze_logical_script_language(
            "Script",
            "---$nolang: unknown",
            LanguageScriptSyntax::AuluaSource,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            LogicalScriptLanguageError::DirectiveExtract(
                LanguageDirectiveExtractError::UnknownNolangTarget { .. }
            )
        ));
    }

    #[test]
    fn wraps_script_info_error() {
        let error = analyze_logical_script_language(
            "Script",
            "---$tips:Orphan",
            LanguageScriptSyntax::AuluaSource,
        )
        .unwrap_err();

        assert_eq!(
            error,
            LogicalScriptLanguageError::ScriptInfo(LanguageScriptInfoError::UnconsumedTips {
                span: SourceSpan {
                    start_line: 1,
                    end_line: 1,
                },
            })
        );
    }

    #[test]
    fn wraps_script_entries_error() {
        let body = concat!(
            "--track@track:Same,-10,10,0\n",
            "--check@check:Same,false\n",
        );

        let error =
            analyze_logical_script_language("Script", body, LanguageScriptSyntax::BuiltScript)
                .unwrap_err();

        assert!(matches!(
            error,
            LogicalScriptLanguageError::ScriptEntries(
                LanguageScriptEntriesError::DuplicateValueUiName { .. }
            )
        ));
    }

    #[test]
    fn ui_extract_error_has_priority_over_directive_error() {
        let body = concat!(
            "---$track:X\n",
            "not an assignment\n",
            "---$nolang: unknown\n",
        );

        let error =
            analyze_logical_script_language("Script", body, LanguageScriptSyntax::AuluaSource)
                .unwrap_err();

        assert!(matches!(
            error,
            LogicalScriptLanguageError::UiExtract(
                LanguageUiExtractError::InvalidSourceSyntax { .. }
            )
        ));
    }
}
