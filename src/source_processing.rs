use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use regex::Regex;
use thiserror::Error;

use crate::config::{ResolvedConfig, ResolvedScriptSource};
use crate::include::process_includes;
use crate::text_utils::read_text;

const RESERVED_PACKAGE_VARIABLES: &[&str] = &["PACKAGE_ID", "PACKAGE_NAME", "PACKAGE_VERSION"];

#[derive(Debug, Error)]
pub(crate) enum SourceProcessingError {
    #[error("{} の読み込みに失敗しました: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{message}")]
    Include { message: String },
    #[error("{scope} に予約変数 {name} を定義することはできません。")]
    ReservedVariable {
        scope: &'static str,
        name: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariableExpansion {
    pub text: String,
    pub undefined_variables: Vec<String>,
}

pub(crate) fn load_source_text(source_path: &Path) -> Result<String, SourceProcessingError> {
    read_text(source_path).map_err(|source| SourceProcessingError::Read {
        path: source_path.to_path_buf(),
        source,
    })
}

pub(crate) fn expand_source_includes(
    source_path: &Path,
    content: &str,
) -> Result<String, SourceProcessingError> {
    let mut stack = Vec::new();
    process_includes(
        content,
        source_path.parent().unwrap_or(Path::new("")),
        &mut stack,
    )
    .map_err(|message| SourceProcessingError::Include { message })
}

pub(crate) fn build_source_variables(
    config: &ResolvedConfig,
    source: &ResolvedScriptSource,
) -> Result<HashMap<String, String>, SourceProcessingError> {
    ensure_no_reserved_package_variables(&config.project.variables, "project.variables")?;
    ensure_no_reserved_package_variables(&source.variables, "source.variables")?;

    let mut variables = config.project.variables.clone();

    if let Some(package) = &config.package {
        if let Some(id) = &package.id {
            variables.insert("PACKAGE_ID".to_string(), id.clone());
        }
        if let Some(name) = &package.name {
            variables.insert("PACKAGE_NAME".to_string(), name.clone());
        }
        if let Some(version) = &package.version {
            variables.insert("PACKAGE_VERSION".to_string(), version.clone());
        }
    }

    variables.extend(source.variables.clone());

    Ok(variables)
}

pub(crate) fn expand_variables(
    text: &str,
    variables: &HashMap<String, String>,
) -> VariableExpansion {
    let regex = Regex::new(r"\$\{([A-Za-z0-9_]+)\}").unwrap();
    let mut undefined_variables = HashSet::new();

    let text = regex.replace_all(text, |captures: &regex::Captures| {
        let name = &captures[1];
        match variables.get(name) {
            Some(value) => value.to_string(),
            None => {
                undefined_variables.insert(name.to_string());
                captures[0].to_string()
            }
        }
    });

    VariableExpansion {
        text: text.into_owned(),
        undefined_variables: undefined_variables.into_iter().collect(),
    }
}

fn ensure_no_reserved_package_variables(
    variables: &HashMap<String, String>,
    scope: &'static str,
) -> Result<(), SourceProcessingError> {
    for &name in RESERVED_PACKAGE_VARIABLES {
        if variables.contains_key(name) {
            return Err(SourceProcessingError::ReservedVariable { scope, name });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::config::{ResolvedBuild, ResolvedInstall, ResolvedPackage, ResolvedProject};

    fn source(path: PathBuf, variables: HashMap<String, String>) -> ResolvedScriptSource {
        ResolvedScriptSource {
            path,
            label: None,
            variables,
        }
    }

    fn config(
        project_variables: HashMap<String, String>,
        package: Option<ResolvedPackage>,
    ) -> ResolvedConfig {
        ResolvedConfig {
            project: ResolvedProject {
                variables: project_variables,
            },
            build: ResolvedBuild {
                out_dir: PathBuf::new(),
                embed_search_dirs: Vec::new(),
            },
            install: ResolvedInstall {
                out_dir: PathBuf::new(),
            },
            package,
            language: None,
            scripts: Vec::new(),
            config_dir: PathBuf::new(),
        }
    }

    fn package() -> ResolvedPackage {
        ResolvedPackage {
            id: Some("package-id".to_string()),
            name: Some("Package Name".to_string()),
            information: None,
            version: Some("1.2.3".to_string()),
            uninstall_sub_folder_file: false,
            out_dir: PathBuf::new(),
            file_name: None,
            script_sub_dir: None,
            message: None,
            assets: Vec::new(),
        }
    }

    #[test]
    fn load_source_text_normalizes_newlines_and_preserves_bom() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("source.lua");
        fs::write(&path, "\u{feff}first\r\nsecond\rthird").unwrap();

        assert_eq!(
            load_source_text(&path).unwrap(),
            "\u{feff}first\nsecond\nthird"
        );
    }

    #[test]
    fn load_source_text_keeps_path_and_io_error() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("missing.lua");

        let error = load_source_text(&path).unwrap_err();

        assert!(matches!(
            error,
            SourceProcessingError::Read {
                path: error_path,
                source,
            } if error_path == path && source.kind() == std::io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn include_entry_uses_each_files_parent_and_normalizes_included_newlines() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("nested");
        fs::create_dir(&nested).unwrap();
        let source_path = temp.path().join("source.lua");
        fs::write(
            &source_path,
            "root\r\n---$include \"nested/first.lua\"\r\n---$include \"shared.lua\"\r\n---$include \"shared.lua\"",
        )
        .unwrap();
        fs::write(
            nested.join("first.lua"),
            "first\r---$include \"../shared.lua\"",
        )
        .unwrap();
        fs::write(temp.path().join("shared.lua"), "shared\r\n").unwrap();

        let content = load_source_text(&source_path).unwrap();
        let expanded = expand_source_includes(&source_path, &content).unwrap();

        assert!(!expanded.contains("---$include"));
        assert!(!expanded.contains('\r'));
        assert_eq!(expanded.matches("shared").count(), 3);
        assert!(expanded.contains("first\nshared"));
    }

    #[test]
    fn include_cycle_is_wrapped() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("source.lua");
        fs::write(&source_path, "---$include \"source.lua\"").unwrap();

        let content = load_source_text(&source_path).unwrap();
        let error = expand_source_includes(&source_path, &content).unwrap_err();

        assert!(matches!(
            error,
            SourceProcessingError::Include { message }
                if message.starts_with("Circular include detected")
        ));
    }

    #[test]
    fn source_variables_override_project_variables_without_package() {
        let config = config(
            HashMap::from([("NAME".to_string(), "project".to_string())]),
            None,
        );
        let source = source(
            PathBuf::from("source.lua"),
            HashMap::from([("NAME".to_string(), "source".to_string())]),
        );

        let variables = build_source_variables(&config, &source).unwrap();

        assert_eq!(variables.get("NAME").unwrap(), "source");
        assert!(!variables.contains_key("PACKAGE_ID"));
        assert!(!variables.contains_key("PACKAGE_NAME"));
        assert!(!variables.contains_key("PACKAGE_VERSION"));
    }

    #[test]
    fn package_variables_include_id_name_and_version() {
        let config = config(HashMap::new(), Some(package()));
        let source = source(PathBuf::from("source.lua"), HashMap::new());

        let variables = build_source_variables(&config, &source).unwrap();

        assert_eq!(variables.get("PACKAGE_ID").unwrap(), "package-id");
        assert_eq!(variables.get("PACKAGE_NAME").unwrap(), "Package Name");
        assert_eq!(variables.get("PACKAGE_VERSION").unwrap(), "1.2.3");
    }

    #[test]
    fn missing_package_fields_are_not_added() {
        let mut package = package();
        package.id = None;
        package.name = None;
        package.version = None;
        let config = config(HashMap::new(), Some(package));
        let source = source(PathBuf::from("source.lua"), HashMap::new());

        let variables = build_source_variables(&config, &source).unwrap();

        assert!(!variables.contains_key("PACKAGE_ID"));
        assert!(!variables.contains_key("PACKAGE_NAME"));
        assert!(!variables.contains_key("PACKAGE_VERSION"));
    }

    #[test]
    fn reserved_package_variables_are_rejected_in_project_and_source() {
        for (project_variables, source_variables, expected_scope, expected_name) in [
            (
                HashMap::from([("PACKAGE_ID".to_string(), "invalid".to_string())]),
                HashMap::new(),
                "project.variables",
                "PACKAGE_ID",
            ),
            (
                HashMap::new(),
                HashMap::from([("PACKAGE_VERSION".to_string(), "invalid".to_string())]),
                "source.variables",
                "PACKAGE_VERSION",
            ),
        ] {
            let config = config(project_variables, Some(package()));
            let source = source(PathBuf::from("source.lua"), source_variables);

            assert!(matches!(
                build_source_variables(&config, &source),
                Err(SourceProcessingError::ReservedVariable { scope, name })
                    if scope == expected_scope && name == expected_name
            ));
        }
    }

    #[test]
    fn expands_defined_variables_and_preserves_unique_undefined_variables() {
        let variables = HashMap::from([
            ("NAME".to_string(), "aulua".to_string()),
            ("name".to_string(), "lowercase".to_string()),
        ]);

        let expansion = expand_variables(
            "${NAME} ${UNKNOWN} ${UNKNOWN} ${name} ${WITH-DASH} $NAME",
            &variables,
        );

        assert_eq!(
            expansion.text,
            "aulua ${UNKNOWN} ${UNKNOWN} lowercase ${WITH-DASH} $NAME"
        );
        assert_eq!(expansion.undefined_variables, vec!["UNKNOWN"]);
    }

    #[test]
    fn primitives_compose_without_embedding_modules_or_converting_ui() {
        let temp = TempDir::new().unwrap();
        let source_path = temp.path().join("source.lua");
        fs::write(
            &source_path,
            concat!(
                "---$include \"included.lua\"\n",
                "---$embed\n",
                "local module = require(\"module\")\n",
                "---$track:${UI_NAME}\n",
                "local value = 0\n",
            ),
        )
        .unwrap();
        fs::write(temp.path().join("included.lua"), "${INCLUDED}").unwrap();
        fs::write(temp.path().join("module.lua"), "module body").unwrap();
        let project_variables = HashMap::from([
            ("INCLUDED".to_string(), "included body".to_string()),
            ("UI_NAME".to_string(), "Track".to_string()),
        ]);
        let config = config(project_variables, None);
        let source = source(source_path.clone(), HashMap::new());

        let content = load_source_text(&source_path).unwrap();
        let content = expand_source_includes(&source_path, &content).unwrap();
        let variables = build_source_variables(&config, &source).unwrap();
        let expansion = expand_variables(&content, &variables);

        assert!(expansion.text.contains("included body"));
        assert!(expansion.text.contains("---$embed"));
        assert!(expansion.text.contains("require(\"module\")"));
        assert!(!expansion.text.contains("module body"));
        assert!(expansion.text.contains("---$track:Track"));
        assert!(!expansion.text.contains("--track@"));
        assert!(expansion.undefined_variables.is_empty());
    }
}
