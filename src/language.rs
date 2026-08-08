use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::config_loader::load_config;
use crate::language_file_request::LanguageFileSelection;
use crate::language_file_update::{UpdateLanguageFilesOptions, update_language_files};
use crate::language_plan::build_language_plan;
use crate::language_script_analysis::{LanguageAnalysisWarning, LanguageScriptInput};

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
    let config = load_config(config_path).context("設定ファイルの読み込みに失敗しました")?;
    let script_input = if scripts.is_empty() {
        LanguageScriptInput::Configured
    } else {
        LanguageScriptInput::Direct(scripts)
    };
    let file_selection = match output {
        Some(path) => LanguageFileSelection::Override(path),
        None => LanguageFileSelection::Configured,
    };
    let plan = build_language_plan(&config, script_input, file_selection)
        .context("language planの構築に失敗しました")?;

    for script in &plan.scripts {
        for warning in &script.prepared.warnings {
            print_language_warning(warning);
        }
    }

    update_language_files(&plan.files, UpdateLanguageFilesOptions { prune })
        .context("language fileの更新に失敗しました")
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
}
