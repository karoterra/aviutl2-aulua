use std::path::Path;

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
