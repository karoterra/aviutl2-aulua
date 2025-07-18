use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// 全体の設定ファイル構造
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Config {
    pub project: Option<Project>,
    pub build: Option<Build>,
    pub install: Option<Install>,
    pub scripts: Vec<Script>,
}

impl Config {
    pub fn build_out_dir(&self) -> PathBuf {
        self.build
            .as_ref()
            .and_then(|b| b.out_dir.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("build"))
    }

    pub fn install_out_dir(&self) -> PathBuf {
        self.install
            .as_ref()
            .and_then(|i| i.out_dir.as_ref())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let program_data =
                    std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".to_string());
                PathBuf::from(program_data).join("aviutl2").join("Script")
            })
    }
}

/// `project` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Project {
    pub variables: Option<HashMap<String, String>>,
}

/// `build` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Build {
    pub out_dir: Option<String>,
}

/// `install` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Install {
    pub out_dir: Option<String>,
}

/// 各出力スクリプト単位の設定
#[derive(Debug, Deserialize, JsonSchema)]
pub struct Script {
    pub name: String,
    pub sources: Vec<ScriptSource>,
}

/// 各スクリプトソースの設定
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ScriptSource {
    pub path: String,
    pub label: Option<String>,
    pub variables: Option<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml_ng;

    #[test]
    fn test_config_deserialize_minimal() {
        let yaml = r#"
        project:
          variables:
            VERSION: "v1.0.0"
            AUTHOR: "karoterra"
        scripts: []
        "#;

        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();

        assert!(config.project.is_some());
        let vars = config.project.unwrap().variables.unwrap();
        assert_eq!(vars.get("VERSION"), Some(&"v1.0.0".to_string()));
    }

    #[test]
    fn test_script_with_sources() {
        let yaml = r#"
        scripts:
          - name: test_script
            sources:
              - path: script/foo.obj2
                variables:
                  INFO: "説明"
        "#;

        let config: Config = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.scripts.len(), 1);
        assert_eq!(config.scripts[0].sources[0].path, "script/foo.obj2");
    }
}
