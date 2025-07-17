use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn process_includes(
    content: &str,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<String, String> {
    let mut result = Vec::new();

    for (line_num, line) in content.split('\n').enumerate() {
        let trimed = line.trim();
        if let Some(rest) = trimed.strip_prefix("---$include") {
            let rest = rest.trim();
            if let Some(include_path_str) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
            {
                let include_path = base_dir.join(include_path_str);
                let canonical = fs::canonicalize(&include_path).map_err(|e| {
                    format!(
                        "Failed to resolve include path {} at line {}: {}",
                        include_path.display(),
                        line_num + 1,
                        e
                    )
                })?;

                if visited.contains(&canonical) {
                    return Err(format!(
                        "Circular include detected: {}",
                        canonical.display()
                    ));
                }

                visited.insert(canonical.clone());

                let include_content = fs::read_to_string(&canonical).map_err(|e| {
                    format!(
                        "Failed to read included file {}: {}",
                        canonical.display(),
                        e
                    )
                })?;

                let included_result = process_includes(
                    &include_content,
                    canonical.parent().unwrap_or(Path::new("")),
                    visited,
                )?;
                result.push(included_result);
            } else {
                return Err(format!(
                    "Malformed $include at line {}: expected quoted path",
                    line_num + 1
                ));
            }
        } else {
            result.push(line.to_string());
        }
    }

    Ok(result.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    #[path = "../../../tests/common/mod.rs"]
    mod common;
    use common::read_fixture;

    #[test]
    fn test_nested_includes() {
        let input = read_fixture("include_main.lua");
        let expected = read_fixture("include_main_out.lua");

        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut visited = HashSet::new();

        let result = process_includes(&input, &base_dir, &mut visited);
        match result {
            Ok(output) => assert_eq!(output, expected),
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn test_cycle_includes() {
        let input = read_fixture("include_cycle_a.lua");

        let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut visited = HashSet::new();

        let result = process_includes(&input, &base_dir, &mut visited);

        assert!(result.is_err());
        let error_message = result.unwrap_err();
        assert!(error_message.starts_with("Circular include detected"));
    }
}
