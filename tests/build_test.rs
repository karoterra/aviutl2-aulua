use aulua::build::build_all;
use aulua::config_loader::load_config;
use std::fs;
use std::path::{Path, PathBuf};

use rstest::rstest;
use tempfile::TempDir;

mod common;

fn collect_relative_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let rel = path.strip_prefix(base).unwrap().to_path_buf();
            if entry.file_type()?.is_dir() {
                walk(&path, base, out)?;
            } else {
                out.push(rel);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    walk(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn assert_dirs_equal(out_dir: &Path, expected_dir: &Path) {
    let out_files = collect_relative_files(out_dir).unwrap();
    let expected_files = collect_relative_files(expected_dir).unwrap();

    assert_eq!(
        out_files, expected_files,
        "file lists differ between out_dir and expected_dir"
    );

    for file in expected_files {
        let out_path = out_dir.join(&file);
        let expected_path = expected_dir.join(&file);
        let out_bytes = fs::read_to_string(&out_path).unwrap();
        let expected_bytes = fs::read_to_string(&expected_path).unwrap();
        assert_eq!(
            out_bytes,
            expected_bytes,
            "contents differ for file: {}",
            file.display()
        );
    }
}

#[rstest]
#[case::basic("basic")]
#[case::embed("embed")]
#[case::pipeline("pipeline")]
#[case::language_directives("language_directives")]
fn test_build(#[case] case_name: &str) {
    let base_dir = common::get_fixture_path("build_tests").join(case_name);
    let config_path = base_dir.join("aulua.yaml");
    let config = load_config(&config_path).unwrap();
    let out_dir = base_dir.join("build");
    let expected_dir = base_dir.join("expected");

    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();

    build_all(&config, &out_dir).unwrap();

    assert_dirs_equal(&out_dir, &expected_dir);
}

#[test]
fn malformed_language_directive_fails_before_writing_the_script() {
    let temp = TempDir::new().unwrap();
    let config_path = temp.path().join("aulua.yaml");
    fs::write(
        &config_path,
        "scripts:\n  - name: malformed.obj2\n    sources:\n      - path: malformed.lua\n",
    )
    .unwrap();
    fs::write(temp.path().join("malformed.lua"), "---$nolang\n").unwrap();
    let config = load_config(&config_path).unwrap();
    let out_dir = temp.path().join("build");

    let error = build_all(&config, &out_dir).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("languageディレクティブの処理に失敗しました")
    );
    assert!(!out_dir.join("malformed.obj2").exists());
}
