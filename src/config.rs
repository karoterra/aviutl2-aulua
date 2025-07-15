use serde::Deserialize;
use std::collections::HashMap;

/// 全体の設定ファイル構造
#[derive(Debug, Deserialize)]
pub struct Config {
    pub project: Option<Project>,
    pub scripts: Vec<Script>,
}

/// `project` セクション
#[derive(Debug, Deserialize)]
pub struct Project {
    pub variables: Option<HashMap<String, String>>,
}

/// 各出力スクリプト単位の設定
#[derive(Debug, Deserialize)]
pub struct Script {
    pub name: String,
    pub sources: Vec<ScriptSource>,
}

/// 各スクリプトソースの設定
#[derive(Debug, Deserialize)]
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
