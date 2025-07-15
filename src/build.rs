use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::config::{Config, Script};

pub fn build_all(config: &Config, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir)?;

    for script in &config.scripts {
        build_script(script, config, out_dir)?;
    }

    Ok(())
}

fn build_script(script: &Script, config: &Config, out_dir: &Path) -> anyhow::Result<()> {
    let mut combined = String::new();

    for source in &script.sources {
        let src_path = PathBuf::from(&source.path);
        let content = fs::read_to_string(&src_path).map_err(|e| {
            anyhow::anyhow!("{} の読み込みに失敗しました: {}", src_path.display(), e)
        })?;

        // ラベル
        if let Some(label) = &source.label {
            combined.push_str(&format!("@{}\n", label));
        }

        // 変数
        let mut vars = HashMap::new();
        if let Some(global) = config.project.as_ref().and_then(|p| p.variables.as_ref()) {
            vars.extend(global.clone());
        }
        if let Some(local) = &source.variables {
            vars.extend(local.clone());
        }
        let (content, warnings) = apply_variables(&content, &vars);

        for warning in warnings {
            eprintln!(
                "⚠️ 未定義の変数: ${} （{} 内）",
                warning,
                src_path.display()
            );
        }

        combined.push_str(&content);
        combined.push_str("\n");
    }

    let out_path = out_dir.join(&script.name);
    fs::write(&out_path, combined)?;
    println!("✅ ビルド完了: {}", out_path.display());

    Ok(())
}

fn apply_variables(text: &str, vars: &HashMap<String, String>) -> (String, Vec<String>) {
    let re = Regex::new(r"\$\{([A-Za-z0-9_]+)\}").unwrap();
    let mut undefined_vars = HashSet::new();

    let result = re.replace_all(text, |caps: &regex::Captures| {
        let key = &caps[1];
        match vars.get(key) {
            Some(val) => val.to_string(),
            None => {
                undefined_vars.insert(key.to_string());
                caps[0].to_string()
            }
        }
    });

    (result.into_owned(), undefined_vars.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_apply_variables_single() {
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "karoterra".to_string());

        let input = "Hello, ${NAME}!";
        let expected = "Hello, karoterra!";

        let (actual, warnings) = apply_variables(input, &vars);
        assert_eq!(actual, expected);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_apply_variables_multiple() {
        let mut vars = HashMap::new();
        vars.insert("FOO".to_string(), "foo".to_string());
        vars.insert("BAR".to_string(), "bar".to_string());

        let input = "This is ${FOO} and that is ${BAR}.";
        let expected = "This is foo and that is bar.";

        let (actual, warnings) = apply_variables(input, &vars);
        assert_eq!(actual, expected);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_apply_variables_missing() {
        let vars = HashMap::new();

        let input = "Unresolved: ${UNKNOWN}";
        let expected = "Unresolved: ${UNKNOWN}";

        let (actual, warnings) = apply_variables(input, &vars);
        assert_eq!(actual, expected);
        assert_eq!(warnings, vec!["UNKNOWN"]);
    }

    #[test]
    fn test_apply_variables_partial_overlap() {
        let mut vars = HashMap::new();
        vars.insert("VERSION".to_string(), "1.2.3".to_string());
        vars.insert("VER".to_string(), "WRONG".to_string());

        let input = "App version: ${VERSION}";
        let expected = "App version: 1.2.3";

        let (actual, warnings) = apply_variables(input, &vars);
        assert_eq!(actual, expected);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_apply_variables_case_sensitive() {
        let mut vars = HashMap::new();
        vars.insert("FOO".to_string(), "X".to_string());

        let input = "Test: ${foo}";
        let expected = "Test: ${foo}";

        let (actual, warnings) = apply_variables(input, &vars);
        assert_eq!(actual, expected);
        assert_eq!(warnings, vec!["foo"]);
    }
}
