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
    pub package: Option<RawPackage>,
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
                .map(|d| config_dir.join(d))
                .unwrap_or_else(|| config_dir.join("build")),
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

        let raw_package = self.package;
        let package = raw_package.map(|p| ResolvedPackage {
            id: p.id,
            name: p.name,
            information: p.information,
            version: p.version,
            uninstall_sub_folder_file: p.uninstall_sub_folder_file.unwrap_or(false),
            out_dir: p
                .out_dir
                .map(|d| config_dir.join(d))
                .unwrap_or_else(|| build.out_dir.clone()),
            file_name: p.file_name,
            script_sub_dir: p.script_sub_dir,
            message: p.message.map(|m| match m {
                RawPackageMessage::File { file } => {
                    ResolvedPackageMessage::File(config_dir.join(file))
                }
                RawPackageMessage::Text { text } => ResolvedPackageMessage::Text(text),
            }),
            assets: p
                .assets
                .into_iter()
                .map(|a| ResolvedPackageAsset {
                    src: config_dir.join(a.src),
                    dest: a.dest,
                })
                .collect(),
        });

        let scripts = self
            .scripts
            .into_iter()
            .map(|s| ResolvedScript {
                name: s.name,
                sources: s
                    .sources
                    .into_iter()
                    .map(|src| ResolvedScriptSource {
                        path: config_dir.join(src.path),
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
            package,
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

/// `package` セクション
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawPackage {
    pub id: Option<String>,
    pub name: Option<String>,
    pub information: Option<String>,
    pub version: Option<String>,
    pub uninstall_sub_folder_file: Option<bool>,
    pub out_dir: Option<String>,
    pub file_name: Option<String>,
    pub script_sub_dir: Option<String>,
    pub message: Option<RawPackageMessage>,

    #[serde(default)]
    pub assets: Vec<RawPackageAsset>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RawPackageMessage {
    File { file: PathBuf },
    Text { text: String },
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RawPackageAsset {
    pub src: PathBuf,
    pub dest: String,
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
    pub package: Option<ResolvedPackage>,
    pub scripts: Vec<ResolvedScript>,
    pub config_dir: PathBuf,
}

fn build_package_template_vars(
    project_vars: &HashMap<String, String>,
    id: &str,
    name: &str,
    version: Option<&str>,
) -> HashMap<String, String> {
    let mut vars = project_vars.clone();

    vars.insert("id".to_string(), id.to_string());
    vars.insert("name".to_string(), name.to_string());

    if let Some(version) = version {
        vars.insert("version".to_string(), version.to_string());
    }

    vars
}

fn render_package_template(input: &str, vars: &HashMap<String, String>) -> anyhow::Result<String> {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '{' {
            result.push(ch);
            continue;
        }

        let mut key = String::new();
        let mut closed = false;

        while let Some(&next) = chars.peek() {
            chars.next();

            if next == '}' {
                closed = true;
                break;
            }

            key.push(next);
        }

        if !closed {
            return Err(anyhow::anyhow!(
                "packageテンプレートの構文エラー: '{{' に対応する '}}' がありません: {}",
                input
            ));
        }

        if key.is_empty() {
            return Err(anyhow::anyhow!(
                "packageテンプレートの構文エラー: 空のプレースホルダー {{}} は使用できません: {}",
                input
            ));
        }

        let value = vars.get(&key).ok_or_else(|| {
            anyhow::anyhow!(
                "packageテンプレート内で未定義の変数 {{{}}} が使われています: {}",
                key,
                input
            )
        })?;

        result.push_str(value);
    }

    Ok(result)
}

impl ResolvedConfig {
    pub fn package_for_pack(&self) -> anyhow::Result<PackConfig> {
        let p = self
            .package
            .as_ref()
            .ok_or(anyhow::anyhow!("packageセクションが定義されていません。"))?;

        let id =
            p.id.clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or(anyhow::anyhow!("package.idが定義されていません。"))?;
        let name = p
            .name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or(anyhow::anyhow!("package.nameが定義されていません。"))?;
        let information_template = p
            .information
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or(anyhow::anyhow!("package.informationが定義されていません。"))?;

        let vars =
            build_package_template_vars(&self.project.variables, &id, &name, p.version.as_deref());

        let file_name_template = p.file_name.clone().unwrap_or_else(|| {
            if p.version.is_some() {
                "{id}-v{version}.au2pkg.zip".to_string()
            } else {
                "{id}.au2pkg.zip".to_string()
            }
        });
        let script_sub_dir_template = p
            .script_sub_dir
            .clone()
            .unwrap_or_else(|| "{id}".to_string());

        let information = render_package_template(&information_template, &vars)?;
        let file_name = render_package_template(&file_name_template, &vars)?;
        let script_sub_dir = render_package_template(&script_sub_dir_template, &vars)?;

        if information.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "package.information の解決結果が空文字列になりました。"
            ));
        }
        if file_name.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "package.file_name の解決結果が空文字列になりました。"
            ));
        }
        if script_sub_dir.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "package.script_sub_dir の解決結果が空文字列になりました。"
            ));
        }

        let assets = p
            .assets
            .iter()
            .map(|asset| {
                Ok(ResolvedPackageAsset {
                    src: asset.src.clone(),
                    dest: render_package_template(&asset.dest, &vars)?,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(PackConfig {
            id,
            name,
            information,
            version: p.version.clone(),
            uninstall_sub_folder_file: p.uninstall_sub_folder_file,
            out_dir: p.out_dir.clone(),
            file_name,
            script_sub_dir,
            message: p.message.clone(),
            assets,
        })
    }
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
pub struct ResolvedPackage {
    pub id: Option<String>,
    pub name: Option<String>,
    pub information: Option<String>,
    pub version: Option<String>,
    pub uninstall_sub_folder_file: bool,
    pub out_dir: PathBuf,
    pub file_name: Option<String>,
    pub script_sub_dir: Option<String>,
    pub message: Option<ResolvedPackageMessage>,
    pub assets: Vec<ResolvedPackageAsset>,
}

#[derive(Debug, Clone)]
pub enum ResolvedPackageMessage {
    File(PathBuf),
    Text(String),
}

#[derive(Debug, Clone)]
pub struct ResolvedPackageAsset {
    pub src: PathBuf,
    pub dest: String,
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

#[derive(Debug, Clone)]
pub struct PackConfig {
    pub id: String,
    pub name: String,
    pub information: String,
    pub version: Option<String>,
    pub uninstall_sub_folder_file: bool,
    pub out_dir: PathBuf,
    pub file_name: String,
    pub script_sub_dir: String,
    pub message: Option<ResolvedPackageMessage>,
    pub assets: Vec<ResolvedPackageAsset>,
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
            package: None,
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

    #[test]
    fn test_render_package_template() {
        let input = "{id}-v{version}.au2pkg.zip";
        let vars = HashMap::from([
            ("id".to_string(), "SamplePackage".to_string()),
            ("version".to_string(), "1.0.0".to_string()),
        ]);
        let expected = "SamplePackage-v1.0.0.au2pkg.zip";

        let output = render_package_template(input, &vars).unwrap();
        assert_eq!(output, expected);
    }
}
