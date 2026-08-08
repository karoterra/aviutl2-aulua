use std::{fs, path::Path};

use aulua::config_loader::{ConfigError, load_config};

#[test]
fn test_load_valid_config() {
    let path = Path::new("tests/fixtures/valid_config.yaml");
    let config = load_config(path).expect("設定ファイルの読み込みに失敗");
    assert_eq!(config.scripts.len(), 1);
}

#[test]
fn test_invalid_config_should_fail() {
    let path = Path::new("tests/fixtures/invalid_config.yaml");
    let result = load_config(path);

    assert!(result.is_err());

    match result {
        Err(ConfigError::Parse(_)) => {
            // パースエラーが期待値
        }
        Err(e) => panic!("予期しないエラー種別: {e}"),
        Ok(_) => panic!("エラーが発生すべき入力で成功してしまった"),
    }
}

#[test]
fn test_load_language_config() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("aulua.yaml");
    fs::write(
        &config_path,
        r#"
scripts: []
language:
  files:
    - path: language/English.example.aul2
    - path: language/Default.example.aul2
      text: false
      tooltip: true
"#,
    )
    .unwrap();

    let config = load_config(&config_path).expect("設定ファイルの読み込みに失敗");
    let language = config
        .language
        .expect("language セクションが解決されていない");

    assert_eq!(language.files.len(), 2);
    assert_eq!(
        language.files[0].path,
        dir.path().join("language/English.example.aul2")
    );
    assert!(language.files[0].text);
    assert!(language.files[0].tooltip);
    assert_eq!(
        language.files[1].path,
        dir.path().join("language/Default.example.aul2")
    );
    assert!(!language.files[1].text);
    assert!(language.files[1].tooltip);
}

#[test]
fn test_language_file_text_and_tooltip_both_false_should_fail() {
    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("aulua.yaml");
    fs::write(
        &config_path,
        r#"
scripts: []
language:
  files:
    - path: language/Disabled.example.aul2
      text: false
      tooltip: false
"#,
    )
    .unwrap();

    let result = load_config(&config_path);

    match result {
        Err(ConfigError::Resolve(error)) => assert_eq!(
            error.to_string(),
            "language.files[0] の text と tooltip を両方 false にすることはできません。"
        ),
        Err(error) => panic!("予期しないエラー種別: {error}"),
        Ok(_) => panic!("エラーが発生すべき入力で成功してしまった"),
    }
}
