use thiserror::Error;

use crate::config::ResolvedConfig;
use crate::language_file_plan::{LanguageFilePlan, build_language_file_plans};
use crate::language_file_request::{
    LanguageFileSelection, ResolveLanguageFileRequestsError, resolve_language_file_requests,
};
use crate::language_script_analysis::{
    AnalyzeLanguageScriptsError, AnalyzedLogicalScript, LanguageScriptInput,
    analyze_language_scripts,
};
use crate::language_section_catalog::{
    BuildLanguageSectionCatalogError, build_language_section_catalog,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguagePlan {
    pub scripts: Vec<AnalyzedLogicalScript>,
    pub files: Vec<LanguageFilePlan>,
}

#[derive(Debug, Error)]
pub(crate) enum BuildLanguagePlanError {
    #[error(transparent)]
    Scripts(#[from] AnalyzeLanguageScriptsError),
    #[error(transparent)]
    Sections(#[from] BuildLanguageSectionCatalogError),
    #[error(transparent)]
    FileRequests(#[from] ResolveLanguageFileRequestsError),
}

pub(crate) fn build_language_plan(
    config: &ResolvedConfig,
    script_input: LanguageScriptInput<'_>,
    file_selection: LanguageFileSelection<'_>,
) -> Result<LanguagePlan, BuildLanguagePlanError> {
    let scripts = analyze_language_scripts(config, script_input)?;
    let catalog = build_language_section_catalog(&scripts)?;
    let requests = resolve_language_file_requests(config, file_selection)?;
    let files = build_language_file_plans(&requests, &catalog);

    Ok(LanguagePlan { scripts, files })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::*;
    use crate::common::get_fixture_path;
    use crate::config::{
        ResolvedBuild, ResolvedInstall, ResolvedLanguage, ResolvedLanguageFile, ResolvedProject,
        ResolvedScript, ResolvedScriptSource,
    };
    use crate::config_loader::load_config;
    use crate::language_file_plan::LanguageSectionPlan;
    use crate::language_file_request::LanguageFileRequestOrigin;
    use crate::language_script_analysis::{LanguageAnalysisWarning, LogicalScriptOrigin};

    fn config(
        config_dir: impl Into<PathBuf>,
        scripts: Vec<ResolvedScript>,
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
            scripts,
            config_dir: config_dir.into(),
        }
    }

    fn language(files: impl IntoIterator<Item = (PathBuf, bool, bool)>) -> ResolvedLanguage {
        ResolvedLanguage {
            files: files
                .into_iter()
                .map(|(path, text, tooltip)| ResolvedLanguageFile {
                    path,
                    text,
                    tooltip,
                })
                .collect(),
        }
    }

    fn script(name: &str, source_paths: impl IntoIterator<Item = PathBuf>) -> ResolvedScript {
        ResolvedScript {
            name: name.to_string(),
            sources: source_paths
                .into_iter()
                .map(|path| ResolvedScriptSource {
                    path,
                    label: None,
                    variables: HashMap::new(),
                })
                .collect(),
        }
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    fn section_names(file: &LanguageFilePlan) -> Vec<&str> {
        file.sections.iter().map(|section| section.name()).collect()
    }

    #[test]
    fn builds_configured_plan_from_basic_fixture() {
        let config_path = get_fixture_path("language/basic/input/aulua.yaml");
        let config = load_config(config_path).unwrap();

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();

        assert_eq!(
            plan.scripts
                .iter()
                .map(|script| script.prepared.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "single",
                "Combined Part 1@combined",
                "Combined Part 2@combined"
            ]
        );
        assert_eq!(
            plan.scripts[0].prepared.origin,
            LogicalScriptOrigin::Configured {
                source_paths: vec![config.scripts[0].sources[0].path.clone()]
            }
        );

        assert_eq!(plan.files.len(), 2);
        assert_eq!(
            plan.files[0].request.origin,
            LanguageFileRequestOrigin::Configured { index: 0 }
        );
        assert_eq!(
            plan.files[1].request.origin,
            LanguageFileRequestOrigin::Configured { index: 1 }
        );
        assert!(plan.files[0].request.is_default);
        assert!(!plan.files[0].request.text);
        assert!(plan.files[0].request.tooltip);
        assert_eq!(
            section_names(&plan.files[0]),
            vec!["Tips.single", "Tips.Combined Part 1@combined"]
        );
        assert_eq!(
            section_names(&plan.files[1]),
            vec![
                "single",
                "Tips.single",
                "Combined Part 1@combined",
                "Tips.Combined Part 1@combined",
                "Combined Part 2@combined"
            ]
        );

        let LanguageSectionPlan::Tooltip { entries, .. } = &plan.files[0].sections[0] else {
            panic!("Default fileの先頭sectionはTooltipのはずです");
        };
        assert!(entries[0].value.contains('\n'));
        assert!(!entries[0].value.is_empty());

        for section in &plan.files[1].sections {
            match section {
                LanguageSectionPlan::Text { entries, .. } => {
                    assert!(entries.iter().all(|entry| entry.value.is_empty()));
                }
                LanguageSectionPlan::Tooltip { entries, .. } => {
                    assert!(entries.iter().all(|entry| entry.value.is_empty()));
                }
            }
        }
    }

    #[test]
    fn builds_direct_default_override_without_using_configured_inputs() {
        let temp = TempDir::new().unwrap();
        let direct_path = temp.path().join("direct.anm2");
        write(
            &direct_path,
            "---$script_tips:Direct description\n--check@enabled:Enabled,false\n",
        );
        let broken_configured = ResolvedScript {
            name: "broken.anm2".to_string(),
            sources: Vec::new(),
        };
        let config = config(
            temp.path(),
            vec![broken_configured],
            Some(language([(PathBuf::from("invalid.txt"), true, true)])),
        );
        let direct_paths = vec![direct_path.clone()];

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Direct(&direct_paths),
            LanguageFileSelection::Override(Path::new("Default.direct.aul2")),
        )
        .unwrap();

        assert_eq!(plan.scripts.len(), 1);
        assert_eq!(plan.scripts[0].prepared.name, "direct");
        assert_eq!(
            plan.scripts[0].prepared.origin,
            LogicalScriptOrigin::Direct { path: direct_path }
        );
        assert!(plan.scripts[0].prepared.warnings.is_empty());

        assert_eq!(plan.files.len(), 1);
        assert_eq!(
            plan.files[0].request.path,
            temp.path().join("Default.direct.aul2")
        );
        assert_eq!(
            plan.files[0].request.origin,
            LanguageFileRequestOrigin::Override
        );
        assert!(plan.files[0].request.is_default);
        assert_eq!(section_names(&plan.files[0]), vec!["direct", "Tips.direct"]);

        let LanguageSectionPlan::Text { entries, .. } = &plan.files[0].sections[0] else {
            panic!("先頭sectionはTextのはずです");
        };
        assert_eq!(entries[0].key, "direct");
        assert_eq!(entries[0].value, "direct");
        let LanguageSectionPlan::Tooltip { entries, .. } = &plan.files[0].sections[1] else {
            panic!("2番目のsectionはTooltipのはずです");
        };
        assert_eq!(entries[0].key, "effect.name");
        assert_eq!(entries[0].value, "Direct description");
    }

    #[test]
    fn retains_warning_with_script_when_file_plans_have_no_sections() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("warning.lua");
        write(&source_path, "---$nolang: script_name\n-- ${MISSING}\n");
        let default_path = temp.path().join("Default.test.aul2");
        let translation_path = temp.path().join("English.test.aul2");
        let config = config(
            temp.path(),
            vec![script("warning.anm2", [source_path.clone()])],
            Some(language([
                (default_path, true, true),
                (translation_path, true, true),
            ])),
        );

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();

        assert_eq!(plan.scripts.len(), 1);
        assert_eq!(plan.scripts[0].prepared.name, "warning");
        assert_eq!(
            plan.scripts[0].prepared.warnings,
            vec![LanguageAnalysisWarning::UndefinedVariable {
                source_path,
                variable_name: "MISSING".to_string(),
            }]
        );
        assert_eq!(plan.files.len(), 2);
        assert!(plan.files.iter().all(|file| file.sections.is_empty()));
    }

    #[test]
    fn wraps_errors_from_each_fallible_stage() {
        let temp = TempDir::new().unwrap();
        let empty_config = config(temp.path(), Vec::new(), None);
        let override_path = Path::new("English.aul2");

        assert!(matches!(
            build_language_plan(
                &empty_config,
                LanguageScriptInput::Direct(&[]),
                LanguageFileSelection::Override(override_path),
            ),
            Err(BuildLanguagePlanError::Scripts(_))
        ));

        let foo_path = temp.path().join("foo.anm2");
        let tips_foo_path = temp.path().join("Tips.foo.anm2");
        write(&foo_path, "---$script_tips:tips\n");
        write(&tips_foo_path, "");
        let paths = vec![foo_path, tips_foo_path];
        assert!(matches!(
            build_language_plan(
                &empty_config,
                LanguageScriptInput::Direct(&paths),
                LanguageFileSelection::Override(override_path),
            ),
            Err(BuildLanguagePlanError::Sections(_))
        ));

        assert!(matches!(
            build_language_plan(
                &empty_config,
                LanguageScriptInput::Configured,
                LanguageFileSelection::Configured,
            ),
            Err(BuildLanguagePlanError::FileRequests(_))
        ));
    }
}
