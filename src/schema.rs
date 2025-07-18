use std::fs;
use std::path::Path;

use anyhow::anyhow;
use schemars::schema_for;

use crate::config::Config;

pub fn generate_config_schema(path: &Path) -> anyhow::Result<()> {
    let schema = schema_for!(Config);
    let schema_json = serde_json::to_string_pretty(&schema)
        .map_err(|e| anyhow!("スキーマの生成に失敗しました: {e}"))?;
    fs::write(path, schema_json)
        .map_err(|e| anyhow!("スキーマファイルの書き込みに失敗しました: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_config_schema() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        generate_config_schema(path).expect("スキーマの生成に失敗しました");
    }
}
