use std::path::PathBuf;

use thiserror::Error;

use crate::config::{ResolvedConfig, ResolvedScriptSource};
use crate::configured_script::ConfiguredLogicalScript;
use crate::source_processing::{
    SourceProcessingError, VariableExpansion, build_source_variables, expand_source_includes,
    expand_variables, load_source_text,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedConfiguredLogicalScript {
    pub name: String,
    pub body: String,
    pub source_paths: Vec<PathBuf>,
    pub warnings: Vec<ConfiguredSourceWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfiguredSourceWarning {
    pub source_path: PathBuf,
    pub undefined_variable: String,
}

#[derive(Debug, Error)]
pub(crate) enum PrepareConfiguredLogicalScriptError {
    #[error("configured論理スクリプトにsourceがありません: {script_name}")]
    MissingSource { script_name: String },
    #[error(
        "論理スクリプト {script_name} のsource {} の処理に失敗しました: {source}",
        source_path.display()
    )]
    Source {
        script_name: String,
        source_path: PathBuf,
        #[source]
        source: SourceProcessingError,
    },
}

pub(crate) fn prepare_configured_logical_script(
    config: &ResolvedConfig,
    logical_script: &ConfiguredLogicalScript<'_>,
) -> Result<PreparedConfiguredLogicalScript, PrepareConfiguredLogicalScriptError> {
    if logical_script.sources.is_empty() {
        return Err(PrepareConfiguredLogicalScriptError::MissingSource {
            script_name: logical_script.name.clone(),
        });
    }

    let mut source_bodies = Vec::with_capacity(logical_script.sources.len());
    let mut source_paths = Vec::with_capacity(logical_script.sources.len());
    let mut warnings = Vec::new();

    for source in logical_script.sources {
        let expansion = prepare_source(config, source).map_err(|error| {
            PrepareConfiguredLogicalScriptError::Source {
                script_name: logical_script.name.clone(),
                source_path: source.path.clone(),
                source: error,
            }
        })?;

        warnings.extend(
            expansion
                .undefined_variables
                .into_iter()
                .map(|undefined_variable| ConfiguredSourceWarning {
                    source_path: source.path.clone(),
                    undefined_variable,
                }),
        );
        source_paths.push(source.path.clone());
        source_bodies.push(expansion.text);
    }

    Ok(PreparedConfiguredLogicalScript {
        name: logical_script.name.clone(),
        body: join_source_bodies(&source_bodies),
        source_paths,
        warnings,
    })
}

fn prepare_source(
    config: &ResolvedConfig,
    source: &ResolvedScriptSource,
) -> Result<VariableExpansion, SourceProcessingError> {
    let content = load_source_text(&source.path)?;
    let content = expand_source_includes(&source.path, &content)?;
    let variables = build_source_variables(config, source)?;
    Ok(expand_variables(&content, &variables))
}

fn join_source_bodies(source_bodies: &[String]) -> String {
    let Some((first, remaining)) = source_bodies.split_first() else {
        return String::new();
    };

    let mut body = first.clone();
    let mut previous = first.as_str();

    for next in remaining {
        let trailing_lfs = previous
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\n')
            .count();
        let leading_lfs = next.bytes().take_while(|byte| *byte == b'\n').count();
        let inserted_lfs = 2usize.saturating_sub(trailing_lfs + leading_lfs);

        for _ in 0..inserted_lfs {
            body.push('\n');
        }
        body.push_str(next);
        previous = next;
    }

    body
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::config::{
        ResolvedBuild, ResolvedConfig, ResolvedInstall, ResolvedProject, ResolvedScript,
        ResolvedScriptSource,
    };
    use crate::configured_script::resolve_configured_script;

    fn config(project_variables: HashMap<String, String>) -> ResolvedConfig {
        ResolvedConfig {
            project: ResolvedProject {
                variables: project_variables,
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
            scripts: Vec::new(),
            config_dir: PathBuf::new(),
        }
    }

    fn source(
        path: impl Into<PathBuf>,
        label: Option<&str>,
        variables: HashMap<String, String>,
    ) -> ResolvedScriptSource {
        ResolvedScriptSource {
            path: path.into(),
            label: label.map(str::to_string),
            variables,
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
    fn prepares_single_source_without_embedding_or_ui_conversion() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("main.lua");
        let include_path = temp.path().join("included.lua");
        write(
            &source_path,
            concat!(
                "---$include \"included.lua\"\n",
                "---$embed\n",
                "local module = require(\"module\")\n",
                "---$tips:${TIP}\n",
                "---$track:${UI_NAME}\n",
                "local value = 0\n",
                "local missing = \"${MISSING}\""
            ),
        );
        write(
            &include_path,
            "local included = \"${INCLUDED}\"\nlocal other = \"${MISSING}\"\n",
        );
        let sources = vec![source(
            &source_path,
            Some("Label must not be inserted"),
            HashMap::from([
                ("TIP".to_string(), "Description".to_string()),
                ("UI_NAME".to_string(), "Value".to_string()),
                ("INCLUDED".to_string(), "Included".to_string()),
            ]),
        )];

        let prepared = prepare_configured_logical_script(
            &config(HashMap::new()),
            &logical_script("Script", &sources),
        )
        .unwrap();

        assert_eq!(prepared.name, "Script");
        assert_eq!(prepared.source_paths, vec![source_path.clone()]);
        assert_eq!(
            prepared.body,
            concat!(
                "local included = \"Included\"\n",
                "local other = \"${MISSING}\"\n\n",
                "---$embed\n",
                "local module = require(\"module\")\n",
                "---$tips:Description\n",
                "---$track:Value\n",
                "local value = 0\n",
                "local missing = \"${MISSING}\""
            )
        );
        assert!(prepared.body.contains("---$embed"));
        assert!(prepared.body.contains("require(\"module\")"));
        assert!(prepared.body.contains("---$track:Value"));
        assert!(!prepared.body.contains("--track@"));
        assert!(!prepared.body.contains("Label must not be inserted"));
        assert_eq!(
            prepared.warnings,
            vec![ConfiguredSourceWarning {
                source_path,
                undefined_variable: "MISSING".to_string(),
            }]
        );
    }

    #[test]
    fn prepares_multiple_sources_in_order_with_source_specific_variables_and_warnings() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("first.lua");
        let second_path = temp.path().join("second.lua");
        write(&first_path, "${VALUE} ${MISSING} ${MISSING}");
        write(&second_path, "${VALUE} ${MISSING}");
        let sources = vec![
            source(
                &first_path,
                None,
                HashMap::from([("VALUE".to_string(), "First".to_string())]),
            ),
            source(
                &second_path,
                None,
                HashMap::from([("VALUE".to_string(), "Second".to_string())]),
            ),
        ];

        let prepared = prepare_configured_logical_script(
            &config(HashMap::new()),
            &logical_script("Combined", &sources),
        )
        .unwrap();

        assert_eq!(
            prepared.body,
            "First ${MISSING} ${MISSING}\n\nSecond ${MISSING}"
        );
        assert_eq!(
            prepared.source_paths,
            vec![first_path.clone(), second_path.clone()]
        );
        assert_eq!(
            prepared.warnings,
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
    }

    #[test]
    fn inserts_only_the_lfs_needed_for_each_source_boundary() {
        for (sources, expected) in [
            (vec!["A", "B"], "A\n\nB"),
            (vec!["A\n", "B"], "A\n\nB"),
            (vec!["A\n\n", "B"], "A\n\nB"),
            (vec!["A\n\n\n", "B"], "A\n\n\nB"),
            (vec!["A", "\nB"], "A\n\nB"),
            (vec!["A\n", "\nB"], "A\n\nB"),
            (vec!["A\n\n", "\nB"], "A\n\n\nB"),
            (vec!["A\n \n", "B"], "A\n \n\nB"),
        ] {
            let sources = sources.into_iter().map(str::to_string).collect::<Vec<_>>();
            assert_eq!(join_source_bodies(&sources), expected);
        }
    }

    #[test]
    fn preserves_empty_sources_as_independent_boundaries() {
        for (sources, expected) in [
            (vec!["", "B"], "\n\nB"),
            (vec!["A", ""], "A\n\n"),
            (vec!["", ""], "\n\n"),
            (vec!["A", "", "B"], "A\n\n\n\nB"),
            (vec!["", "", ""], "\n\n\n\n"),
        ] {
            let sources = sources.into_iter().map(str::to_string).collect::<Vec<_>>();
            assert_eq!(join_source_bodies(&sources), expected);
        }
    }

    #[test]
    fn does_not_change_a_single_source_body() {
        for source in ["", "A", "\nA", "A\n", "A\n\n"] {
            assert_eq!(join_source_bodies(&[source.to_string()]), source);
        }
    }

    #[test]
    fn rejects_a_manually_constructed_logical_script_without_sources() {
        let error = prepare_configured_logical_script(
            &config(HashMap::new()),
            &logical_script("Empty", &[]),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PrepareConfiguredLogicalScriptError::MissingSource { script_name }
                if script_name == "Empty"
        ));
    }

    #[test]
    fn uses_resolved_normal_and_multiple_script_sources_without_labels_or_preamble() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("first.lua");
        let second_path = temp.path().join("second.lua");
        let preamble_path = temp.path().join("preamble.lua");
        let part_path = temp.path().join("part.lua");
        write(&first_path, "First");
        write(&second_path, "Second");
        write(&preamble_path, "Preamble");
        write(&part_path, "Part");

        let normal = ResolvedScript {
            name: "normal.anm2".to_string(),
            sources: vec![
                source(&first_path, Some("Ignored first"), HashMap::new()),
                source(&second_path, Some("Ignored second"), HashMap::new()),
            ],
        };
        let multiple = ResolvedScript {
            name: "@container.anm2".to_string(),
            sources: vec![
                source(&preamble_path, None, HashMap::new()),
                source(&part_path, Some("Part Label"), HashMap::new()),
            ],
        };
        let normal = resolve_configured_script(&normal).unwrap().unwrap();
        let multiple = resolve_configured_script(&multiple).unwrap().unwrap();

        let normal_prepared =
            prepare_configured_logical_script(&config(HashMap::new()), &normal.logical_scripts[0])
                .unwrap();
        let multiple_prepared = prepare_configured_logical_script(
            &config(HashMap::new()),
            &multiple.logical_scripts[0],
        )
        .unwrap();

        assert_eq!(normal_prepared.name, "normal");
        assert_eq!(normal_prepared.body, "First\n\nSecond");
        assert!(!normal_prepared.body.contains("Ignored"));
        assert_eq!(multiple.preamble_sources.len(), 1);
        assert_eq!(multiple_prepared.name, "Part Label@container");
        assert_eq!(multiple_prepared.body, "Part");
        assert_eq!(multiple_prepared.source_paths, vec![part_path]);
        assert!(!multiple_prepared.body.contains("Preamble"));
        assert!(!multiple_prepared.body.contains("Part Label"));
    }

    #[test]
    fn read_error_has_logical_script_and_top_level_source_context() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("missing.lua");
        let sources = vec![source(&source_path, None, HashMap::new())];

        let error = prepare_configured_logical_script(
            &config(HashMap::new()),
            &logical_script("Script", &sources),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PrepareConfiguredLogicalScriptError::Source {
                script_name,
                source_path: context_path,
                source: SourceProcessingError::Read { path, .. },
            } if script_name == "Script" && context_path == source_path && path == source_path
        ));
    }

    #[test]
    fn include_error_has_logical_script_and_top_level_source_context() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("main.lua");
        write(&source_path, "---$include missing-quotes");
        let sources = vec![source(&source_path, None, HashMap::new())];

        let error = prepare_configured_logical_script(
            &config(HashMap::new()),
            &logical_script("Script", &sources),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PrepareConfiguredLogicalScriptError::Source {
                script_name,
                source_path: context_path,
                source: SourceProcessingError::Include { .. },
            } if script_name == "Script" && context_path == source_path
        ));
    }

    #[test]
    fn reserved_variable_error_has_logical_script_and_top_level_source_context() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("main.lua");
        write(&source_path, "Body");
        let sources = vec![source(&source_path, None, HashMap::new())];

        let error = prepare_configured_logical_script(
            &config(HashMap::from([(
                "PACKAGE_ID".to_string(),
                "invalid".to_string(),
            )])),
            &logical_script("Script", &sources),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            PrepareConfiguredLogicalScriptError::Source {
                script_name,
                source_path: context_path,
                source: SourceProcessingError::ReservedVariable {
                    scope: "project.variables",
                    name: "PACKAGE_ID",
                },
            } if script_name == "Script" && context_path == source_path
        ));
    }
}
