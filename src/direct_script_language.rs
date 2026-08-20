use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::direct_script::{DirectScriptError, resolve_direct_script};
use crate::language_ui::LanguageScriptSyntax;
use crate::logical_script_language::{
    LanguageScriptFormat, LogicalScriptLanguage, LogicalScriptLanguageError,
    analyze_logical_script_language_with_format,
};
use crate::text_utils::read_text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedDirectLogicalScript {
    pub origin_path: PathBuf,
    pub name: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnalyzedDirectLogicalScript {
    pub prepared: PreparedDirectLogicalScript,
    pub language: LogicalScriptLanguage,
}

#[derive(Debug, Error)]
pub(crate) enum AnalyzeDirectScriptFileError {
    #[error("{} の読み込みに失敗しました: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{} のdirectスクリプト解決に失敗しました: {source}", path.display())]
    Resolve {
        path: PathBuf,
        #[source]
        source: DirectScriptError,
    },
    #[error(
        "direct論理スクリプト {script_name}（{}）のlanguage解析に失敗しました: {source}",
        path.display()
    )]
    Language {
        path: PathBuf,
        script_name: String,
        #[source]
        source: Box<LogicalScriptLanguageError>,
    },
}

#[derive(Debug, Error)]
pub(crate) enum AnalyzeDirectScriptsError {
    #[error(transparent)]
    File(#[from] AnalyzeDirectScriptFileError),
    #[error(
        "論理スクリプト名 {name} がdirectファイル間で重複しています: {} / {}",
        first_path.display(),
        duplicate_path.display()
    )]
    DuplicateLogicalScriptName {
        name: String,
        first_path: PathBuf,
        duplicate_path: PathBuf,
    },
}

pub(crate) fn analyze_direct_script_file(
    path: &Path,
) -> Result<Vec<AnalyzedDirectLogicalScript>, AnalyzeDirectScriptFileError> {
    let content = read_text(path).map_err(|source| AnalyzeDirectScriptFileError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let direct_script = resolve_direct_script(path, &content).map_err(|source| {
        AnalyzeDirectScriptFileError::Resolve {
            path: path.to_path_buf(),
            source,
        }
    })?;
    let format = if direct_script
        .identity
        .extension
        .eq_ignore_ascii_case("tra2")
    {
        LanguageScriptFormat::Tra2
    } else {
        LanguageScriptFormat::Standard(LanguageScriptSyntax::BuiltScript)
    };
    let mut analyzed = Vec::with_capacity(direct_script.logical_scripts.len());

    for logical_script in direct_script.logical_scripts {
        let prepared = PreparedDirectLogicalScript {
            origin_path: path.to_path_buf(),
            name: logical_script.name,
            body: logical_script.body.to_owned(),
        };
        let language =
            analyze_logical_script_language_with_format(&prepared.name, &prepared.body, format)
                .map_err(|source| AnalyzeDirectScriptFileError::Language {
                    path: prepared.origin_path.clone(),
                    script_name: prepared.name.clone(),
                    source: Box::new(source),
                })?;

        analyzed.push(AnalyzedDirectLogicalScript { prepared, language });
    }

    Ok(analyzed)
}

pub(crate) fn analyze_direct_scripts(
    paths: &[PathBuf],
) -> Result<Vec<AnalyzedDirectLogicalScript>, AnalyzeDirectScriptsError> {
    let mut origins: HashMap<String, PathBuf> = HashMap::new();
    let mut analyzed = Vec::new();

    for path in paths {
        for logical_script in analyze_direct_script_file(path)? {
            let name = &logical_script.prepared.name;
            if let Some(first_path) = origins.get(name) {
                return Err(AnalyzeDirectScriptsError::DuplicateLogicalScriptName {
                    name: name.clone(),
                    first_path: first_path.clone(),
                    duplicate_path: logical_script.prepared.origin_path.clone(),
                });
            }
            origins.insert(name.clone(), logical_script.prepared.origin_path.clone());
            analyzed.push(logical_script);
        }
    }

    Ok(analyzed)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::direct_script::DirectScriptError;
    use crate::language_ui::{LanguageUiExtractError, LanguageUiKind, SourceSpan};
    use crate::logical_script_language::LogicalScriptLanguageError;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn analyzes_normal_built_script_after_read_text_normalization() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("normal.anm2");
        write(
            &path,
            concat!(
                "---$tips:Description\r\n",
                "--check@enabled:Enabled,false\r",
                "local enabled = false\r\n",
            ),
        );

        let result = analyze_direct_script_file(&path).unwrap();

        assert_eq!(result.len(), 1);
        let script = &result[0];
        assert_eq!(script.prepared.origin_path, path);
        assert_eq!(script.prepared.name, "normal");
        assert_eq!(
            script.prepared.body,
            "---$tips:Description\n--check@enabled:Enabled,false\nlocal enabled = false\n"
        );
        assert!(!script.prepared.body.contains('\r'));
        assert_eq!(script.language.info.ui_items.len(), 1);
        assert_eq!(script.language.info.ui_items[0].kind, LanguageUiKind::Check);
        assert_eq!(
            script.language.info.ui_items[0]
                .tips
                .as_ref()
                .unwrap()
                .value,
            "Description"
        );
        assert_eq!(script.language.entries.tips_entries[0].key, "Enabled");
    }

    #[test]
    fn retains_utf8_bom_in_normal_script_body() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("bom.obj2");
        write(
            &path,
            "\u{feff}local value = true\n--check@flag:Flag,false\n",
        );

        let result = analyze_direct_script_file(&path).unwrap();

        assert!(result[0].prepared.body.starts_with('\u{feff}'));
        assert_eq!(result[0].language.info.ui_items.len(), 1);
    }

    #[test]
    fn analyzes_multiple_script_sections_in_label_order_with_body_local_spans() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("@combined.obj2");
        write(
            &path,
            concat!(
                "---$nolang: unknown\n",
                "--check@invalid:\n",
                "@First\n",
                "---$tips:First tip\n",
                "--check@first:First,false\n",
                "@Second\n",
                "--check@second:Second,false\n",
                "@Empty\n",
            ),
        );

        let result = analyze_direct_script_file(&path).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["First@combined", "Second@combined", "Empty@combined"]
        );
        assert_eq!(
            result[0].prepared.body,
            "---$tips:First tip\n--check@first:First,false\n"
        );
        assert_eq!(result[0].language.entries.tips_entries[0].key, "First");
        assert_eq!(
            result[0].language.info.ui_items[0].span,
            SourceSpan {
                start_line: 2,
                end_line: 2,
            }
        );
        assert_eq!(result[1].prepared.body, "--check@second:Second,false\n");
        assert_eq!(
            result[1].language.info.ui_items[0].span,
            SourceSpan {
                start_line: 1,
                end_line: 1,
            }
        );
        assert!(result[2].prepared.body.is_empty());
        assert!(result[2].language.info.ui_items.is_empty());
        assert!(
            result
                .iter()
                .all(|script| !script.prepared.body.contains("@First")
                    && !script.prepared.body.contains("unknown")
                    && !script.prepared.body.contains("invalid"))
        );
    }

    #[test]
    fn analyzes_multiple_tra2_sections_with_param_only_ui() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("@Basic_S.tra2");
        write(
            &path,
            concat!(
                "--param:Preamble,1\n",
                "@First\n",
                "--track@vx:X速度,-10,10,0\n",
                "--param:aaa::周期,0.5\n",
                "--param:周期,0.5\n",
                "--param:周期,1.0\n",
                "@Second\n",
                "--param:デューティ比%,50\n",
            ),
        );

        let result = analyze_direct_script_file(&path).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["First@Basic_S", "Second@Basic_S"]
        );
        assert_eq!(
            result[0]
                .language
                .entries
                .text_entries
                .iter()
                .map(|entry| (entry.key.as_str(), entry.origins.len()))
                .collect::<Vec<_>>(),
            vec![("First@Basic_S", 1), ("aaa::周期", 1), ("周期", 2)]
        );
        assert_eq!(
            result[1]
                .language
                .entries
                .text_entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            vec!["Second@Basic_S", "デューティ比%"]
        );
        assert!(result.iter().all(|script| {
            !script
                .language
                .entries
                .text_entries
                .iter()
                .any(|entry| matches!(entry.key.as_str(), "Preamble" | "X速度"))
        }));
    }

    #[test]
    fn wraps_read_error_with_input_path() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("missing.anm2");

        let error = analyze_direct_script_file(&path).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptFileError::Read {
                path: error_path,
                source: _,
            } if error_path == path
        ));
    }

    #[test]
    fn wraps_direct_script_resolution_error_with_input_path() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("@missing.anm2");
        write(&path, "preamble only\n");

        let error = analyze_direct_script_file(&path).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptFileError::Resolve {
                path: error_path,
                source: DirectScriptError::MissingLabelLine { .. },
            } if error_path == path
        ));
    }

    #[test]
    fn wraps_unsupported_extension_after_successful_read() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("script.lua");
        write(&path, "body\n");

        let error = analyze_direct_script_file(&path).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptFileError::Resolve {
                path: error_path,
                source: DirectScriptError::UnsupportedScriptExtension { .. },
            } if error_path == path
        ));
    }

    #[test]
    fn wraps_language_error_with_origin_path_and_section_name() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("@combined.anm2");
        write(
            &path,
            "@Valid\n--check@valid:Valid,false\n@Broken\n--check@broken:\n",
        );

        let error = analyze_direct_script_file(&path).unwrap_err();

        let AnalyzeDirectScriptFileError::Language {
            path: error_path,
            script_name,
            source,
        } = error
        else {
            panic!("unexpected error variant");
        };
        assert_eq!(error_path, path);
        assert_eq!(script_name, "Broken@combined");
        assert!(
            matches!(
                *source,
                LogicalScriptLanguageError::UiExtract(
                    LanguageUiExtractError::InvalidBuiltSyntax { .. }
                )
            ),
            "unexpected lower error: {source:?}"
        );
    }

    #[test]
    fn preserves_path_and_section_order_across_multiple_files() {
        let temp = TempDir::new().unwrap();
        let normal_path = temp.path().join("normal.anm2");
        let multiple_path = temp.path().join("@combined.obj2");
        write(&normal_path, "--check@normal:Normal,false\n");
        write(
            &multiple_path,
            "@First\n--check@first:First,false\n@Second\n--check@second:Second,false\n",
        );

        let result = analyze_direct_scripts(&[normal_path.clone(), multiple_path.clone()]).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["normal", "First@combined", "Second@combined"]
        );
        assert_eq!(result[0].prepared.origin_path, normal_path);
        assert_eq!(result[1].prepared.origin_path, multiple_path);
        assert_eq!(result[2].prepared.origin_path, multiple_path);
    }

    #[test]
    fn duplicate_name_across_different_paths_reports_both_origins() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("first").join("same.anm2");
        let duplicate_path = temp.path().join("second").join("same.obj2");
        write(&first_path, "body\n");
        write(&duplicate_path, "body\n");

        let error =
            analyze_direct_scripts(&[first_path.clone(), duplicate_path.clone()]).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptsError::DuplicateLogicalScriptName {
                name,
                first_path: first,
                duplicate_path: duplicate,
            } if name == "same" && first == first_path && duplicate == duplicate_path
        ));
    }

    #[test]
    fn repeated_path_is_processed_and_then_rejected_as_duplicate_name() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("same.scn2");
        write(&path, "body\n");

        let error = analyze_direct_scripts(&[path.clone(), path.clone()]).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptsError::DuplicateLogicalScriptName {
                name,
                first_path,
                duplicate_path,
            } if name == "same" && first_path == path && duplicate_path == path
        ));
    }

    #[test]
    fn duplicate_name_between_multiple_section_and_normal_file_is_an_error() {
        let temp = TempDir::new().unwrap();
        let multiple_path = temp.path().join("@container.anm2");
        let normal_path = temp.path().join("Part@container.obj2");
        write(&multiple_path, "@Part\nbody\n");
        write(&normal_path, "body\n");

        let error =
            analyze_direct_scripts(&[multiple_path.clone(), normal_path.clone()]).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptsError::DuplicateLogicalScriptName {
                name,
                first_path,
                duplicate_path,
            } if name == "Part@container"
                && first_path == multiple_path
                && duplicate_path == normal_path
        ));
    }

    #[test]
    fn cross_file_duplicate_detection_is_exact_without_unicode_normalization() {
        let temp = TempDir::new().unwrap();
        let paths = [
            temp.path().join("Name.anm2"),
            temp.path().join("name.obj2"),
            temp.path().join("é.scn2"),
            temp.path().join("e\u{301}.cam2"),
        ];
        for path in &paths {
            write(path, "body\n");
        }

        let result = analyze_direct_scripts(&paths).unwrap();

        assert_eq!(
            result
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Name", "name", "é", "e\u{301}"]
        );
    }

    #[test]
    fn wraps_file_error_from_multiple_path_entry_point() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("missing.cam2");

        let error = analyze_direct_scripts(std::slice::from_ref(&path)).unwrap_err();

        assert!(matches!(
            error,
            AnalyzeDirectScriptsError::File(AnalyzeDirectScriptFileError::Read {
                path: error_path,
                ..
            }) if error_path == path
        ));
    }
}
