use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::aul2_check::LanguageCheckFinding;
use crate::config_loader::load_config;
use crate::language_file_check::{
    LanguageCheckResult, LanguageFileCheckResult, check_language_files,
};
use crate::language_file_request::LanguageFileSelection;
use crate::language_file_update::{UpdateLanguageFilesOptions, update_language_files};
use crate::language_plan::{LanguagePlan, build_language_plan};
use crate::language_script_analysis::{LanguageAnalysisWarning, LanguageScriptInput};

/// `language check`の実行結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageCheckStatus {
    /// findingは検出されなかった。
    Clean,
    /// findingが1件以上検出された。
    Findings,
}

/// `aulua.yaml` とCLIの選択内容に基づいてlanguage fileを作成・更新する。
pub fn update_language(
    scripts: &[PathBuf],
    output: Option<&Path>,
    prune: bool,
) -> anyhow::Result<()> {
    update_language_from_config(Path::new("aulua.yaml"), scripts, output, prune)
}

fn update_language_from_config(
    config_path: &Path,
    scripts: &[PathBuf],
    output: Option<&Path>,
    prune: bool,
) -> anyhow::Result<()> {
    let plan = build_language_plan_from_cli(config_path, scripts, output)?;
    print_language_warnings(&plan);

    update_language_files(&plan.files, UpdateLanguageFilesOptions { prune })
        .context("language fileの更新に失敗しました")
}

/// `aulua.yaml` とCLIの選択内容に基づいてlanguage fileをcheckする。
pub fn check_language(
    scripts: &[PathBuf],
    target: Option<&Path>,
) -> anyhow::Result<LanguageCheckStatus> {
    check_language_from_config(Path::new("aulua.yaml"), scripts, target)
}

fn check_language_from_config(
    config_path: &Path,
    scripts: &[PathBuf],
    target: Option<&Path>,
) -> anyhow::Result<LanguageCheckStatus> {
    let plan = build_language_plan_from_cli(config_path, scripts, target)?;
    print_language_warnings(&plan);

    let result = check_language_files(&plan.files).context("language fileのcheckに失敗しました")?;
    print_language_check_result(&result);

    Ok(if result.has_findings() {
        LanguageCheckStatus::Findings
    } else {
        LanguageCheckStatus::Clean
    })
}

fn build_language_plan_from_cli(
    config_path: &Path,
    scripts: &[PathBuf],
    file_override: Option<&Path>,
) -> anyhow::Result<LanguagePlan> {
    let config = load_config(config_path).context("設定ファイルの読み込みに失敗しました")?;
    let script_input = if scripts.is_empty() {
        LanguageScriptInput::Configured
    } else {
        LanguageScriptInput::Direct(scripts)
    };
    let file_selection = match file_override {
        Some(path) => LanguageFileSelection::Override(path),
        None => LanguageFileSelection::Configured,
    };
    build_language_plan(&config, script_input, file_selection)
        .context("language planの構築に失敗しました")
}

fn print_language_warnings(plan: &LanguagePlan) {
    for script in &plan.scripts {
        for warning in &script.prepared.warnings {
            print_language_warning(warning);
        }
    }
}

fn print_language_check_result(result: &LanguageCheckResult) {
    for file in &result.files {
        match file {
            LanguageFileCheckResult::MissingFile { path } => {
                print_diagnostic(path, None, "language fileが存在しません");
            }
            LanguageFileCheckResult::Checked { path, findings } => {
                for finding in findings {
                    match finding {
                        LanguageCheckFinding::MissingSection { section_name } => {
                            print_diagnostic(
                                path,
                                None,
                                &format!("sectionが存在しません: [{section_name}]"),
                            );
                        }
                        LanguageCheckFinding::MissingKey { section_name, key } => {
                            print_diagnostic(
                                path,
                                None,
                                &format!("keyが存在しません: [{section_name}] {key}"),
                            );
                        }
                        LanguageCheckFinding::EmptyValue {
                            section_name,
                            key,
                            source_line,
                        } => {
                            print_diagnostic(
                                path,
                                *source_line,
                                &format!("値が空です: [{section_name}] {key}"),
                            );
                        }
                        LanguageCheckFinding::UnusedKey {
                            section_name,
                            key,
                            source_line,
                        } => {
                            print_diagnostic(
                                path,
                                *source_line,
                                &format!("使用されていないkeyです: [{section_name}] {key}"),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn print_diagnostic(path: &Path, source_line: Option<usize>, diagnostic: &str) {
    match source_line {
        Some(line) => println!("{}:{line}: {diagnostic}", path.display()),
        None => println!("{}: {diagnostic}", path.display()),
    }
}

fn print_language_warning(warning: &LanguageAnalysisWarning) {
    match warning {
        LanguageAnalysisWarning::UndefinedVariable {
            source_path,
            variable_name,
        } => {
            eprintln!(
                "⚠️ 未定義の変数: ${{{variable_name}}} （{} 内）",
                source_path.display()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn updates_from_an_explicit_config_path_without_exposing_internal_plan_types() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("aulua.yaml");
        fs::write(
            &config_path,
            "scripts:\n  - name: effect.anm2\n    sources:\n      - path: effect.lua\nlanguage:\n  files:\n    - path: Language/English.aul2\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("effect.lua"),
            "---$track:Speed\nlocal speed = 0\n",
        )
        .unwrap();

        update_language_from_config(&config_path, &[], None, false).unwrap();

        let output = fs::read_to_string(temp.path().join("Language/English.aul2")).unwrap();
        assert_eq!(output, "[effect]\neffect=\nSpeed=\n");
    }

    #[test]
    fn returns_clean_or_findings_without_exposing_internal_check_types() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("aulua.yaml");
        let language_path = temp.path().join("English.aul2");
        fs::write(
            &config_path,
            "scripts:\n  - name: effect.anm2\n    sources:\n      - path: effect.lua\nlanguage:\n  files:\n    - path: English.aul2\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("effect.lua"),
            "---$track:Speed\nlocal speed = 0\n",
        )
        .unwrap();
        fs::write(
            &language_path,
            "[effect]\neffect=Translated\nSpeed=Translated\n",
        )
        .unwrap();

        assert_eq!(
            check_language_from_config(&config_path, &[], None).unwrap(),
            LanguageCheckStatus::Clean
        );

        fs::write(&language_path, "[effect]\neffect=Translated\n").unwrap();
        assert_eq!(
            check_language_from_config(&config_path, &[], None).unwrap(),
            LanguageCheckStatus::Findings
        );
    }
}
