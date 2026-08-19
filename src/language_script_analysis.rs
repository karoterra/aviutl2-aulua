use std::collections::HashSet;
use std::path::PathBuf;

use thiserror::Error;

use crate::config::ResolvedConfig;
use crate::configured_script_body::{ConfiguredSourceWarning, PreparedConfiguredLogicalScript};
use crate::configured_script_language::{
    AnalyzeConfiguredScriptsError, AnalyzedConfiguredLogicalScript,
    analyze_configured_scripts_selected,
};
use crate::direct_script_language::{
    AnalyzeDirectScriptsError, AnalyzedDirectLogicalScript, PreparedDirectLogicalScript,
    analyze_direct_scripts,
};
use crate::logical_script_language::LogicalScriptLanguage;

#[derive(Debug)]
pub(crate) enum LanguageScriptInput<'a> {
    Configured,
    Direct(&'a [PathBuf]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LogicalScriptOrigin {
    Configured { source_paths: Vec<PathBuf> },
    Direct { path: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LanguageAnalysisWarning {
    UndefinedVariable {
        source_path: PathBuf,
        variable_name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedLogicalScript {
    pub name: String,
    pub body: String,
    pub origin: LogicalScriptOrigin,
    pub warnings: Vec<LanguageAnalysisWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalyzedLogicalScript {
    pub configured_script_index: Option<usize>,
    pub prepared: PreparedLogicalScript,
    pub language: LogicalScriptLanguage,
}

impl From<ConfiguredSourceWarning> for LanguageAnalysisWarning {
    fn from(warning: ConfiguredSourceWarning) -> Self {
        let ConfiguredSourceWarning {
            source_path,
            undefined_variable,
        } = warning;

        Self::UndefinedVariable {
            source_path,
            variable_name: undefined_variable,
        }
    }
}

impl From<PreparedConfiguredLogicalScript> for PreparedLogicalScript {
    fn from(prepared: PreparedConfiguredLogicalScript) -> Self {
        let PreparedConfiguredLogicalScript {
            name,
            body,
            source_paths,
            warnings,
        } = prepared;

        Self {
            name,
            body,
            origin: LogicalScriptOrigin::Configured { source_paths },
            warnings: warnings.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<PreparedDirectLogicalScript> for PreparedLogicalScript {
    fn from(prepared: PreparedDirectLogicalScript) -> Self {
        let PreparedDirectLogicalScript {
            origin_path,
            name,
            body,
        } = prepared;

        Self {
            name,
            body,
            origin: LogicalScriptOrigin::Direct { path: origin_path },
            warnings: Vec::new(),
        }
    }
}

impl From<AnalyzedConfiguredLogicalScript> for AnalyzedLogicalScript {
    fn from(analyzed: AnalyzedConfiguredLogicalScript) -> Self {
        let AnalyzedConfiguredLogicalScript {
            configured_script_index,
            prepared,
            language,
        } = analyzed;

        Self {
            configured_script_index,
            prepared: prepared.into(),
            language,
        }
    }
}

impl From<AnalyzedDirectLogicalScript> for AnalyzedLogicalScript {
    fn from(analyzed: AnalyzedDirectLogicalScript) -> Self {
        let AnalyzedDirectLogicalScript { prepared, language } = analyzed;

        Self {
            configured_script_index: None,
            prepared: prepared.into(),
            language,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum AnalyzeLanguageScriptsError {
    #[error(transparent)]
    Configured(#[from] AnalyzeConfiguredScriptsError),
    #[error(transparent)]
    Direct(#[from] AnalyzeDirectScriptsError),
    #[error("directスクリプトが指定されていません")]
    EmptyDirectScripts,
}

pub(crate) fn analyze_language_scripts(
    config: &ResolvedConfig,
    input: LanguageScriptInput<'_>,
) -> Result<Vec<AnalyzedLogicalScript>, AnalyzeLanguageScriptsError> {
    analyze_language_scripts_selected(config, input, None)
}

pub(crate) fn analyze_language_scripts_selected(
    config: &ResolvedConfig,
    input: LanguageScriptInput<'_>,
    selected_configured_script_indices: Option<&HashSet<usize>>,
) -> Result<Vec<AnalyzedLogicalScript>, AnalyzeLanguageScriptsError> {
    match input {
        LanguageScriptInput::Configured => Ok(analyze_configured_scripts_selected(
            config,
            selected_configured_script_indices,
        )?
        .into_iter()
        .map(Into::into)
        .collect()),
        LanguageScriptInput::Direct(paths) => {
            if paths.is_empty() {
                return Err(AnalyzeLanguageScriptsError::EmptyDirectScripts);
            }

            Ok(analyze_direct_scripts(paths)?
                .into_iter()
                .map(Into::into)
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::*;
    use crate::config::{
        ResolvedBuild, ResolvedInstall, ResolvedProject, ResolvedScript, ResolvedScriptSource,
    };

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

    fn source(
        path: impl Into<PathBuf>,
        label: Option<&str>,
        variables: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> ResolvedScriptSource {
        ResolvedScriptSource {
            path: path.into(),
            label: label.map(str::to_string),
            variables: variables
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        }
    }

    fn script(name: &str, sources: Vec<ResolvedScriptSource>) -> ResolvedScript {
        ResolvedScript {
            name: name.to_string(),
            sources,
            language: None,
        }
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn configured_input_preserves_order_origins_warnings_body_and_language() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("first.lua");
        let second_path = temp.path().join("second.lua");
        let part_one_path = temp.path().join("part_one.lua");
        let part_two_path = temp.path().join("part_two.lua");
        write(&first_path, "local missing = \"${MISSING}\"");
        write(&second_path, "local second = true");
        write(&part_one_path, "local part_one = true");
        write(&part_two_path, "local part_two = true");
        let config = config(vec![
            script(
                "normal.anm2",
                vec![
                    source(&first_path, None, []),
                    source(&second_path, None, []),
                ],
            ),
            script(
                "@combined.obj2",
                vec![
                    source(&part_one_path, Some("Part One"), []),
                    source(&part_two_path, Some("Part Two"), []),
                ],
            ),
        ]);

        let result = analyze_language_scripts(&config, LanguageScriptInput::Configured).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["normal", "Part One@combined", "Part Two@combined"]
        );
        assert_eq!(
            result[0].prepared.origin,
            LogicalScriptOrigin::Configured {
                source_paths: vec![first_path.clone(), second_path.clone()],
            }
        );
        assert_eq!(
            result[0].prepared.warnings,
            vec![LanguageAnalysisWarning::UndefinedVariable {
                source_path: first_path,
                variable_name: "MISSING".to_string(),
            }]
        );
        assert!(result[0].prepared.body.contains("${MISSING}"));
        assert_eq!(result[0].language.info.script_name.value, "normal");
        assert_eq!(
            result[1].prepared.origin,
            LogicalScriptOrigin::Configured {
                source_paths: vec![part_one_path],
            }
        );
        assert_eq!(
            result[2].prepared.origin,
            LogicalScriptOrigin::Configured {
                source_paths: vec![part_two_path],
            }
        );
        assert!(
            result
                .iter()
                .all(|script| script.prepared.warnings.is_empty()
                    || script.prepared.name == "normal")
        );
    }

    #[test]
    fn direct_input_ignores_broken_configured_scripts_and_preserves_direct_order() {
        let temp = TempDir::new().unwrap();
        let missing_configured_path = temp.path().join("missing.lua");
        let normal_path = temp.path().join("normal.anm2");
        let multiple_path = temp.path().join("@combined.obj2");
        write(&normal_path, "--check@normal:Normal,false\n");
        write(
            &multiple_path,
            "@First\n--check@first:First,false\n@Second\n--check@second:Second,false\n",
        );
        let config = config(vec![script(
            "broken.anm2",
            vec![source(&missing_configured_path, None, [])],
        )]);
        let paths = [normal_path.clone(), multiple_path.clone()];

        let result =
            analyze_language_scripts(&config, LanguageScriptInput::Direct(&paths)).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["normal", "First@combined", "Second@combined"]
        );
        assert_eq!(
            result[0].prepared.origin,
            LogicalScriptOrigin::Direct {
                path: normal_path.clone(),
            }
        );
        assert_eq!(
            result[1].prepared.origin,
            LogicalScriptOrigin::Direct {
                path: multiple_path.clone(),
            }
        );
        assert_eq!(
            result[2].prepared.origin,
            LogicalScriptOrigin::Direct {
                path: multiple_path,
            }
        );
        assert!(
            result
                .iter()
                .all(|script| script.prepared.warnings.is_empty())
        );
        assert_eq!(result[0].prepared.body, "--check@normal:Normal,false\n");
        assert_eq!(result[0].language.info.ui_items.len(), 1);
    }

    #[test]
    fn configured_error_is_wrapped_by_input_path_variant() {
        let temp = TempDir::new().unwrap();
        let missing_path = temp.path().join("missing.lua");
        let config = config(vec![script(
            "broken.anm2",
            vec![source(&missing_path, None, [])],
        )]);

        let error = analyze_language_scripts(&config, LanguageScriptInput::Configured).unwrap_err();

        assert!(matches!(error, AnalyzeLanguageScriptsError::Configured(_)));
    }

    #[test]
    fn direct_error_is_wrapped_by_input_path_variant() {
        let temp = TempDir::new().unwrap();
        let missing_path = temp.path().join("missing.anm2");
        let config = config(Vec::new());

        let error = analyze_language_scripts(
            &config,
            LanguageScriptInput::Direct(std::slice::from_ref(&missing_path)),
        )
        .unwrap_err();

        assert!(matches!(error, AnalyzeLanguageScriptsError::Direct(_)));
    }

    #[test]
    fn empty_direct_input_is_an_error_without_configured_fallback() {
        let temp = TempDir::new().unwrap();
        let valid_path = temp.path().join("valid.lua");
        write(&valid_path, "local valid = true");
        let config = config(vec![script(
            "valid.anm2",
            vec![source(&valid_path, None, [])],
        )]);

        let error =
            analyze_language_scripts(&config, LanguageScriptInput::Direct(&[])).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeLanguageScriptsError::EmptyDirectScripts
        ));
    }
}
