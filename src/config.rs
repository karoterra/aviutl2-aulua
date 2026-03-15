use schemars::JsonSchema;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 全体の設定ファイル構造
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawConfig {
    pub project: Option<RawProject>,
    pub build: Option<RawBuild>,
    pub install: Option<RawInstall>,
    pub scripts: Vec<RawScript>,
}

impl RawConfig {
    pub fn resolve(self, config_path: &Path) -> ResolvedConfig {
        let config_dir = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let raw_project = self.project.unwrap_or(RawProject { variables: None });
        let project = ResolvedProject {
            variables: raw_project.variables.unwrap_or_default(),
        };

        let raw_build = self.build.unwrap_or(RawBuild {
            out_dir: None,
            embed_search_dirs: None,
        });
        let build = ResolvedBuild {
            out_dir: raw_build
                .out_dir
                .unwrap_or_else(|| "build".to_string())
                .into(),
            embed_search_dirs: raw_build
                .embed_search_dirs
                .unwrap_or_default()
                .into_iter()
                .map(|d| config_dir.join(d))
                .collect(),
        };

        let raw_install = self.install.unwrap_or(RawInstall { out_dir: None });
        let install = ResolvedInstall {
            out_dir: raw_install.out_dir.map(PathBuf::from).unwrap_or_else(|| {
                let program_data =
                    std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".to_string());
                PathBuf::from(program_data).join("aviutl2").join("Script")
            }),
        };

        let scripts = self
            .scripts
            .into_iter()
            .map(|s| ResolvedScript {
                name: s.name,
                sources: s
                    .sources
                    .into_iter()
                    .map(|src| ResolvedScriptSource {
                        path: src.path.into(),
                        label: src.label,
                        variables: src.variables.unwrap_or_default(),
                    })
                    .collect(),
            })
            .collect();

        ResolvedConfig {
            project,
            build,
            install,
            scripts,
            config_dir,
        }
    }
}

/// `project` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawProject {
    pub variables: Option<HashMap<String, String>>,
}

/// `build` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawBuild {
    pub out_dir: Option<String>,
    pub embed_search_dirs: Option<Vec<String>>,
}

/// `install` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawInstall {
    pub out_dir: Option<String>,
}

/// 各出力スクリプト単位の設定
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawScript {
    pub name: String,
    pub sources: Vec<RawScriptSource>,
}

/// 各スクリプトソースの設定
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawScriptSource {
    pub path: String,
    pub label: Option<String>,
    pub variables: Option<HashMap<String, String>>,
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub project: ResolvedProject,
    pub build: ResolvedBuild,
    pub install: ResolvedInstall,
    pub scripts: Vec<ResolvedScript>,
    pub config_dir: PathBuf,
}

impl ResolvedConfig {
    pub fn build_out_dir(&self) -> PathBuf {
        self.build.out_dir.clone()
    }

    pub fn install_out_dir(&self) -> PathBuf {
        self.install.out_dir.clone()
    }
}

#[derive(Debug)]
pub struct ResolvedProject {
    pub variables: HashMap<String, String>,
}

#[derive(Debug)]
pub struct ResolvedBuild {
    pub out_dir: PathBuf,
    pub embed_search_dirs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct ResolvedInstall {
    pub out_dir: PathBuf,
}

#[derive(Debug)]
pub struct ResolvedScript {
    pub name: String,
    pub sources: Vec<ResolvedScriptSource>,
}

#[derive(Debug)]
pub struct ResolvedScriptSource {
    pub path: PathBuf,
    pub label: Option<String>,
    pub variables: HashMap<String, String>,
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

        let config: RawConfig = serde_yaml_ng::from_str(yaml).unwrap();

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

        let config: RawConfig = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(config.scripts.len(), 1);
        assert_eq!(config.scripts[0].sources[0].path, "script/foo.obj2");
    }

    #[test]
    fn test_resolve_embed_search_dirs() {
        let raw = RawConfig {
            project: None,
            build: Some(RawBuild {
                out_dir: None,
                embed_search_dirs: Some(vec!["lib".to_string(), "modules".to_string()]),
            }),
            install: None,
            scripts: vec![],
        };

        let config_path = PathBuf::from("/tmp/project/aulua.yaml");
        let resolved = raw.resolve(&config_path);

        assert_eq!(
            resolved.build.embed_search_dirs,
            vec![
                PathBuf::from("/tmp/project/lib"),
                PathBuf::from("/tmp/project/modules")
            ]
        );
    }
}
