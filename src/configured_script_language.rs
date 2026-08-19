use std::collections::HashSet;
use std::path::PathBuf;

use thiserror::Error;

use crate::config::ResolvedConfig;
use crate::configured_script::{
    ConfiguredLogicalScript, ConfiguredScriptError, resolve_indexed_configured_scripts,
};
use crate::configured_script_body::{
    PrepareConfiguredLogicalScriptError, PreparedConfiguredLogicalScript,
    prepare_configured_logical_script,
};
use crate::language_ui::LanguageScriptSyntax;
use crate::logical_script_language::{
    LogicalScriptLanguage, LogicalScriptLanguageError, analyze_logical_script_language,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalyzedConfiguredLogicalScript {
    pub configured_script_index: Option<usize>,
    pub prepared: PreparedConfiguredLogicalScript,
    pub language: LogicalScriptLanguage,
}

#[derive(Debug, Error)]
pub(crate) enum AnalyzeConfiguredLogicalScriptError {
    #[error(transparent)]
    Prepare(#[from] PrepareConfiguredLogicalScriptError),
    #[error("configured論理スクリプト {script_name} のlanguage解析に失敗しました: {source}")]
    Language {
        script_name: String,
        source_paths: Vec<PathBuf>,
        #[source]
        source: Box<LogicalScriptLanguageError>,
    },
}

#[derive(Debug, Error)]
pub(crate) enum AnalyzeConfiguredScriptsError {
    #[error(transparent)]
    Resolve(#[from] ConfiguredScriptError),
    #[error(transparent)]
    LogicalScript(#[from] AnalyzeConfiguredLogicalScriptError),
}

pub(crate) fn analyze_configured_logical_script(
    config: &ResolvedConfig,
    logical_script: &ConfiguredLogicalScript<'_>,
) -> Result<AnalyzedConfiguredLogicalScript, AnalyzeConfiguredLogicalScriptError> {
    let prepared = prepare_configured_logical_script(config, logical_script)?;
    let language = analyze_logical_script_language(
        &prepared.name,
        &prepared.body,
        LanguageScriptSyntax::AuluaSource,
    )
    .map_err(|source| AnalyzeConfiguredLogicalScriptError::Language {
        script_name: prepared.name.clone(),
        source_paths: prepared.source_paths.clone(),
        source: Box::new(source),
    })?;

    Ok(AnalyzedConfiguredLogicalScript {
        configured_script_index: None,
        prepared,
        language,
    })
}

#[cfg(test)]
pub(crate) fn analyze_configured_scripts(
    config: &ResolvedConfig,
) -> Result<Vec<AnalyzedConfiguredLogicalScript>, AnalyzeConfiguredScriptsError> {
    analyze_configured_scripts_selected(config, None)
}

pub(crate) fn analyze_configured_scripts_selected(
    config: &ResolvedConfig,
    selected_script_indices: Option<&HashSet<usize>>,
) -> Result<Vec<AnalyzedConfiguredLogicalScript>, AnalyzeConfiguredScriptsError> {
    // Resolve every configured script first so that structural validation and the
    // global logical-script-name uniqueness constraint remain independent of the
    // language-file scopes selected for content analysis.
    let configured_files = resolve_indexed_configured_scripts(&config.scripts)?;
    let mut analyzed = Vec::new();

    for configured_file in configured_files {
        if selected_script_indices
            .is_some_and(|indices| !indices.contains(&configured_file.script_index))
        {
            continue;
        }

        for logical_script in &configured_file.file.logical_scripts {
            let analyzed_script = analyze_configured_logical_script(config, logical_script)?;
            analyzed.push(AnalyzedConfiguredLogicalScript {
                configured_script_index: Some(configured_file.script_index),
                prepared: analyzed_script.prepared,
                language: analyzed_script.language,
            });
        }
    }

    Ok(analyzed)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::config::{
        ResolvedBuild, ResolvedInstall, ResolvedProject, ResolvedScript, ResolvedScriptSource,
    };
    use crate::configured_script::ConfiguredScriptError;
    use crate::configured_script_body::{
        ConfiguredSourceWarning, PrepareConfiguredLogicalScriptError,
    };
    use crate::language_script_info::LanguageUiInfoMeta;
    use crate::language_ui::{LanguageUiExtractError, LanguageUiKind, SourceSpan};
    use crate::logical_script_language::LogicalScriptLanguageError;
    use crate::source_processing::SourceProcessingError;

    fn config(scripts: Vec<ResolvedScript>) -> ResolvedConfig {
        ResolvedConfig {
            project: ResolvedProject {
                variables: HashMap::new(),
            },
            build: ResolvedBuild {
                out_dir: PathBuf::new(),
                embed_search_dirs: Vec::new(),
            },
            install: ResolvedInstall {
                out_dir: PathBuf::new(),
            },
            package: None,
            language: None,
            scripts,
            config_dir: PathBuf::new(),
        }
    }

    fn source(path: impl Into<PathBuf>, label: Option<&str>) -> ResolvedScriptSource {
        ResolvedScriptSource {
            path: path.into(),
            label: label.map(str::to_string),
            variables: HashMap::new(),
        }
    }

    fn script(name: &str, sources: Vec<ResolvedScriptSource>) -> ResolvedScript {
        ResolvedScript {
            name: name.to_string(),
            sources,
            language: None,
        }
    }

    fn logical_script<'a>(
        name: &str,
        sources: &'a [ResolvedScriptSource],
    ) -> ConfiguredLogicalScript<'a> {
        ConfiguredLogicalScript {
            name: name.to_string(),
            sources,
        }
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn applies_pending_tips_across_source_boundary_with_combined_spans() {
        let temp = TempDir::new().unwrap();
        let tips_path = temp.path().join("tips.lua");
        let ui_path = temp.path().join("ui.lua");
        write(&tips_path, "---$tips:Description");
        write(&ui_path, "---$check:Enabled\nlocal enabled = false");
        let sources = vec![source(&tips_path, None), source(&ui_path, None)];

        let result = analyze_configured_logical_script(
            &config(Vec::new()),
            &logical_script("Script", &sources),
        )
        .unwrap();

        assert_eq!(
            result.prepared.body,
            "---$tips:Description\n\n---$check:Enabled\nlocal enabled = false"
        );
        let ui = &result.language.info.ui_items[0];
        assert_eq!(ui.kind, LanguageUiKind::Check);
        assert_eq!(ui.tips.as_ref().unwrap().value, "Description");
        assert_eq!(
            ui.tips.as_ref().unwrap().span,
            SourceSpan {
                start_line: 1,
                end_line: 1,
            }
        );
        assert_eq!(
            ui.span,
            SourceSpan {
                start_line: 3,
                end_line: 4,
            }
        );
        assert_eq!(result.language.entries.tips_entries.len(), 1);
        assert_eq!(result.language.entries.tips_entries[0].key, "Enabled");
        assert_eq!(result.language.entries.tips_entries[0].value, "Description");
    }

    #[test]
    fn applies_pending_nolang_across_source_boundary() {
        let temp = TempDir::new().unwrap();
        let nolang_path = temp.path().join("nolang.lua");
        let ui_path = temp.path().join("ui.lua");
        write(&nolang_path, "---$nolang: name");
        write(&ui_path, "---$check:Enabled\nlocal enabled = false");
        let sources = vec![source(&nolang_path, None), source(&ui_path, None)];

        let result = analyze_configured_logical_script(
            &config(Vec::new()),
            &logical_script("Script", &sources),
        )
        .unwrap();

        assert!(result.language.info.script_name.enabled);
        assert!(!result.language.info.ui_items[0].name.enabled);
        assert_eq!(result.language.entries.text_entries.len(), 1);
        assert_eq!(result.language.entries.text_entries[0].key, "Script");
        assert!(
            !result
                .language
                .entries
                .text_entries
                .iter()
                .any(|entry| entry.key == "Enabled")
        );
    }

    #[test]
    fn preserves_source_order_warnings_after_successful_analysis() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("first.lua");
        let second_path = temp.path().join("second.lua");
        write(&first_path, "local first = \"${MISSING}\"");
        write(&second_path, "local second = \"${MISSING}\"");
        let sources = vec![source(&first_path, None), source(&second_path, None)];

        let result = analyze_configured_logical_script(
            &config(Vec::new()),
            &logical_script("Script", &sources),
        )
        .unwrap();

        assert_eq!(
            result.prepared.warnings,
            vec![
                ConfiguredSourceWarning {
                    source_path: first_path,
                    undefined_variable: "MISSING".to_string(),
                },
                ConfiguredSourceWarning {
                    source_path: second_path,
                    undefined_variable: "MISSING".to_string(),
                },
            ]
        );
        assert_eq!(result.language.info.script_name.value, "Script");
        assert!(result.language.info.ui_items.is_empty());
    }

    #[test]
    fn wraps_prepare_error_transparently() {
        let temp = TempDir::new().unwrap();
        let missing_path = temp.path().join("missing.lua");
        let sources = vec![source(&missing_path, None)];

        let error = analyze_configured_logical_script(
            &config(Vec::new()),
            &logical_script("Script", &sources),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            AnalyzeConfiguredLogicalScriptError::Prepare(
                PrepareConfiguredLogicalScriptError::Source {
                    script_name,
                    source_path,
                    source: SourceProcessingError::Read { .. },
                }
            ) if script_name == "Script" && source_path == missing_path
        ));
    }

    #[test]
    fn incomplete_ui_does_not_cross_source_boundary_and_has_language_context() {
        let temp = TempDir::new().unwrap();
        let directive_path = temp.path().join("directive.lua");
        let assignment_path = temp.path().join("assignment.lua");
        write(&directive_path, "---$track:X");
        write(&assignment_path, "local x = 0");
        let sources = vec![
            source(&directive_path, None),
            source(&assignment_path, None),
        ];

        let error = analyze_configured_logical_script(
            &config(Vec::new()),
            &logical_script("Script", &sources),
        )
        .unwrap_err();

        let AnalyzeConfiguredLogicalScriptError::Language {
            script_name,
            source_paths,
            source,
        } = error
        else {
            panic!("unexpected error variant");
        };
        assert_eq!(script_name, "Script");
        assert_eq!(source_paths, vec![directive_path, assignment_path]);
        assert_eq!(
            *source,
            LogicalScriptLanguageError::UiExtract(LanguageUiExtractError::InvalidSourceSyntax {
                kind: LanguageUiKind::Track,
                line_number: 1,
            })
        );
    }

    #[test]
    fn analyzes_config_in_script_and_label_order_while_skipping_unsupported_and_preamble() {
        let temp = TempDir::new().unwrap();
        let normal_path = temp.path().join("normal.lua");
        let preamble_path = temp.path().join("preamble.lua");
        let first_path = temp.path().join("first.lua");
        let second_path = temp.path().join("second.lua");
        write(&normal_path, "local normal = true");
        write(&preamble_path, "---$track:Invalid preamble");
        write(&first_path, "local first = true");
        write(&second_path, "local second = true");
        let config = config(vec![
            script("normal.anm2", vec![source(&normal_path, None)]),
            script("ignored.lua", Vec::new()),
            script(
                "@container.obj2",
                vec![
                    source(&preamble_path, None),
                    source(&first_path, Some("First")),
                    source(&second_path, Some("Second")),
                ],
            ),
        ]);

        let result = analyze_configured_scripts(&config).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["normal", "First@container", "Second@container"]
        );
        assert_eq!(result[0].prepared.source_paths, vec![normal_path]);
        assert_eq!(result[1].prepared.source_paths, vec![first_path]);
        assert_eq!(result[2].prepared.source_paths, vec![second_path]);
        assert!(
            result
                .iter()
                .all(|script| !script.prepared.body.contains("Invalid preamble"))
        );
        assert!(result[1].prepared.body.contains("local first = true"));
        assert!(!result[1].prepared.body.contains("@First"));
    }

    #[test]
    fn wraps_configured_resolution_error() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("source.lua");
        write(&source_path, "Body");
        let config = config(vec![
            script("same.anm2", vec![source(&source_path, None)]),
            script("same.obj2", vec![source(&source_path, None)]),
        ]);

        let error = analyze_configured_scripts(&config).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeConfiguredScriptsError::Resolve(
                ConfiguredScriptError::DuplicateLogicalScriptName { name }
            ) if name == "same"
        ));
    }

    #[test]
    fn wraps_logical_script_error_from_config_entry_point() {
        let temp = TempDir::new().unwrap();
        let missing_path = temp.path().join("missing.lua");
        let config = config(vec![script(
            "broken.anm2",
            vec![source(&missing_path, None)],
        )]);

        let error = analyze_configured_scripts(&config).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeConfiguredScriptsError::LogicalScript(
                AnalyzeConfiguredLogicalScriptError::Prepare(
                    PrepareConfiguredLogicalScriptError::Source {
                        script_name,
                        source_path,
                        source: SourceProcessingError::Read { .. },
                    }
                )
            ) if script_name == "broken" && source_path == missing_path
        ));
    }

    #[test]
    fn keeps_track_metadata_from_source_syntax() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("track.lua");
        write(&source_path, "---$track:X\n---zero_label=Stop\nlocal x = 0");
        let sources = vec![source(&source_path, None)];

        let result = analyze_configured_logical_script(
            &config(Vec::new()),
            &logical_script("Script", &sources),
        )
        .unwrap();

        assert!(matches!(
            &result.language.info.ui_items[0].meta,
            LanguageUiInfoMeta::Track {
                zero_label: Some(zero_label)
            } if zero_label.value == "Stop"
        ));
    }
}
