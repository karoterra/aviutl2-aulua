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
        "scripts:\n  - name: broken.anm2\n    sources:\n      - path: missing.lua\n    language:\n      files:\n        - path: invalid.txt\n        - path: invalid.txt\nlanguage:\n  files:\n    - path: Language/configured.aul2\n",
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

#[test]
fn configured_update_creates_global_and_script_specific_files() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        r#"language:
  files:
    - path: Language/Global.aul2
scripts:
  - name: foo.anm2
    sources:
      - path: foo.lua
    language:
      files:
        - path: Language/Foo.aul2
  - name: "@group.anm2"
    sources:
      - path: preamble.lua
      - path: first.lua
        label: First
      - path: second.lua
        label: Second
    language:
      files:
        - path: Language/Group.aul2
"#,
    );
    write_project_file(
        temp.path(),
        "foo.lua",
        "---$track:FooValue\nlocal value = 0\n",
    );
    write_project_file(
        temp.path(),
        "preamble.lua",
        "---$track:PreambleValue\nlocal value = 0\n",
    );
    write_project_file(
        temp.path(),
        "first.lua",
        "---$track:FirstValue\nlocal value = 0\n",
    );
    write_project_file(
        temp.path(),
        "second.lua",
        "---$track:SecondValue\nlocal value = 0\n",
    );

    let output = run_aulua(temp.path(), &["language", "update"]);

    assert!(output.status.success(), "{output:?}");
    let global = fs::read_to_string(temp.path().join("Language/Global.aul2")).unwrap();
    let foo = fs::read_to_string(temp.path().join("Language/Foo.aul2")).unwrap();
    let group = fs::read_to_string(temp.path().join("Language/Group.aul2")).unwrap();
    assert!(global.contains("[foo]"));
    assert!(global.contains("[First@group]"));
    assert!(global.contains("[Second@group]"));
    assert!(!global.contains("PreambleValue"));
    assert!(foo.contains("[foo]"));
    assert!(!foo.contains("[First@group]"));
    assert!(!foo.contains("[Second@group]"));
    assert!(group.contains("[First@group]"));
    assert!(group.contains("[Second@group]"));
    assert!(!group.contains("[foo]"));
}

#[test]
fn direct_script_without_output_updates_all_global_and_nested_files() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        r#"language:
  files:
    - path: Global.aul2
scripts:
  - name: configured.anm2
    sources:
      - path: missing.lua
    language:
      files:
        - path: Nested.aul2
"#,
    );
    write_project_file(
        temp.path(),
        "direct.anm2",
        "--check@enabled:Enabled,false\n",
    );

    let output = run_aulua(
        temp.path(),
        &["language", "update", "--script", "direct.anm2"],
    );

    assert!(output.status.success(), "{output:?}");
    for path in ["Global.aul2", "Nested.aul2"] {
        let language = fs::read_to_string(temp.path().join(path)).unwrap();
        assert!(language.contains("[direct]"), "{path}: {language}");
        assert!(language.contains("Enabled="), "{path}: {language}");
    }
}

#[test]
fn scoped_prune_does_not_manage_sections_from_other_scripts() {
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
"#,
    );
    write_project_file(
        temp.path(),
        "foo.lua",
        "---$track:Current\nlocal value = 0\n",
    );
    write_project_file(temp.path(), "bar.lua", "---$track:Bar\nlocal value = 0\n");
    write_project_file(
        temp.path(),
        "Foo.aul2",
        "[foo]\nfoo=Translated\nCurrent=Translated\nOld=Remove\n\n[bar]\nbar=Keep\nOld=Keep\n",
    );

    let output = run_aulua(temp.path(), &["language", "update", "--prune"]);

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(temp.path().join("Foo.aul2")).unwrap(),
        "[foo]\nfoo=Translated\nCurrent=Translated\n\n[bar]\nbar=Keep\nOld=Keep\n"
    );
}

#[test]
fn direct_tra2_update_outputs_supported_param_names_and_options() {
    let temp = TempDir::new().unwrap();
    write_project_file(
        temp.path(),
        "aulua.yaml",
        "scripts:\n  - name: broken.anm2\n    sources:\n      - path: missing.lua\n",
    );
    write_project_file(
        temp.path(),
        "@Basic_S.tra2",
        concat!(
            "@コマ落ち反復\n",
            "--track@vx:X速度,-10,10,0\n",
            "--param:周期の単位/select/秒=0/フレーム=1/Hz=2,0\n",
            "--param:aaa::周期,0.5\n",
            "--param:周期,0.5\n",
            "--param:周期,1.0\n",
            "--param:周期(分母),1\n",
            "--param:周期ずれ%,0\n",
            "--param:空初期値,\n",
            "--param:有効/check,0\n",
            "--param:デューティ比%,50\n",
        ),
    );

    let output = run_aulua(
        temp.path(),
        &[
            "language",
            "update",
            "--script",
            "@Basic_S.tra2",
            "--output",
            "English.aul2",
        ],
    );

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fs::read_to_string(temp.path().join("English.aul2")).unwrap(),
        concat!(
            "[コマ落ち反復@Basic_S]\n",
            "コマ落ち反復@Basic_S=\n",
            "周期の単位=\n",
            "秒=\n",
            "フレーム=\n",
            "Hz=\n",
            "aaa::周期=\n",
            "周期=\n",
            "周期(分母)=\n",
            "周期ずれ%=\n",
            "有効=\n",
            "デューティ比%=\n",
        )
    );
}
