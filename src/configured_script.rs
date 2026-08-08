use std::collections::HashSet;
use std::ffi::OsStr;

use thiserror::Error;

use crate::config::{ResolvedScript, ResolvedScriptSource};
use crate::script_identity::{
    ScriptFileIdentity, ScriptFileKind, ScriptIdentityError, resolve_logical_script_name,
    resolve_script_file_identity,
};

#[derive(Debug)]
pub struct ConfiguredScriptFile<'a> {
    pub identity: ScriptFileIdentity,
    pub preamble_sources: &'a [ResolvedScriptSource],
    pub logical_scripts: Vec<ConfiguredLogicalScript<'a>>,
}

#[derive(Debug)]
pub struct ConfiguredLogicalScript<'a> {
    pub name: String,
    pub sources: &'a [ResolvedScriptSource],
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfiguredScriptError {
    #[error(transparent)]
    ScriptIdentity(#[from] ScriptIdentityError),
    #[error("通常形式のスクリプトにsourceがありません: {script_name}")]
    MissingSource { script_name: String },
    #[error(
        "複数スクリプト形式のlabelが空または空白のみです: {script_name} sources[{source_index}]"
    )]
    EmptyLabel {
        script_name: String,
        source_index: usize,
    },
    #[error(
        "複数スクリプト形式でlabel付きsourceの後にlabelなしsourceがあります: {script_name} sources[{source_index}]"
    )]
    UnlabeledSourceAfterLabeledSource {
        script_name: String,
        source_index: usize,
    },
    #[error("複数スクリプト形式にlabel付きsourceがありません: {script_name}")]
    MissingLabeledSource { script_name: String },
    #[error("論理スクリプト名が重複しています: {name}")]
    DuplicateLogicalScriptName { name: String },
}

pub fn resolve_configured_script(
    script: &ResolvedScript,
) -> Result<Option<ConfiguredScriptFile<'_>>, ConfiguredScriptError> {
    let Some(identity) = resolve_script_file_identity(script.name.as_ref())? else {
        return Ok(None);
    };

    let configured_script = match &identity.kind {
        ScriptFileKind::Single => resolve_single_script(script, identity).map(Some),
        ScriptFileKind::Multiple { .. } => resolve_multiple_script(script, identity).map(Some),
    }?;

    if let Some(configured_script) = &configured_script {
        ensure_unique_logical_script_names(
            configured_script
                .logical_scripts
                .iter()
                .map(|logical_script| &logical_script.name),
        )?;
    }

    Ok(configured_script)
}

pub fn resolve_configured_scripts(
    scripts: &[ResolvedScript],
) -> Result<Vec<ConfiguredScriptFile<'_>>, ConfiguredScriptError> {
    let mut result = Vec::new();
    let mut logical_script_names = HashSet::new();

    for script in scripts {
        let Some(configured_script) = resolve_configured_script(script)? else {
            continue;
        };

        for logical_script in &configured_script.logical_scripts {
            if !logical_script_names.insert(logical_script.name.clone()) {
                return Err(ConfiguredScriptError::DuplicateLogicalScriptName {
                    name: logical_script.name.clone(),
                });
            }
        }

        result.push(configured_script);
    }

    Ok(result)
}

fn ensure_unique_logical_script_names<'a>(
    names: impl IntoIterator<Item = &'a String>,
) -> Result<(), ConfiguredScriptError> {
    let mut unique_names = HashSet::new();

    for name in names {
        if !unique_names.insert(name) {
            return Err(ConfiguredScriptError::DuplicateLogicalScriptName { name: name.clone() });
        }
    }

    Ok(())
}

fn resolve_single_script(
    script: &ResolvedScript,
    identity: ScriptFileIdentity,
) -> Result<ConfiguredScriptFile<'_>, ConfiguredScriptError> {
    if script.sources.is_empty() {
        return Err(ConfiguredScriptError::MissingSource {
            script_name: script.name.clone(),
        });
    }

    let name = resolve_logical_script_name(&identity, None)?;

    Ok(ConfiguredScriptFile {
        identity,
        preamble_sources: &script.sources[..0],
        logical_scripts: vec![ConfiguredLogicalScript {
            name,
            sources: &script.sources,
        }],
    })
}

fn resolve_multiple_script(
    script: &ResolvedScript,
    identity: ScriptFileIdentity,
) -> Result<ConfiguredScriptFile<'_>, ConfiguredScriptError> {
    let preamble_len = script
        .sources
        .iter()
        .take_while(|source| source.label.is_none())
        .count();
    let mut logical_scripts = Vec::new();

    for (source_index, source) in script.sources.iter().enumerate().skip(preamble_len) {
        let Some(label) = source.label.as_deref() else {
            return Err(ConfiguredScriptError::UnlabeledSourceAfterLabeledSource {
                script_name: script.name.clone(),
                source_index,
            });
        };

        if label.trim().is_empty() {
            return Err(ConfiguredScriptError::EmptyLabel {
                script_name: script.name.clone(),
                source_index,
            });
        }

        let name = resolve_logical_script_name(&identity, Some(OsStr::new(label)))?;
        logical_scripts.push(ConfiguredLogicalScript {
            name,
            sources: &script.sources[source_index..source_index + 1],
        });
    }

    if logical_scripts.is_empty() {
        return Err(ConfiguredScriptError::MissingLabeledSource {
            script_name: script.name.clone(),
        });
    }

    Ok(ConfiguredScriptFile {
        identity,
        preamble_sources: &script.sources[..preamble_len],
        logical_scripts,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::*;

    fn source(path: &str, label: Option<&str>) -> ResolvedScriptSource {
        ResolvedScriptSource {
            path: PathBuf::from(path),
            label: label.map(str::to_string),
            variables: HashMap::new(),
        }
    }

    fn script(name: &str, sources: Vec<ResolvedScriptSource>) -> ResolvedScript {
        ResolvedScript {
            name: name.to_string(),
            sources,
        }
    }

    #[test]
    fn resolves_single_script_with_all_sources_in_order() {
        let input = script(
            "effect.anm2",
            vec![
                source("first.lua", Some("ignored")),
                source("second.lua", Some("   ")),
                source("third.lua", None),
            ],
        );

        let output = resolve_configured_script(&input).unwrap().unwrap();

        assert!(output.preamble_sources.is_empty());
        assert_eq!(output.logical_scripts.len(), 1);
        assert_eq!(output.logical_scripts[0].name, "effect");
        assert_eq!(output.logical_scripts[0].sources.len(), 3);
        assert_eq!(
            output.logical_scripts[0].sources[0].path,
            PathBuf::from("first.lua")
        );
        assert_eq!(
            output.logical_scripts[0].sources[1].path,
            PathBuf::from("second.lua")
        );
        assert_eq!(
            output.logical_scripts[0].sources[2].path,
            PathBuf::from("third.lua")
        );
    }

    #[test]
    fn single_script_without_sources_is_an_error() {
        let input = script("effect.anm2", vec![]);

        assert_eq!(
            resolve_configured_script(&input).unwrap_err(),
            ConfiguredScriptError::MissingSource {
                script_name: "effect.anm2".to_string()
            }
        );
    }

    #[test]
    fn resolves_multiple_script_with_preamble_and_labeled_sources() {
        let input = script(
            "@combined.anm2",
            vec![
                source("preamble-1.lua", None),
                source("preamble-2.lua", None),
                source("part-1.lua", Some("Part 1")),
                source("part-2.lua", Some("Part 2")),
            ],
        );

        let output = resolve_configured_script(&input).unwrap().unwrap();

        assert_eq!(output.preamble_sources.len(), 2);
        assert_eq!(
            output.preamble_sources[0].path,
            PathBuf::from("preamble-1.lua")
        );
        assert_eq!(
            output.preamble_sources[1].path,
            PathBuf::from("preamble-2.lua")
        );
        assert_eq!(output.logical_scripts.len(), 2);
        assert_eq!(output.logical_scripts[0].name, "Part 1@combined");
        assert_eq!(output.logical_scripts[0].sources.len(), 1);
        assert_eq!(
            output.logical_scripts[0].sources[0].path,
            PathBuf::from("part-1.lua")
        );
        assert_eq!(output.logical_scripts[1].name, "Part 2@combined");
        assert_eq!(output.logical_scripts[1].sources.len(), 1);
        assert_eq!(
            output.logical_scripts[1].sources[0].path,
            PathBuf::from("part-2.lua")
        );
    }

    #[test]
    fn resolves_multiple_script_without_preamble() {
        let input = script("@combined.anm2", vec![source("part.lua", Some("Part"))]);

        let output = resolve_configured_script(&input).unwrap().unwrap();

        assert!(output.preamble_sources.is_empty());
        assert_eq!(output.logical_scripts[0].name, "Part@combined");
    }

    #[test]
    fn unlabeled_source_after_labeled_source_is_an_error() {
        let input = script(
            "@combined.anm2",
            vec![
                source("preamble.lua", None),
                source("part.lua", Some("Part")),
                source("invalid.lua", None),
            ],
        );

        assert_eq!(
            resolve_configured_script(&input).unwrap_err(),
            ConfiguredScriptError::UnlabeledSourceAfterLabeledSource {
                script_name: "@combined.anm2".to_string(),
                source_index: 2,
            }
        );
    }

    #[test]
    fn multiple_script_without_labeled_source_is_an_error() {
        for sources in [vec![], vec![source("preamble.lua", None)]] {
            let input = script("@combined.anm2", sources);

            assert_eq!(
                resolve_configured_script(&input).unwrap_err(),
                ConfiguredScriptError::MissingLabeledSource {
                    script_name: "@combined.anm2".to_string()
                }
            );
        }
    }

    #[test]
    fn empty_or_whitespace_only_label_is_an_error() {
        for label in ["", " ", "\t\r\n"] {
            let input = script("@combined.anm2", vec![source("part.lua", Some(label))]);

            assert_eq!(
                resolve_configured_script(&input).unwrap_err(),
                ConfiguredScriptError::EmptyLabel {
                    script_name: "@combined.anm2".to_string(),
                    source_index: 0,
                }
            );
        }
    }

    #[test]
    fn non_empty_label_is_not_trimmed() {
        let input = script("@combined.anm2", vec![source("part.lua", Some(" Part "))]);

        let output = resolve_configured_script(&input).unwrap().unwrap();

        assert_eq!(output.logical_scripts[0].name, " Part @combined");
    }

    #[test]
    fn unsupported_script_is_excluded_without_validating_sources() {
        for input in [
            script("legacy.anm", vec![]),
            script(
                "@legacy.lua",
                vec![
                    source("invalid.lua", Some("   ")),
                    source("later.lua", None),
                ],
            ),
            script("@.lua", vec![]),
        ] {
            assert!(resolve_configured_script(&input).unwrap().is_none());
        }
    }

    #[test]
    fn resolves_all_scripts_in_configuration_order_and_skips_unsupported_scripts() {
        let inputs = vec![
            script("first.anm2", vec![source("first.lua", None)]),
            script("ignored.lua", vec![]),
            script(
                "@combined.obj2",
                vec![
                    source("part-a.lua", Some("Part A")),
                    source("part-b.lua", Some("Part B")),
                ],
            ),
            script("last.scn2", vec![source("last.lua", None)]),
        ];

        let output = resolve_configured_scripts(&inputs).unwrap();

        assert_eq!(output.len(), 3);
        assert_eq!(output[0].identity.file_name, "first.anm2");
        assert_eq!(output[0].logical_scripts[0].name, "first");
        assert_eq!(output[1].identity.file_name, "@combined.obj2");
        assert_eq!(output[1].logical_scripts[0].name, "Part A@combined");
        assert_eq!(output[1].logical_scripts[1].name, "Part B@combined");
        assert_eq!(output[2].identity.file_name, "last.scn2");
        assert_eq!(output[2].logical_scripts[0].name, "last");
    }

    #[test]
    fn duplicate_logical_names_within_multiple_script_are_an_error() {
        let input = script(
            "@combined.anm2",
            vec![
                source("first.lua", Some("Part")),
                source("second.lua", Some("Part")),
            ],
        );

        assert_eq!(
            resolve_configured_script(&input).unwrap_err(),
            ConfiguredScriptError::DuplicateLogicalScriptName {
                name: "Part@combined".to_string()
            }
        );
    }

    #[test]
    fn duplicate_logical_names_across_script_files_are_an_error() {
        let inputs = vec![
            script("same.anm2", vec![source("first.lua", None)]),
            script("same.obj2", vec![source("second.lua", None)]),
        ];

        assert_eq!(
            resolve_configured_scripts(&inputs).unwrap_err(),
            ConfiguredScriptError::DuplicateLogicalScriptName {
                name: "same".to_string()
            }
        );
    }

    #[test]
    fn duplicate_logical_names_across_single_and_multiple_scripts_are_an_error() {
        let inputs = vec![
            script("@combined.anm2", vec![source("part.lua", Some("Part"))]),
            script("Part@combined.obj2", vec![source("single.lua", None)]),
        ];

        assert_eq!(
            resolve_configured_scripts(&inputs).unwrap_err(),
            ConfiguredScriptError::DuplicateLogicalScriptName {
                name: "Part@combined".to_string()
            }
        );
    }

    #[test]
    fn duplicate_detection_is_case_sensitive_and_does_not_normalize_unicode() {
        let inputs = vec![
            script("Name.anm2", vec![source("upper.lua", None)]),
            script("name.obj2", vec![source("lower.lua", None)]),
            script("é.scn2", vec![source("composed.lua", None)]),
            script("e\u{301}.cam2", vec![source("decomposed.lua", None)]),
        ];

        let output = resolve_configured_scripts(&inputs).unwrap();

        assert_eq!(output.len(), 4);
        assert_eq!(output[0].logical_scripts[0].name, "Name");
        assert_eq!(output[1].logical_scripts[0].name, "name");
        assert_eq!(output[2].logical_scripts[0].name, "é");
        assert_eq!(output[3].logical_scripts[0].name, "e\u{301}");
    }

    #[test]
    fn script_identity_errors_are_wrapped() {
        let input = script("@.anm2", vec![source("part.lua", Some("Part"))]);

        assert!(matches!(
            resolve_configured_script(&input),
            Err(ConfiguredScriptError::ScriptIdentity(
                ScriptIdentityError::EmptyContainer { .. }
            ))
        ));
    }
}
