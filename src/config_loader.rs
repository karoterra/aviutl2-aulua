use std::fs;
use std::path::Path;

use crate::config::Config;
use serde_yaml_ng as yaml;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("設定ファイルの読み込みに失敗しました: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAMLのパースに失敗しました: {0}")]
    Parse(#[from] yaml::Error),
}

/// `aulua.yaml` を読み込んで `Config` に変換
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
    let content = fs::read_to_string(path)?;
    let config: Config = yaml::from_str(&content)?;
    Ok(config)
}
