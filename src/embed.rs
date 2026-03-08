// use std::fs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::text_utils::read_text;

const MODULE_RE: &str = r#"require\s*\(?\s*["']([^"']+)["']\s*\)?"#;

fn get_embed_module_function_name(index: i64) -> String {
    format!("__aulua_embed_{}__()", index)
}

fn find_lua_module(
    module_name: &str,
    base_dir: &Path,
    search_dirs: &[PathBuf],
) -> anyhow::Result<PathBuf> {
    let module_name_formatted = module_name.replace('.', "/");
    let module_path = base_dir.join(format!("{}.lua", module_name_formatted));
    if module_path.exists() {
        return Ok(module_path);
    }

    for search_dir in search_dirs {
        let search_path = base_dir
            .join(search_dir)
            .join(format!("{}.lua", module_name_formatted));
        if search_path.exists() {
            return Ok(search_path);
        }
    }

    Err(anyhow::anyhow!(
        "Module '{}' not found in base directory or search directories",
        module_name
    ))
}

pub fn process_embeds(
    content: &str,
    base_dir: &Path,
    search_dirs: &[PathBuf],
) -> anyhow::Result<String> {
    let mut lines = content.split("\n").peekable();
    let mut new_lines: Vec<String> = Vec::new();
    let mut modules: HashMap<String, i64> = HashMap::new();
    let re = Regex::new(MODULE_RE).unwrap();

    while let Some(line) = lines.next() {
        let trimed = line.trim();
        if trimed.starts_with("---$embed") {
            let next_line = lines.peek().unwrap();
            match re.captures(next_line) {
                Some(caps) => {
                    let module_name = &caps[1];
                    if let Some(index) = modules.get(module_name) {
                        let new_line =
                            next_line.replace(&caps[0], &get_embed_module_function_name(*index));
                        new_lines.push(new_line);
                    } else {
                        let module_path = find_lua_module(module_name, base_dir, search_dirs)?;
                        let module_content = read_text(&module_path).map_err(|e| {
                            anyhow::anyhow!(
                                "Failed to read embedded module {}: {}",
                                module_path.display(),
                                e
                            )
                        })?;
                        let index = modules.len() as i64 + 1;
                        let embed_module_function_name = get_embed_module_function_name(index);
                        modules.insert(module_name.to_string(), index);
                        new_lines.push(format!("-- aulua embed: {}", module_name));
                        new_lines.push(format!("local function {}", embed_module_function_name));
                        new_lines.push(module_content);
                        new_lines.push("end".to_string());
                        let new_line = next_line.replace(&caps[0], &embed_module_function_name);
                        new_lines.push(new_line);
                    }
                    lines.next();
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "Expected require statement after $embed, found: {}",
                        next_line
                    ));
                }
            }
        } else if let Some(caps) = re.captures(line) {
            let module_name = &caps[1];
            if let Some(index) = modules.get(module_name) {
                let new_line = line.replace(&caps[0], &get_embed_module_function_name(*index));
                new_lines.push(new_line);
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }
    Ok(new_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::common::get_fixture_path;

    use rstest::rstest;

    #[rstest]
    #[case("require 'foo'", "foo")]
    #[case("require(\"bar\")", "bar")]
    #[case("require ( 'baz' )", "baz")]
    #[case("require\t('qux')", "qux")]
    #[case("require('foo.bar')", "foo.bar")]
    #[case("local x = require(\"foo\")", "foo")]
    fn test_module_regex(#[case] input: &str, #[case] expected: &str) {
        let re = Regex::new(MODULE_RE).unwrap();
        let caps = re.captures(input).unwrap();
        assert_eq!(&caps[1], expected);
    }

    #[rstest]
    #[case::basic("basic")]
    #[case::dot("dot")]
    #[case::multi("multi")]
    #[case::repeat("repeat")]
    #[case::only_next_require("only_next_require")]
    fn test_process_embeds_normal(#[case] case_name: &str) {
        let base_dir = get_fixture_path("embed").join(case_name);
        let input_path = base_dir.join("main.lua");
        let input = read_text(&input_path).unwrap();
        let expected_path = base_dir.join("out.lua");
        let expected = read_text(&expected_path).unwrap();
        let output = process_embeds(&input, &base_dir, &[]).unwrap();
        assert_eq!(output, expected);
    }

    #[rstest]
    fn test_process_embeds_elsewhere() {
        let base_dir = get_fixture_path("embed/elsewhere");
        let input_path = base_dir.join("main.lua");
        let input = read_text(&input_path).unwrap();
        let expected_path = base_dir.join("out.lua");
        let expected = read_text(&expected_path).unwrap();
        let search_dirs = [PathBuf::from("lib")];
        let output = process_embeds(&input, &base_dir, &search_dirs).unwrap();
        assert_eq!(output, expected);
    }

    #[rstest]
    fn test_process_embeds_notfound() {
        let base_dir = get_fixture_path("embed/notfound");
        let input_path = base_dir.join("main.lua");
        let input = read_text(&input_path).unwrap();
        let result = process_embeds(&input, &base_dir, &[]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_process_embeds_non_require_error() {
        let base_dir = get_fixture_path("embed/non_require_error");
        let input_path = base_dir.join("main.lua");
        let input = read_text(&input_path).unwrap();
        let result = process_embeds(&input, &base_dir, &[]);
        assert!(result.is_err());
    }
}
