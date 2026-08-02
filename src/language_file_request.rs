use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::config::ResolvedConfig;

#[derive(Debug, Clone, Copy)]
pub(crate) enum LanguageFileSelection<'a> {
    Configured,
    Override(&'a Path),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LanguageFileRequestOrigin {
    Configured { index: usize },
    Override,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageFileRequest {
    pub path: PathBuf,
    pub text: bool,
    pub tooltip: bool,
    pub is_default: bool,
    pub origin: LanguageFileRequestOrigin,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ResolveLanguageFileRequestsError {
    #[error("language file設定がありません")]
    MissingLanguageConfig,
    #[error("language fileが1件も設定されていません")]
    EmptyLanguageFiles,
    #[error(
        "language file pathの拡張子が.aul2ではありません: {}",
        path.display()
    )]
    InvalidExtension {
        path: PathBuf,
        origin: LanguageFileRequestOrigin,
    },
    #[error("language file pathが重複しています: {}", path.display())]
    DuplicatePath {
        path: PathBuf,
        first_origin: LanguageFileRequestOrigin,
        duplicate_origin: LanguageFileRequestOrigin,
    },
    #[error(
        "language file pathのfile nameがUTF-8ではありません: {}",
        path.display()
    )]
    NonUtf8FileName {
        path: PathBuf,
        origin: LanguageFileRequestOrigin,
    },
}

pub(crate) fn resolve_language_file_requests(
    config: &ResolvedConfig,
    selection: LanguageFileSelection<'_>,
) -> Result<Vec<LanguageFileRequest>, ResolveLanguageFileRequestsError> {
    match selection {
        LanguageFileSelection::Configured => resolve_configured_requests(config),
        LanguageFileSelection::Override(path) => {
            let path = resolve_override_path(&config.config_dir, path);
            let origin = LanguageFileRequestOrigin::Override;
            let is_default = validate_language_file_path(&path, origin)?;

            Ok(vec![LanguageFileRequest {
                path,
                text: true,
                tooltip: true,
                is_default,
                origin,
            }])
        }
    }
}

fn resolve_configured_requests(
    config: &ResolvedConfig,
) -> Result<Vec<LanguageFileRequest>, ResolveLanguageFileRequestsError> {
    let language = config
        .language
        .as_ref()
        .ok_or(ResolveLanguageFileRequestsError::MissingLanguageConfig)?;
    if language.files.is_empty() {
        return Err(ResolveLanguageFileRequestsError::EmptyLanguageFiles);
    }

    let requests = language
        .files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            let origin = LanguageFileRequestOrigin::Configured { index };
            let is_default = validate_language_file_path(&file.path, origin)?;

            Ok(LanguageFileRequest {
                path: file.path.clone(),
                text: file.text,
                tooltip: file.tooltip,
                is_default,
                origin,
            })
        })
        .collect::<Result<Vec<_>, ResolveLanguageFileRequestsError>>()?;

    validate_unique_paths(&requests)?;
    Ok(requests)
}

fn resolve_override_path(config_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.join(path)
    }
}

fn validate_language_file_path(
    path: &Path,
    origin: LanguageFileRequestOrigin,
) -> Result<bool, ResolveLanguageFileRequestsError> {
    let Some(file_name) = path.file_name() else {
        return Err(ResolveLanguageFileRequestsError::InvalidExtension {
            path: path.to_path_buf(),
            origin,
        });
    };
    let Some(file_name) = file_name.to_str() else {
        return Err(ResolveLanguageFileRequestsError::NonUtf8FileName {
            path: path.to_path_buf(),
            origin,
        });
    };

    if path.extension() != Some(OsStr::new("aul2")) {
        return Err(ResolveLanguageFileRequestsError::InvalidExtension {
            path: path.to_path_buf(),
            origin,
        });
    }

    Ok(is_default_file_name(file_name))
}

fn is_default_file_name(file_name: &str) -> bool {
    file_name == "Default.aul2"
        || (file_name.starts_with("Default.") && file_name.ends_with(".aul2"))
}

fn validate_unique_paths(
    requests: &[LanguageFileRequest],
) -> Result<(), ResolveLanguageFileRequestsError> {
    let mut origins = HashMap::new();

    for request in requests {
        if let Some(&first_origin) = origins.get(&request.path) {
            return Err(ResolveLanguageFileRequestsError::DuplicatePath {
                path: request.path.clone(),
                first_origin,
                duplicate_origin: request.origin,
            });
        }
        origins.insert(request.path.clone(), request.origin);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::config::{
        ResolvedBuild, ResolvedInstall, ResolvedLanguage, ResolvedLanguageFile, ResolvedProject,
    };

    fn config(
        config_dir: impl Into<PathBuf>,
        language: Option<ResolvedLanguage>,
    ) -> ResolvedConfig {
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
            language,
            scripts: Vec::new(),
            config_dir: config_dir.into(),
        }
    }

    fn language_file(path: impl Into<PathBuf>, text: bool, tooltip: bool) -> ResolvedLanguageFile {
        ResolvedLanguageFile {
            path: path.into(),
            text,
            tooltip,
        }
    }

    fn language(files: Vec<ResolvedLanguageFile>) -> Option<ResolvedLanguage> {
        Some(ResolvedLanguage { files })
    }

    fn configured_origin(index: usize) -> LanguageFileRequestOrigin {
        LanguageFileRequestOrigin::Configured { index }
    }

    fn absolute_path(file_name: &str) -> PathBuf {
        #[cfg(windows)]
        {
            PathBuf::from(format!(r"C:\Language\{file_name}"))
        }
        #[cfg(not(windows))]
        {
            PathBuf::from(format!("/Language/{file_name}"))
        }
    }

    #[test]
    fn configured_requests_preserve_resolved_paths_flags_order_and_origins() {
        let first_path = PathBuf::from("already-resolved/Default.ja.aul2");
        let second_path = PathBuf::from("already-resolved/English.aul2");
        let config = config(
            "config-directory",
            language(vec![
                language_file(&first_path, false, true),
                language_file(&second_path, true, false),
            ]),
        );

        let requests =
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap();

        assert_eq!(
            requests,
            vec![
                LanguageFileRequest {
                    path: first_path,
                    text: false,
                    tooltip: true,
                    is_default: true,
                    origin: configured_origin(0),
                },
                LanguageFileRequest {
                    path: second_path,
                    text: true,
                    tooltip: false,
                    is_default: false,
                    origin: configured_origin(1),
                },
            ]
        );
    }

    #[test]
    fn override_resolves_relative_path_and_ignores_configured_language_files() {
        let config_dir = PathBuf::from("project/config");
        let config = config(
            &config_dir,
            language(vec![
                language_file("invalid.txt", true, true),
                language_file("invalid.txt", true, true),
            ]),
        );

        let requests = resolve_language_file_requests(
            &config,
            LanguageFileSelection::Override(Path::new("Language/Default.override.aul2")),
        )
        .unwrap();

        assert_eq!(
            requests,
            vec![LanguageFileRequest {
                path: config_dir.join("Language/Default.override.aul2"),
                text: true,
                tooltip: true,
                is_default: true,
                origin: LanguageFileRequestOrigin::Override,
            }]
        );
    }

    #[test]
    fn override_preserves_absolute_path_without_language_config() {
        let path = absolute_path("English.aul2");
        let config = config("ignored-config-directory", None);

        let requests =
            resolve_language_file_requests(&config, LanguageFileSelection::Override(&path))
                .unwrap();

        assert_eq!(requests[0].path, path);
        assert_eq!(requests[0].origin, LanguageFileRequestOrigin::Override);
        assert!(requests[0].text);
        assert!(requests[0].tooltip);
    }

    #[test]
    fn configured_requires_language_config_and_at_least_one_file() {
        let missing = config("config", None);
        let empty = config("config", language(Vec::new()));

        assert_eq!(
            resolve_language_file_requests(&missing, LanguageFileSelection::Configured)
                .unwrap_err(),
            ResolveLanguageFileRequestsError::MissingLanguageConfig
        );
        assert_eq!(
            resolve_language_file_requests(&empty, LanguageFileSelection::Configured).unwrap_err(),
            ResolveLanguageFileRequestsError::EmptyLanguageFiles
        );
    }

    #[test]
    fn configured_and_override_apply_the_same_lowercase_aul2_extension_rule() {
        let configured_path = PathBuf::from("Language/English.AUL2");
        let configured = config(
            "config",
            language(vec![language_file(&configured_path, true, true)]),
        );
        assert_eq!(
            resolve_language_file_requests(&configured, LanguageFileSelection::Configured)
                .unwrap_err(),
            ResolveLanguageFileRequestsError::InvalidExtension {
                path: configured_path,
                origin: configured_origin(0),
            }
        );

        let override_config = config("config", None);
        for invalid in [
            "Language/English",
            "Language/English.aul2.bak",
            "directory.aul2/English",
        ] {
            let expected_path = PathBuf::from("config").join(invalid);
            assert_eq!(
                resolve_language_file_requests(
                    &override_config,
                    LanguageFileSelection::Override(Path::new(invalid)),
                )
                .unwrap_err(),
                ResolveLanguageFileRequestsError::InvalidExtension {
                    path: expected_path,
                    origin: LanguageFileRequestOrigin::Override,
                }
            );
        }
    }

    #[test]
    fn default_detection_uses_only_the_exact_case_sensitive_file_name() {
        let config = config(
            "ignored",
            language(vec![
                language_file("one/Default.aul2", true, true),
                language_file("two/Default.ja.aul2", true, true),
                language_file("three/Default..aul2", true, true),
                language_file("Default.directory/default.aul2", true, true),
            ]),
        );

        let requests =
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap();

        assert_eq!(
            requests
                .iter()
                .map(|request| request.is_default)
                .collect::<Vec<_>>(),
            vec![true, true, true, false]
        );
    }

    #[test]
    fn exact_duplicate_path_reports_first_and_duplicate_origins() {
        let path = PathBuf::from("Language/English.aul2");
        let config = config(
            "ignored",
            language(vec![
                language_file(&path, true, true),
                language_file("Language/Other.aul2", true, true),
                language_file(&path, false, true),
            ]),
        );

        assert_eq!(
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap_err(),
            ResolveLanguageFileRequestsError::DuplicatePath {
                path,
                first_origin: configured_origin(0),
                duplicate_origin: configured_origin(2),
            }
        );
    }

    #[test]
    fn duplicate_comparison_does_not_fold_case_or_resolve_parent_components() {
        let config = config(
            "ignored",
            language(vec![
                language_file("Language/English.aul2", true, true),
                language_file("Language/english.aul2", true, true),
                language_file("Language/a/../English.aul2", true, true),
            ]),
        );

        let requests =
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap();

        assert_eq!(requests.len(), 3);
    }

    #[test]
    fn path_validation_precedes_duplicate_detection() {
        let duplicate = PathBuf::from("Language/English.aul2");
        let invalid = PathBuf::from("Language/Invalid.txt");
        let config = config(
            "ignored",
            language(vec![
                language_file(&duplicate, true, true),
                language_file(&duplicate, true, true),
                language_file(&invalid, true, true),
            ]),
        );

        assert_eq!(
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap_err(),
            ResolveLanguageFileRequestsError::InvalidExtension {
                path: invalid,
                origin: configured_origin(2),
            }
        );
    }

    #[test]
    fn duplicate_detection_fails_on_the_first_duplicate_in_configuration_order() {
        let first = PathBuf::from("Language/First.aul2");
        let second = PathBuf::from("Language/Second.aul2");
        let config = config(
            "ignored",
            language(vec![
                language_file(&first, true, true),
                language_file(&second, true, true),
                language_file(&first, true, true),
                language_file(&second, true, true),
            ]),
        );

        assert_eq!(
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap_err(),
            ResolveLanguageFileRequestsError::DuplicatePath {
                path: first,
                first_origin: configured_origin(0),
                duplicate_origin: configured_origin(2),
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_file_name_is_an_error() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let mut bytes = b"Language/Invalid-".to_vec();
        bytes.push(0xff);
        bytes.extend_from_slice(b".aul2");
        let path = PathBuf::from(OsString::from_vec(bytes));
        let config = config("ignored", language(vec![language_file(&path, true, true)]));

        assert_eq!(
            resolve_language_file_requests(&config, LanguageFileSelection::Configured).unwrap_err(),
            ResolveLanguageFileRequestsError::NonUtf8FileName {
                path,
                origin: configured_origin(0),
            }
        );
    }
}
