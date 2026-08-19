use std::fs;
use std::io::Read;
use zip::ZipArchive;

use aulua::config_loader::load_config;
use aulua::pack::pack_project;

fn write_language_pack_config(root: &std::path::Path, files: &str, assets: &str) {
    let assets = if assets.is_empty() {
        "  assets: []\n".to_string()
    } else {
        format!("  assets:\n{assets}")
    };
    fs::write(
        root.join("aulua.yaml"),
        format!(
            "package:\n  id: karoterra.language-test\n  name: Language Test\n  information: Language package test\n  out_dir: dist\n{assets}language:\n  files:\n{files}scripts: []\n"
        ),
    )
    .unwrap();
}

fn read_zip_entry(zip: &mut ZipArchive<fs::File>, name: &str) -> Vec<u8> {
    let mut content = Vec::new();
    zip.by_name(name)
        .unwrap()
        .read_to_end(&mut content)
        .unwrap();
    content
}

#[test]
fn pack_project_creates_expected_archive_contents() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();

    fs::write(
        root.join("src").join("main.in.anm2"),
        "-- version: ${PACKAGE_VERSION}\nprint('hello')\n",
    )
    .unwrap();

    fs::write(root.join("docs").join("README.md"), "# README\n").unwrap();

    fs::write(root.join("package-message.txt"), "line1\nline2\n").unwrap();

    fs::write(
        root.join("aulua.yaml"),
        r#"
project:
  variables: {}

build:
  out_dir: build

package:
  id: karoterra.example
  name: Example
  information: Example package
  version: 1.2.3
  out_dir: dist
  file_name: "{id}-v{version}.au2pkg.zip"
  script_sub_dir: "{id}"
  message:
    file: package-message.txt
  assets:
    - src: docs/README.md
      dest: Script/{id}/docs/README.md

scripts:
  - name: main.anm2
    sources:
      - path: src/main.in.anm2
"#,
    )
    .unwrap();

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let archive_path = pack_project(&config).unwrap();

    assert_eq!(
        archive_path.file_name().unwrap().to_string_lossy(),
        "karoterra.example-v1.2.3.au2pkg.zip"
    );

    let file = fs::File::open(&archive_path).unwrap();
    let mut zip = ZipArchive::new(file).unwrap();

    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();

    assert!(names.contains(&"package.ini".to_string()));
    assert!(names.contains(&"package.txt".to_string()));
    assert!(names.contains(&"Script/karoterra.example/main.anm2".to_string()));
    assert!(names.contains(&"Script/karoterra.example/docs/README.md".to_string()));

    let mut package_ini = String::new();
    zip.by_name("package.ini")
        .unwrap()
        .read_to_string(&mut package_ini)
        .unwrap();
    assert!(package_ini.contains("[package]\r\n"));
    assert!(package_ini.contains("id=karoterra.example\r\n"));
    assert!(package_ini.contains("name=Example\r\n"));
    assert!(package_ini.contains("information=Example package\r\n"));

    let mut package_txt = Vec::new();
    zip.by_name("package.txt")
        .unwrap()
        .read_to_end(&mut package_txt)
        .unwrap();
    assert_eq!(package_txt, b"line1\r\nline2\r\n");

    let mut script = String::new();
    zip.by_name("Script/karoterra.example/main.anm2")
        .unwrap()
        .read_to_string(&mut script)
        .unwrap();
    assert!(script.contains("-- version: 1.2.3"));

    let mut readme = String::new();
    zip.by_name("Script/karoterra.example/docs/README.md")
        .unwrap()
        .read_to_string(&mut readme)
        .unwrap();
    assert_eq!(readme, "# README\n");
}

#[test]
fn pack_project_includes_configured_language_files_without_changing_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let english_path = root.join("translations/English.aul2");
    let japanese_path = root.join("other/Japanese.aul2");
    let english = b"\xef\xbb\xbf[effect]\r\nText=translated\r\n";
    let japanese = "[effect]\nText=翻訳\n".as_bytes();

    fs::create_dir_all(english_path.parent().unwrap()).unwrap();
    fs::create_dir_all(japanese_path.parent().unwrap()).unwrap();
    fs::write(&english_path, english).unwrap();
    fs::write(&japanese_path, japanese).unwrap();
    write_language_pack_config(
        root,
        "    - path: translations/English.aul2\n      text: false\n      tooltip: true\n    - path: other/Japanese.aul2\n      text: true\n      tooltip: false\n",
        "",
    );

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let archive_path = pack_project(&config).unwrap();
    let mut zip = ZipArchive::new(fs::File::open(archive_path).unwrap()).unwrap();

    assert_eq!(read_zip_entry(&mut zip, "Language/English.aul2"), english);
    assert_eq!(read_zip_entry(&mut zip, "Language/Japanese.aul2"), japanese);
    assert!(zip.by_name("Language/translations/English.aul2").is_err());
    assert!(zip.by_name("Language/other/Japanese.aul2").is_err());
    assert_eq!(fs::read(&english_path).unwrap(), english);
    assert_eq!(fs::read(&japanese_path).unwrap(), japanese);
}

#[test]
fn pack_project_errors_when_configured_language_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let missing_path = root.join("translations/Missing.aul2");
    write_language_pack_config(root, "    - path: translations/Missing.aul2\n", "");

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let error = pack_project(&config).unwrap_err().to_string();

    assert!(error.contains("入力ファイルを開けませんでした"));
    assert!(error.contains(&missing_path.display().to_string()));
    assert!(!missing_path.exists());
}

#[test]
fn pack_project_rejects_duplicate_language_file_names() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("a")).unwrap();
    fs::create_dir_all(root.join("b")).unwrap();
    fs::write(root.join("a/English.aul2"), "[a]\nkey=value\n").unwrap();
    fs::write(root.join("b/English.aul2"), "[b]\nkey=value\n").unwrap();
    write_language_pack_config(
        root,
        "    - path: a/English.aul2\n    - path: b/English.aul2\n",
        "",
    );

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let error = pack_project(&config).unwrap_err().to_string();

    assert!(error.contains("zip 内に同じパスが複数回追加されています"));
    assert!(error.contains("Language/English.aul2"));
}

#[test]
fn pack_project_rejects_language_file_and_asset_collision() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("translations")).unwrap();
    fs::create_dir_all(root.join("assets")).unwrap();
    fs::write(
        root.join("translations/English.aul2"),
        "[effect]\nkey=value\n",
    )
    .unwrap();
    fs::write(root.join("assets/English.aul2"), "asset").unwrap();
    write_language_pack_config(
        root,
        "    - path: translations/English.aul2\n",
        "    - src: assets/English.aul2\n      dest: Language/English.aul2\n",
    );

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let error = pack_project(&config).unwrap_err().to_string();

    assert!(error.contains("zip 内に同じパスが複数回追加されています"));
    assert!(error.contains("Language/English.aul2"));
}

#[test]
fn pack_project_rejects_language_file_with_invalid_extension() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("translations")).unwrap();
    fs::write(root.join("translations/English.txt"), "not a language file").unwrap();
    write_language_pack_config(root, "    - path: translations/English.txt\n", "");

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let error = pack_project(&config).unwrap_err().to_string();

    assert!(error.contains("拡張子が.aul2ではありません"));
    assert!(error.contains("English.txt"));
}

#[test]
fn pack_project_includes_global_and_script_specific_language_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("translations")).unwrap();
    fs::write(root.join("script.lua"), "").unwrap();
    fs::write(
        root.join("translations/Global.aul2"),
        "[global]\nkey=global\n",
    )
    .unwrap();
    fs::write(
        root.join("translations/Scoped.aul2"),
        "[scoped]\nkey=scoped\n",
    )
    .unwrap();
    fs::write(
        root.join("aulua.yaml"),
        r#"package:
  id: karoterra.scoped-language-test
  name: Scoped Language Test
  information: Scoped language package test
  out_dir: dist
  assets: []
language:
  files:
    - path: translations/Global.aul2
scripts:
  - name: script.anm2
    sources:
      - path: script.lua
    language:
      files:
        - path: translations/Scoped.aul2
"#,
    )
    .unwrap();

    let config = load_config(root.join("aulua.yaml")).unwrap();
    let archive_path = pack_project(&config).unwrap();
    let mut zip = ZipArchive::new(fs::File::open(archive_path).unwrap()).unwrap();

    assert_eq!(
        read_zip_entry(&mut zip, "Language/Global.aul2"),
        b"[global]\nkey=global\n"
    );
    assert_eq!(
        read_zip_entry(&mut zip, "Language/Scoped.aul2"),
        b"[scoped]\nkey=scoped\n"
    );
}
