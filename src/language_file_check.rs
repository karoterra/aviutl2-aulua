use std::fs;
use std::io;
use std::path::PathBuf;

use thiserror::Error;

use crate::aul2_check::{LanguageCheckFinding, check_aul2_document};
use crate::aul2_parser::{ParseAul2DocumentError, parse_aul2_document};
use crate::language_file_plan::LanguageFilePlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LanguageFileCheckResult {
    MissingFile {
        path: PathBuf,
    },
    Checked {
        path: PathBuf,
        findings: Vec<LanguageCheckFinding>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageCheckResult {
    pub files: Vec<LanguageFileCheckResult>,
}

impl LanguageCheckResult {
    pub(crate) fn has_findings(&self) -> bool {
        self.files.iter().any(|file| match file {
            LanguageFileCheckResult::MissingFile { .. } => true,
            LanguageFileCheckResult::Checked { findings, .. } => !findings.is_empty(),
        })
    }
}

#[derive(Debug, Error)]
pub(crate) enum CheckLanguageFilesError {
    #[error("language fileの読み込みに失敗しました: {}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("language fileのparseに失敗しました: {}", path.display())]
    Parse {
        path: PathBuf,
        #[source]
        source: ParseAul2DocumentError,
    },
}

pub(crate) fn check_language_files(
    plans: &[LanguageFilePlan],
) -> Result<LanguageCheckResult, CheckLanguageFilesError> {
    let files = plans
        .iter()
        .map(check_language_file)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LanguageCheckResult { files })
}

fn check_language_file(
    plan: &LanguageFilePlan,
) -> Result<LanguageFileCheckResult, CheckLanguageFilesError> {
    let path = &plan.request.path;

    match fs::read_to_string(path) {
        Ok(input) => {
            let document =
                parse_aul2_document(&input).map_err(|source| CheckLanguageFilesError::Parse {
                    path: path.clone(),
                    source,
                })?;
            let findings = check_aul2_document(&document, plan);

            Ok(LanguageFileCheckResult::Checked {
                path: path.clone(),
                findings,
            })
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            Ok(LanguageFileCheckResult::MissingFile { path: path.clone() })
        }
        Err(source) => Err(CheckLanguageFilesError::Read {
            path: path.clone(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::language_file_plan::{LanguageSectionPlan, LanguageTextEntryPlan};
    use crate::language_file_request::{LanguageFileRequest, LanguageFileRequestOrigin};
    use crate::language_script_analysis::LogicalScriptOrigin;
    use crate::language_script_entries::LanguageTextOrigin;

    fn plan(path: impl Into<PathBuf>, sections: Vec<LanguageSectionPlan>) -> LanguageFilePlan {
        LanguageFilePlan {
            request: LanguageFileRequest {
                path: path.into(),
                text: true,
                tooltip: true,
                is_default: false,
                origin: LanguageFileRequestOrigin::Override,
            },
            sections,
        }
    }

    fn text_section(name: &str, keys: &[&str]) -> LanguageSectionPlan {
        LanguageSectionPlan::Text {
            name: name.to_string(),
            logical_script_name: name.to_string(),
            origin: LogicalScriptOrigin::Direct {
                path: PathBuf::from("script.anm2"),
            },
            entries: keys
                .iter()
                .map(|key| LanguageTextEntryPlan {
                    key: (*key).to_string(),
                    value: String::new(),
                    origins: vec![LanguageTextOrigin::ScriptName],
                })
                .collect(),
        }
    }

    fn write(path: &Path, content: &str) {
        fs::write(path, content).unwrap();
    }

    #[test]
    fn clean_existing_file_is_retained_as_checked() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("English.aul2");
        write(&path, "[section]\nkey=translated\n");
        let plans = [plan(&path, vec![text_section("section", &["key"])])];

        let result = check_language_files(&plans).unwrap();

        assert_eq!(
            result.files,
            vec![LanguageFileCheckResult::Checked {
                path,
                findings: Vec::new(),
            }]
        );
        assert!(!result.has_findings());
    }

    #[test]
    fn document_findings_keep_path_source_lines_and_order() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("English.aul2");
        write(&path, "; preamble\n[section]\nempty=\nold=value\n");
        let plans = [plan(
            &path,
            vec![text_section("section", &["missing", "empty"])],
        )];

        let result = check_language_files(&plans).unwrap();

        assert_eq!(
            result.files,
            vec![LanguageFileCheckResult::Checked {
                path,
                findings: vec![
                    LanguageCheckFinding::MissingKey {
                        section_name: "section".to_string(),
                        key: "missing".to_string(),
                    },
                    LanguageCheckFinding::EmptyValue {
                        section_name: "section".to_string(),
                        key: "empty".to_string(),
                        source_line: Some(3),
                    },
                    LanguageCheckFinding::UnusedKey {
                        section_name: "section".to_string(),
                        key: "old".to_string(),
                        source_line: Some(4),
                    },
                ],
            }]
        );
        assert!(result.has_findings());
    }

    #[test]
    fn missing_nonempty_file_is_one_file_level_finding() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("missing.aul2");
        let plans = [plan(&path, vec![text_section("section", &["a", "b"])])];

        let result = check_language_files(&plans).unwrap();

        assert_eq!(
            result.files,
            vec![LanguageFileCheckResult::MissingFile { path }]
        );
        assert!(result.has_findings());
    }

    #[test]
    fn missing_empty_plan_is_still_missing_file() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("missing.aul2");

        let result = check_language_files(&[plan(&path, Vec::new())]).unwrap();

        assert_eq!(
            result.files,
            vec![LanguageFileCheckResult::MissingFile { path }]
        );
    }

    #[test]
    fn invalid_utf8_is_read_error_with_target_path() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("invalid.aul2");
        fs::write(&path, [0xff]).unwrap();

        let error = check_language_files(&[plan(&path, Vec::new())]).unwrap_err();

        match error {
            CheckLanguageFilesError::Read {
                path: error_path,
                source,
            } => {
                assert_eq!(error_path, path);
                assert_eq!(source.kind(), io::ErrorKind::InvalidData);
            }
            other => panic!("Read errorではありません: {other:?}"),
        }
    }

    #[test]
    fn malformed_document_is_parse_error_with_target_path() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("malformed.aul2");
        write(&path, "[broken\n");

        let error = check_language_files(&[plan(&path, Vec::new())]).unwrap_err();

        match error {
            CheckLanguageFilesError::Parse {
                path: error_path, ..
            } => assert_eq!(error_path, path),
            other => panic!("Parse errorではありません: {other:?}"),
        }
    }

    #[test]
    fn multiple_files_preserve_plan_and_finding_order() {
        let directory = TempDir::new().unwrap();
        let clean_path = directory.path().join("A.aul2");
        let findings_path = directory.path().join("B.aul2");
        let missing_path = directory.path().join("C.aul2");
        write(&clean_path, "[clean]\nkey=value\n");
        write(&findings_path, "[managed]\nempty=\nold=value\n");
        let plans = [
            plan(&clean_path, vec![text_section("clean", &["key"])]),
            plan(
                &findings_path,
                vec![text_section("managed", &["missing", "empty"])],
            ),
            plan(&missing_path, vec![text_section("missing", &["key"])]),
        ];

        let result = check_language_files(&plans).unwrap();

        assert_eq!(
            result.files,
            vec![
                LanguageFileCheckResult::Checked {
                    path: clean_path,
                    findings: Vec::new(),
                },
                LanguageFileCheckResult::Checked {
                    path: findings_path,
                    findings: vec![
                        LanguageCheckFinding::MissingKey {
                            section_name: "managed".to_string(),
                            key: "missing".to_string(),
                        },
                        LanguageCheckFinding::EmptyValue {
                            section_name: "managed".to_string(),
                            key: "empty".to_string(),
                            source_line: Some(2),
                        },
                        LanguageCheckFinding::UnusedKey {
                            section_name: "managed".to_string(),
                            key: "old".to_string(),
                            source_line: Some(3),
                        },
                    ],
                },
                LanguageFileCheckResult::MissingFile { path: missing_path },
            ]
        );
    }

    #[test]
    fn later_parse_error_returns_no_partial_result_and_changes_no_files() {
        let directory = TempDir::new().unwrap();
        let first_path = directory.path().join("first.aul2");
        let second_path = directory.path().join("second.aul2");
        let first_content = "[section]\nold=value\n";
        let second_content = "[broken\n";
        write(&first_path, first_content);
        write(&second_path, second_content);
        let plans = [
            plan(&first_path, vec![text_section("section", &["missing"])]),
            plan(&second_path, Vec::new()),
        ];

        assert!(matches!(
            check_language_files(&plans),
            Err(CheckLanguageFilesError::Parse { path, .. }) if path == second_path
        ));
        assert_eq!(fs::read_to_string(first_path).unwrap(), first_content);
        assert_eq!(fs::read_to_string(second_path).unwrap(), second_content);
    }

    #[test]
    fn empty_plan_slice_is_clean() {
        let result = check_language_files(&[]).unwrap();

        assert!(result.files.is_empty());
        assert!(!result.has_findings());
    }

    #[test]
    fn existing_file_with_empty_plan_is_checked_and_clean() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("existing.aul2");
        write(&path, "[Unknown]\nempty=\n");

        let result = check_language_files(&[plan(&path, Vec::new())]).unwrap();

        assert_eq!(
            result.files,
            vec![LanguageFileCheckResult::Checked {
                path,
                findings: Vec::new(),
            }]
        );
        assert!(!result.has_findings());
    }
}
