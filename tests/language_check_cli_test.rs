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

fn configured_project(temp: &TempDir, source: &str) {
    write_project_file(
        temp.path(),
        "aulua.yaml",
        "scripts:\n  - name: effect.anm2\n    sources:\n      - path: effect.lua\nlanguage:\n  files:\n    - path: English.aul2\n",
    );
    write_project_file(temp.path(), "effect.lua", source);
}

#[test]
fn direct_script_and_target_override_can_be_clean_without_output() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        "scripts:\n  - name: broken.anm2\n    sources:\n      - path: missing.lua\n    language:\n      files:\n        - path: invalid.txt\n        - path: invalid.txt\nlanguage:\n  files:\n    - path: missing-configured.aul2\n",
    );
    write_project_file(
        temp.path(),
        "direct.anm2",
        "--check@enabled:Enabled,false\n",
    );
    write_project_file(
        temp.path(),
        "target.aul2",
        "[direct]\ndirect=Translated\nEnabled=Translated\n",
    );

    let output = run_aulua(
        temp.path(),
        &[
            "language",
            "check",
            "--script",
            "direct.anm2",
            "--target",
            "target.aul2",
        ],
    );

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn multiple_findings_use_typed_order_and_optional_source_lines() {
    let temp = TempDir::new().unwrap();
    configured_project(
        &temp,
        "---$script_tips:Effect tips\n---$track:Missing\nlocal missing = 0\n---$track:Empty\nlocal empty = 0\n",
    );
    write_project_file(
        temp.path(),
        "English.aul2",
        "; preamble\n[effect]\neffect=Translated\nEmpty=\nOld=Unused\n",
    );

    let output = run_aulua(temp.path(), &["language", "check"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "English.aul2: keyが存在しません: [effect] Missing\nEnglish.aul2:4: 値が空です: [effect] Empty\nEnglish.aul2:5: 使用されていないkeyです: [effect] Old\nEnglish.aul2: sectionが存在しません: [Tips.effect]\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn missing_file_is_one_file_level_finding() {
    let temp = TempDir::new().unwrap();
    configured_project(&temp, "---$track:Speed\nlocal speed = 0\n");

    let output = run_aulua(temp.path(), &["language", "check"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "English.aul2: language fileが存在しません\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn warning_only_is_stderr_and_keeps_clean_exit_status() {
    let temp = TempDir::new().unwrap();
    configured_project(&temp, "---$track:Speed\nlocal value = \"${MISSING}\"\n");
    write_project_file(
        temp.path(),
        "English.aul2",
        "[effect]\neffect=Translated\nSpeed=Translated\n",
    );

    let output = run_aulua(temp.path(), &["language", "check"]);

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("⚠️ 未定義の変数: ${MISSING}").count(), 1);
    assert!(stderr.contains("effect.lua"));
}

#[test]
fn malformed_language_file_is_an_execution_error_not_a_finding() {
    let temp = TempDir::new().unwrap();
    configured_project(&temp, "---$track:Speed\nlocal speed = 0\n");
    write_project_file(temp.path(), "English.aul2", "malformed\n");

    let output = run_aulua(temp.path(), &["language", "check"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("language fileのcheckに失敗しました")
    );
}

#[test]
fn configured_check_uses_each_script_specific_scope() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        r#"scripts:
  - name: foo.anm2
    sources:
      - path: foo.lua
    language:
      files:
        - path: Foo.aul2
  - name: bar.anm2
    sources:
      - path: bar.lua
    language:
      files:
        - path: Bar.aul2
"#,
    );
    write_project_file(
        temp.path(),
        "foo.lua",
        "---$track:FooValue\nlocal value = 0\n",
    );
    write_project_file(
        temp.path(),
        "bar.lua",
        "---$track:BarValue\nlocal value = 0\n",
    );
    write_project_file(
        temp.path(),
        "Foo.aul2",
        "[foo]\nfoo=Translated\nFooValue=Translated\n\n[bar]\nUnmanaged=\n",
    );
    write_project_file(
        temp.path(),
        "Bar.aul2",
        "[bar]\nbar=Translated\nBarValue=Translated\n",
    );

    let output = run_aulua(temp.path(), &["language", "check"]);

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn nested_only_check_skips_unscoped_script_content_analysis() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        r#"scripts:
  - name: included.anm2
    sources:
      - path: included.lua
    language:
      files:
        - path: Included.aul2
  - name: skipped.anm2
    sources:
      - path: missing.lua
"#,
    );
    write_project_file(
        temp.path(),
        "included.lua",
        "---$track:Value\nlocal value = 0\n",
    );
    write_project_file(
        temp.path(),
        "Included.aul2",
        "[included]\nincluded=Translated\nValue=Translated\n",
    );

    let output = run_aulua(temp.path(), &["language", "check"]);

    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
