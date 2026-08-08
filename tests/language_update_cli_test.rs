use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn run_aulua(project_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aulua"))
        .current_dir(project_dir)
        .args(args)
        .output()
        .unwrap()
}

fn write_project_file(project_dir: &Path, relative_path: &str, content: &str) {
    let path = project_dir.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

#[test]
fn configured_update_creates_language_file_and_prints_warning_once() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        "scripts:\n  - name: effect.anm2\n    sources:\n      - path: effect.lua\nlanguage:\n  files:\n    - path: Language/English.aul2\n",
    );
    write_project_file(
        temp.path(),
        "effect.lua",
        "---$track:Speed\nlocal value = \"${MISSING}\"\n",
    );

    let output = run_aulua(temp.path(), &["language", "update"]);

    assert!(output.status.success(), "{output:?}");
    let language = fs::read_to_string(temp.path().join("Language/English.aul2")).unwrap();
    assert_eq!(language, "[effect]\neffect=\nSpeed=\n");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("⚠️ 未定義の変数: ${MISSING}").count(), 1);
    assert!(stderr.contains("effect.lua"));
}

#[test]
fn direct_scripts_and_output_override_ignore_configured_inputs_and_files() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        "scripts:\n  - name: broken.anm2\n    sources:\n      - path: missing.lua\nlanguage:\n  files:\n    - path: Language/configured.aul2\n",
    );
    write_project_file(
        temp.path(),
        "direct.anm2",
        "--check@enabled:Enabled,false\n",
    );

    let output = run_aulua(
        temp.path(),
        &[
            "language",
            "update",
            "--script",
            "direct.anm2",
            "--output",
            "Language/override.aul2",
        ],
    );

    assert!(output.status.success(), "{output:?}");
    assert!(!temp.path().join("Language/configured.aul2").exists());
    let language = fs::read_to_string(temp.path().join("Language/override.aul2")).unwrap();
    assert!(language.contains("[direct]"));
    assert!(language.contains("Enabled="));
}

#[test]
fn prune_flag_removes_unused_entries_while_normal_update_keeps_them() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        "scripts:\n  - name: effect.anm2\n    sources:\n      - path: effect.lua\nlanguage:\n  files:\n    - path: English.aul2\n",
    );
    write_project_file(
        temp.path(),
        "effect.lua",
        "---$track:Speed\nlocal speed = 0\n",
    );
    let existing = "[effect]\neffect=Translated\nSpeed=Translated speed\nold=Unused\n";
    write_project_file(temp.path(), "English.aul2", existing);

    let normal = run_aulua(temp.path(), &["language", "update"]);
    assert!(normal.status.success(), "{normal:?}");
    assert_eq!(
        fs::read_to_string(temp.path().join("English.aul2")).unwrap(),
        existing
    );

    let pruned = run_aulua(temp.path(), &["language", "update", "--prune"]);
    assert!(pruned.status.success(), "{pruned:?}");
    assert_eq!(
        fs::read_to_string(temp.path().join("English.aul2")).unwrap(),
        "[effect]\neffect=Translated\nSpeed=Translated speed\n"
    );
}

#[test]
fn config_errors_make_the_command_fail() {
    let temp = TempDir::new().unwrap();

    let output = run_aulua(temp.path(), &["language", "update"]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("language fileの更新に失敗しました")
    );
}
