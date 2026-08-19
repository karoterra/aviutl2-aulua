use std::collections::{HashMap, HashSet};

use thiserror::Error;

use crate::config::ResolvedConfig;
use crate::language_file_plan::{LanguageFilePlan, build_language_file_plan};
use crate::language_file_request::{
    LanguageFileScope, LanguageFileSelection, ResolveLanguageFileRequestsError,
    resolve_language_file_requests,
};
use crate::language_script_analysis::{
    AnalyzeLanguageScriptsError, AnalyzedLogicalScript, LanguageScriptInput,
    analyze_language_scripts_selected,
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
    let use_configured_scopes = matches!(&script_input, LanguageScriptInput::Configured)
        && matches!(file_selection, LanguageFileSelection::Configured);
    let selected_script_indices =
        configured_script_indices_for_analysis(config, &script_input, file_selection);
    let scripts =
        analyze_language_scripts_selected(config, script_input, selected_script_indices.as_ref())?;
    let requests = resolve_language_file_requests(config, file_selection)?;
    let mut catalogs = HashMap::new();
    let mut files = Vec::with_capacity(requests.len());

    for request in requests {
        let effective_scope = if use_configured_scopes {
            request.scope()
        } else {
            LanguageFileScope::All
        };

        let catalog = match catalogs.entry(effective_scope) {
            std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let catalog = build_language_section_catalog(
                    scripts
                        .iter()
                        .filter(|script| script_belongs_to_scope(script, effective_scope)),
                )?;
                entry.insert(catalog)
            }
        };
        files.push(build_language_file_plan(&request, catalog));
    }

    Ok(LanguagePlan { scripts, files })
}

fn configured_script_indices_for_analysis(
    config: &ResolvedConfig,
    script_input: &LanguageScriptInput<'_>,
    file_selection: LanguageFileSelection<'_>,
) -> Option<HashSet<usize>> {
    if !matches!(script_input, LanguageScriptInput::Configured)
        || matches!(file_selection, LanguageFileSelection::Override(_))
        || config
            .language
            .as_ref()
            .is_some_and(|language| !language.files.is_empty())
    {
        return None;
    }

    Some(
        config
            .scripts
            .iter()
            .enumerate()
            .filter_map(|(script_index, script)| {
                script
                    .language
                    .as_ref()
                    .is_some_and(|language| !language.files.is_empty())
                    .then_some(script_index)
            })
            .collect(),
    )
}

fn script_belongs_to_scope(script: &AnalyzedLogicalScript, scope: LanguageFileScope) -> bool {
    match scope {
        LanguageFileScope::All => true,
        LanguageFileScope::ConfiguredScript { script_index } => {
            script.configured_script_index == Some(script_index)
        }
    }
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
            language: None,
        }
    }

    fn with_language(mut script: ResolvedScript, language: ResolvedLanguage) -> ResolvedScript {
        script.language = Some(language);
        script
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
            language: None,
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

    #[test]
    fn configured_plan_applies_global_and_nested_scopes_in_file_order() {
        let temp = TempDir::new().unwrap();
        let foo_source = temp.path().join("foo.lua");
        let bar_source = temp.path().join("bar.lua");
        write(&foo_source, "---$track:FooValue\nlocal value = 0\n");
        write(&bar_source, "---$track:BarValue\nlocal value = 0\n");

        let global_path = temp.path().join("Global.aul2");
        let foo_path = temp.path().join("Foo.aul2");
        let bar_path = temp.path().join("Bar.aul2");
        let scripts = vec![
            with_language(
                script("foo.anm2", [foo_source]),
                language([(foo_path.clone(), true, true)]),
            ),
            with_language(
                script("bar.anm2", [bar_source]),
                language([(bar_path.clone(), true, true)]),
            ),
        ];
        let config = config(
            temp.path(),
            scripts,
            Some(language([(global_path.clone(), true, true)])),
        );

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();

        assert_eq!(
            plan.files
                .iter()
                .map(|file| file.request.path.clone())
                .collect::<Vec<_>>(),
            vec![global_path, foo_path, bar_path]
        );
        assert_eq!(section_names(&plan.files[0]), vec!["foo", "bar"]);
        assert_eq!(section_names(&plan.files[1]), vec!["foo"]);
        assert_eq!(section_names(&plan.files[2]), vec!["bar"]);
        assert_eq!(plan.scripts[0].configured_script_index, Some(0));
        assert_eq!(plan.scripts[1].configured_script_index, Some(1));
    }

    #[test]
    fn nested_only_plan_analyzes_only_scripts_with_language_files() {
        let temp = TempDir::new().unwrap();
        let included_source = temp.path().join("included.lua");
        write(&included_source, "---$track:Included\nlocal value = 0\n");
        let nested_path = temp.path().join("Nested.aul2");
        let scripts = vec![
            with_language(
                script("included.anm2", [included_source]),
                language([(nested_path, true, true)]),
            ),
            script("not-analyzed.anm2", [temp.path().join("missing.lua")]),
        ];
        let config = config(temp.path(), scripts, None);

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Configured,
        )
        .unwrap();

        assert_eq!(plan.scripts.len(), 1);
        assert_eq!(plan.scripts[0].prepared.name, "included");
        assert_eq!(section_names(&plan.files[0]), vec!["included"]);
    }

    #[test]
    fn nested_only_plan_still_validates_global_logical_script_name_uniqueness() {
        let temp = TempDir::new().unwrap();
        let first_source = temp.path().join("first.lua");
        let second_source = temp.path().join("second.lua");
        write(&first_source, "");
        write(&second_source, "");
        let scripts = vec![
            with_language(
                script("duplicate.anm2", [first_source]),
                language([(temp.path().join("Nested.aul2"), true, true)]),
            ),
            script("duplicate.anm2", [second_source]),
        ];
        let config = config(temp.path(), scripts, None);

        assert!(matches!(
            build_language_plan(
                &config,
                LanguageScriptInput::Configured,
                LanguageFileSelection::Configured,
            ),
            Err(BuildLanguagePlanError::Scripts(_))
        ));
    }

    #[test]
    fn section_name_collisions_are_checked_per_effective_scope() {
        let temp = TempDir::new().unwrap();
        let foo_source = temp.path().join("foo.lua");
        let tips_foo_source = temp.path().join("tips_foo.lua");
        write(&foo_source, "---$script_tips:Foo tips\n");
        write(&tips_foo_source, "");
        let scripts = vec![
            with_language(
                script("foo.anm2", [foo_source]),
                language([(temp.path().join("Foo.aul2"), true, true)]),
            ),
            with_language(
                script("Tips.foo.anm2", [tips_foo_source]),
                language([(temp.path().join("TipsFoo.aul2"), true, true)]),
            ),
        ];

        let nested_only = config(temp.path(), scripts, None);
        assert!(
            build_language_plan(
                &nested_only,
                LanguageScriptInput::Configured,
                LanguageFileSelection::Configured,
            )
            .is_ok()
        );

        let mut with_global = nested_only;
        with_global.language = Some(language([(temp.path().join("Global.aul2"), true, true)]));
        assert!(matches!(
            build_language_plan(
                &with_global,
                LanguageScriptInput::Configured,
                LanguageFileSelection::Configured,
            ),
            Err(BuildLanguagePlanError::Sections(_))
        ));
    }

    #[test]
    fn direct_selection_applies_one_catalog_to_global_and_nested_files() {
        let temp = TempDir::new().unwrap();
        let direct_path = temp.path().join("direct.anm2");
        write(&direct_path, "--check@enabled:Direct,false\n");
        let global_path = temp.path().join("Global.aul2");
        let nested_path = temp.path().join("Nested.aul2");
        let scripts = vec![with_language(
            script("configured.anm2", [temp.path().join("missing.lua")]),
            language([(nested_path, true, true)]),
        )];
        let config = config(
            temp.path(),
            scripts,
            Some(language([(global_path, true, true)])),
        );
        let direct_paths = vec![direct_path];

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Direct(&direct_paths),
            LanguageFileSelection::Configured,
        )
        .unwrap();

        assert_eq!(plan.files.len(), 2);
        assert!(
            plan.files
                .iter()
                .all(|file| section_names(file) == vec!["direct"])
        );
    }

    #[test]
    fn override_ignores_configured_files_and_analyzes_all_configured_scripts() {
        let temp = TempDir::new().unwrap();
        let foo_source = temp.path().join("foo.lua");
        let bar_source = temp.path().join("bar.lua");
        write(&foo_source, "");
        write(&bar_source, "");
        let invalid_language = language([
            (temp.path().join("Invalid.txt"), true, true),
            (temp.path().join("Invalid.txt"), true, true),
        ]);
        let scripts = vec![
            with_language(script("foo.anm2", [foo_source]), invalid_language),
            script("bar.anm2", [bar_source]),
        ];
        let config = config(temp.path(), scripts, None);

        let plan = build_language_plan(
            &config,
            LanguageScriptInput::Configured,
            LanguageFileSelection::Override(Path::new("Override.aul2")),
        )
        .unwrap();

        assert_eq!(plan.files.len(), 1);
        assert_eq!(section_names(&plan.files[0]), vec!["foo", "bar"]);
    }
}
