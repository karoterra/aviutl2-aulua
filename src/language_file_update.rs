use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;
use thiserror::Error;

use crate::aul2_document_builder::build_new_aul2_document;
use crate::aul2_parser::{ParseAul2DocumentError, parse_aul2_document};
use crate::aul2_prune::prune_aul2_document;
use crate::aul2_serializer::serialize_aul2_document;
use crate::aul2_update::update_aul2_document;
use crate::language_file_plan::LanguageFilePlan;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UpdateLanguageFilesOptions {
    pub prune: bool,
}

#[derive(Debug, Error)]
pub(crate) enum UpdateLanguageFilesError {
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

    #[error(
        "language fileの親directory作成に失敗しました: {}",
        path.display()
    )]
    CreateParent {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("language file用temporary fileの作成に失敗しました: {}", path.display())]
    CreateTemporary {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("language file用temporary fileの書き込みに失敗しました: {}", path.display())]
    WriteTemporary {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("language fileの置換に失敗しました: {}", path.display())]
    Persist {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub(crate) fn update_language_files(
    plans: &[LanguageFilePlan],
    options: UpdateLanguageFilesOptions,
) -> Result<(), UpdateLanguageFilesError> {
    let prepared = plans
        .iter()
        .map(|plan| prepare_language_file_update(plan, options))
        .collect::<Result<Vec<_>, _>>()?;

    for update in prepared.iter().filter(|update| update.needs_write) {
        create_parent_directory(&update.path)?;
        write_language_file_atomically(&update.path, &update.content)?;
    }

    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct PreparedLanguageFileUpdate {
    path: PathBuf,
    content: String,
    needs_write: bool,
}

fn prepare_language_file_update(
    plan: &LanguageFilePlan,
    options: UpdateLanguageFilesOptions,
) -> Result<PreparedLanguageFileUpdate, UpdateLanguageFilesError> {
    let path = &plan.request.path;

    match fs::read_to_string(path) {
        Ok(original) => {
            let mut document = parse_aul2_document(&original).map_err(|source| {
                UpdateLanguageFilesError::Parse {
                    path: path.clone(),
                    source,
                }
            })?;
            update_aul2_document(&mut document, plan);
            if options.prune {
                prune_aul2_document(&mut document, plan);
            }
            let content = serialize_aul2_document(&document);
            let needs_write = content != original;

            Ok(PreparedLanguageFileUpdate {
                path: path.clone(),
                content,
                needs_write,
            })
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            let document = build_new_aul2_document(plan);
            let content = serialize_aul2_document(&document);

            Ok(PreparedLanguageFileUpdate {
                path: path.clone(),
                content,
                needs_write: true,
            })
        }
        Err(source) => Err(UpdateLanguageFilesError::Read {
            path: path.clone(),
            source,
        }),
    }
}

fn create_parent_directory(path: &Path) -> Result<(), UpdateLanguageFilesError> {
    let Some(parent) = nonempty_parent(path) else {
        return Ok(());
    };

    fs::create_dir_all(parent).map_err(|source| UpdateLanguageFilesError::CreateParent {
        path: path.to_path_buf(),
        source,
    })
}

fn write_language_file_atomically(
    path: &Path,
    content: &str,
) -> Result<(), UpdateLanguageFilesError> {
    let temporary_directory = nonempty_parent(path).unwrap_or_else(|| Path::new("."));
    let mut temporary = NamedTempFile::new_in(temporary_directory).map_err(|source| {
        UpdateLanguageFilesError::CreateTemporary {
            path: path.to_path_buf(),
            source,
        }
    })?;

    temporary
        .write_all(content.as_bytes())
        .and_then(|()| temporary.flush())
        .map_err(|source| UpdateLanguageFilesError::WriteTemporary {
            path: path.to_path_buf(),
            source,
        })?;

    temporary
        .persist(path)
        .map_err(|error| UpdateLanguageFilesError::Persist {
            path: path.to_path_buf(),
            source: error.error,
        })?;

    Ok(())
}

fn nonempty_parent(path: &Path) -> Option<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::common::get_fixture_path;
    use crate::config_loader::load_config;
    use crate::language_file_plan::{LanguageSectionPlan, LanguageTextEntryPlan};
    use crate::language_file_request::{
        LanguageFileRequest, LanguageFileRequestOrigin, LanguageFileSelection,
    };
    use crate::language_plan::build_language_plan;
    use crate::language_script_analysis::{LanguageScriptInput, LogicalScriptOrigin};
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

    fn text_section(
        name: &str,
        entries: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> LanguageSectionPlan {
        LanguageSectionPlan::Text {
            name: name.to_string(),
            logical_script_name: name.to_string(),
            origin: LogicalScriptOrigin::Direct {
                path: PathBuf::from("script.anm2"),
            },
            entries: entries
                .into_iter()
                .map(|(key, value)| LanguageTextEntryPlan {
                    key: key.to_string(),
                    value: value.to_string(),
                    origins: vec![LanguageTextOrigin::ScriptName],
                })
                .collect(),
        }
    }

    #[test]
    fn missing_nonempty_and_empty_files_are_created() {
        let temp = TempDir::new().unwrap();
        let nonempty_path = temp.path().join("Nonempty.aul2");
        let empty_path = temp.path().join("Empty.aul2");
        let plans = [
            plan(
                &nonempty_path,
                vec![text_section("section", [("key", "value")])],
            ),
            plan(&empty_path, Vec::new()),
        ];

        update_language_files(&plans, UpdateLanguageFilesOptions::default()).unwrap();

        assert_eq!(
            fs::read_to_string(nonempty_path).unwrap(),
            "[section]\nkey=value\n"
        );
        assert_eq!(fs::read_to_string(empty_path).unwrap(), "");
    }

    #[test]
    fn missing_parent_directories_are_created_only_for_writes() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("nested/deep/English.aul2");

        update_language_files(
            &[plan(&path, vec![text_section("section", [("key", "")])])],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), "[section]\nkey=\n");
    }

    #[test]
    fn existing_update_fixture_matches_expected_file() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.basic.aul2");
        fs::copy(
            get_fixture_path("language/update/input/English.basic.aul2"),
            &path,
        )
        .unwrap();
        let config = load_config(get_fixture_path("language/update/input/aulua.yaml")).unwrap();
        let language_plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();
        let mut file_plan = language_plan.files[0].clone();
        file_plan.request.path = path.clone();

        update_language_files(&[file_plan], UpdateLanguageFilesOptions::default()).unwrap();

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            fs::read_to_string(get_fixture_path(
                "language/update/expected/English.basic.aul2"
            ))
            .unwrap()
        );
    }

    #[test]
    fn existing_empty_file_keeps_missing_final_newline() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        fs::write(&path, "").unwrap();

        update_language_files(
            &[plan(&path, vec![text_section("section", [("key", "")])])],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), "[section]\nkey=");
    }

    #[test]
    fn parse_error_leaves_single_target_unchanged_with_path_context() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("Broken.aul2");
        let original = "[broken\nkey=value\n";
        fs::write(&path, original).unwrap();

        let error = update_language_files(
            &[plan(&path, Vec::new())],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            UpdateLanguageFilesError::Parse { path: error_path, .. } if error_path == path
        ));
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }

    #[test]
    fn multi_file_preflight_keeps_earlier_file_unchanged_on_later_parse_error() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("First.aul2");
        let second_path = temp.path().join("Second.aul2");
        let first_original = "[section]\nexisting=keep\n";
        let second_original = "[broken";
        fs::write(&first_path, first_original).unwrap();
        fs::write(&second_path, second_original).unwrap();
        let plans = [
            plan(
                &first_path,
                vec![text_section("section", [("missing", "")])],
            ),
            plan(&second_path, Vec::new()),
        ];

        assert!(matches!(
            update_language_files(&plans, UpdateLanguageFilesOptions::default()),
            Err(UpdateLanguageFilesError::Parse { path, .. }) if path == second_path
        ));
        assert_eq!(fs::read_to_string(first_path).unwrap(), first_original);
        assert_eq!(fs::read_to_string(second_path).unwrap(), second_original);
    }

    #[test]
    fn invalid_utf8_is_a_read_error_and_leaves_file_unchanged() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("Invalid.aul2");
        let original = [0xff, 0xfe];
        fs::write(&path, original).unwrap();

        let error = update_language_files(
            &[plan(&path, Vec::new())],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            UpdateLanguageFilesError::Read { path: error_path, .. } if error_path == path
        ));
        assert_eq!(fs::read(path).unwrap(), original);
    }

    #[test]
    fn bom_and_crlf_are_normalized_without_semantic_changes() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        fs::write(&path, "\u{feff}[section]\r\nkey=value\r\n").unwrap();

        update_language_files(
            &[plan(
                &path,
                vec![text_section("section", [("key", "different")])],
            )],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap();

        assert_eq!(fs::read_to_string(path).unwrap(), "[section]\nkey=value\n");
    }

    #[test]
    fn canonical_unchanged_file_is_prepared_without_write() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        let original = "[section]\nkey=value\n";
        fs::write(&path, original).unwrap();
        let file_plan = plan(&path, vec![text_section("section", [("key", "different")])]);

        let prepared =
            prepare_language_file_update(&file_plan, UpdateLanguageFilesOptions::default())
                .unwrap();

        assert_eq!(prepared.content, original);
        assert!(!prepared.needs_write);
    }

    #[test]
    fn parent_path_conflict_reports_create_parent_error() {
        let temp = TempDir::new().unwrap();
        let blocking_path = temp.path().join("blocking");
        let target_path = blocking_path.join("English.aul2");
        fs::write(&blocking_path, "not a directory").unwrap();

        let error = create_parent_directory(&target_path).unwrap_err();

        assert!(matches!(
            error,
            UpdateLanguageFilesError::CreateParent { path, .. } if path == target_path
        ));
        assert_eq!(
            fs::read_to_string(blocking_path).unwrap(),
            "not a directory"
        );
    }

    #[test]
    fn prune_adds_missing_entry_removes_unused_and_keeps_existing_value() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        fs::write(&path, "[section]\nrequired=translated\nold=unused\n").unwrap();
        let file_plan = plan(
            &path,
            vec![text_section(
                "section",
                [("required", "different"), ("missing", "")],
            )],
        );

        update_language_files(&[file_plan], UpdateLanguageFilesOptions { prune: true }).unwrap();

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "[section]\nrequired=translated\nmissing=\n"
        );
    }

    #[test]
    fn prune_preserves_comments_blanks_and_unknown_sections() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        fs::write(
            &path,
            "[section]\nrequired=value\n; keep\nold=unused\n\n[Unknown]\nold=keep\n",
        )
        .unwrap();

        update_language_files(
            &[plan(
                &path,
                vec![text_section("section", [("required", "different")])],
            )],
            UpdateLanguageFilesOptions { prune: true },
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "[section]\nrequired=value\n; keep\n\n[Unknown]\nold=keep\n"
        );
    }

    #[test]
    fn missing_files_have_identical_content_with_or_without_prune() {
        let temp = TempDir::new().unwrap();
        let normal_path = temp.path().join("Normal.aul2");
        let pruned_path = temp.path().join("Pruned.aul2");
        let normal_empty_path = temp.path().join("NormalEmpty.aul2");
        let pruned_empty_path = temp.path().join("PrunedEmpty.aul2");

        update_language_files(
            &[plan(
                &normal_path,
                vec![text_section("section", [("key", "value")])],
            )],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap();
        update_language_files(
            &[plan(
                &pruned_path,
                vec![text_section("section", [("key", "value")])],
            )],
            UpdateLanguageFilesOptions { prune: true },
        )
        .unwrap();
        update_language_files(
            &[plan(&normal_empty_path, Vec::new())],
            UpdateLanguageFilesOptions::default(),
        )
        .unwrap();
        update_language_files(
            &[plan(&pruned_empty_path, Vec::new())],
            UpdateLanguageFilesOptions { prune: true },
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(normal_path).unwrap(),
            fs::read_to_string(pruned_path).unwrap()
        );
        assert_eq!(fs::read_to_string(normal_empty_path).unwrap(), "");
        assert_eq!(fs::read_to_string(pruned_empty_path).unwrap(), "");
    }

    #[test]
    fn prune_only_difference_controls_needs_write() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        let original = "[section]\nrequired=value\nold=unused\n";
        fs::write(&path, original).unwrap();
        let file_plan = plan(
            &path,
            vec![text_section("section", [("required", "different")])],
        );

        let normal =
            prepare_language_file_update(&file_plan, UpdateLanguageFilesOptions::default())
                .unwrap();
        let pruned =
            prepare_language_file_update(&file_plan, UpdateLanguageFilesOptions { prune: true })
                .unwrap();

        assert_eq!(normal.content, original);
        assert!(!normal.needs_write);
        assert_eq!(pruned.content, "[section]\nrequired=value\n");
        assert!(pruned.needs_write);
    }

    #[test]
    fn prune_preflight_keeps_all_files_unchanged_on_later_parse_error() {
        let temp = TempDir::new().unwrap();
        let first_path = temp.path().join("First.aul2");
        let second_path = temp.path().join("Second.aul2");
        let first_original = "[section]\nrequired=value\nold=unused\n";
        let second_original = "[broken";
        fs::write(&first_path, first_original).unwrap();
        fs::write(&second_path, second_original).unwrap();
        let plans = [
            plan(
                &first_path,
                vec![text_section("section", [("required", "different")])],
            ),
            plan(&second_path, Vec::new()),
        ];

        assert!(matches!(
            update_language_files(&plans, UpdateLanguageFilesOptions { prune: true }),
            Err(UpdateLanguageFilesError::Parse { path, .. }) if path == second_path
        ));
        assert_eq!(fs::read_to_string(first_path).unwrap(), first_original);
        assert_eq!(fs::read_to_string(second_path).unwrap(), second_original);
    }

    #[test]
    fn prune_preserves_missing_final_newline() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("English.aul2");
        fs::write(&path, "[section]\nrequired=value\nold=unused").unwrap();

        update_language_files(
            &[plan(
                &path,
                vec![text_section("section", [("required", "different")])],
            )],
            UpdateLanguageFilesOptions { prune: true },
        )
        .unwrap();

        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "[section]\nrequired=value"
        );
    }
}
