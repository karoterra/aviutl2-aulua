use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::script_identity::{
    ScriptFileIdentity, ScriptFileKind, ScriptIdentityError, resolve_logical_script_name,
    resolve_script_file_identity,
};

#[derive(Debug)]
pub struct DirectScriptFile<'a> {
    pub identity: ScriptFileIdentity,
    pub preamble: &'a str,
    pub logical_scripts: Vec<DirectLogicalScript<'a>>,
}

#[derive(Debug)]
pub struct DirectLogicalScript<'a> {
    pub name: String,
    pub body: &'a str,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DirectScriptError {
    #[error(transparent)]
    ScriptIdentity(#[from] ScriptIdentityError),
    #[error("language処理の対象ではないスクリプトです: {}", path.display())]
    UnsupportedScriptExtension { path: PathBuf },
    #[error("複数スクリプト形式のlabelが空です: {}:{line_number}", path.display())]
    EmptyLabel { path: PathBuf, line_number: usize },
    #[error("複数スクリプト形式にラベル行がありません: {}", path.display())]
    MissingLabelLine { path: PathBuf },
    #[error("論理スクリプト名が重複しています: {name}")]
    DuplicateLogicalScriptName { name: String },
}

pub fn resolve_direct_script<'a>(
    path: &Path,
    content: &'a str,
) -> Result<DirectScriptFile<'a>, DirectScriptError> {
    let identity = resolve_script_file_identity(path)?.ok_or_else(|| {
        DirectScriptError::UnsupportedScriptExtension {
            path: path.to_path_buf(),
        }
    })?;

    match &identity.kind {
        ScriptFileKind::Single => resolve_single_script(identity, content),
        ScriptFileKind::Multiple { .. } => resolve_multiple_script(path, identity, content),
    }
}

fn resolve_single_script(
    identity: ScriptFileIdentity,
    content: &str,
) -> Result<DirectScriptFile<'_>, DirectScriptError> {
    let name = resolve_logical_script_name(&identity, None)?;

    Ok(DirectScriptFile {
        identity,
        preamble: &content[..0],
        logical_scripts: vec![DirectLogicalScript {
            name,
            body: content,
        }],
    })
}

fn resolve_multiple_script<'a>(
    path: &Path,
    identity: ScriptFileIdentity,
    content: &'a str,
) -> Result<DirectScriptFile<'a>, DirectScriptError> {
    let sections = split_multiple_script(content).map_err(|error| match error {
        SplitMultipleScriptError::EmptyLabel { line_number } => DirectScriptError::EmptyLabel {
            path: path.to_path_buf(),
            line_number,
        },
        SplitMultipleScriptError::MissingLabelLine => DirectScriptError::MissingLabelLine {
            path: path.to_path_buf(),
        },
    })?;
    let mut logical_script_names = HashSet::new();
    let mut logical_scripts = Vec::with_capacity(sections.sections.len());

    for section in sections.sections {
        let name = resolve_logical_script_name(&identity, Some(OsStr::new(section.label)))?;
        if !logical_script_names.insert(name.clone()) {
            return Err(DirectScriptError::DuplicateLogicalScriptName { name });
        }
        logical_scripts.push(DirectLogicalScript {
            name,
            body: section.body,
        });
    }

    Ok(DirectScriptFile {
        identity,
        preamble: sections.preamble,
        logical_scripts,
    })
}

#[derive(Debug, PartialEq, Eq)]
struct MultipleScriptSections<'a> {
    preamble: &'a str,
    sections: Vec<MultipleScriptSection<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
struct MultipleScriptSection<'a> {
    label: &'a str,
    body: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
enum SplitMultipleScriptError {
    EmptyLabel { line_number: usize },
    MissingLabelLine,
}

fn split_multiple_script(
    content: &str,
) -> Result<MultipleScriptSections<'_>, SplitMultipleScriptError> {
    let mut sections = Vec::new();
    let mut current_section: Option<(&str, usize)> = None;
    let mut preamble_end = None;
    let mut line_start = 0;

    for (line_index, line) in content.split_inclusive('\n').enumerate() {
        let line_end = line_start + line.len();
        let line_without_newline = if let Some(line_without_lf) = line.strip_suffix('\n') {
            line_without_lf
                .strip_suffix('\r')
                .unwrap_or(line_without_lf)
        } else {
            line
        };

        if let Some(label) = line_without_newline.strip_prefix('@') {
            if label.is_empty() {
                return Err(SplitMultipleScriptError::EmptyLabel {
                    line_number: line_index + 1,
                });
            }

            if let Some((previous_label, body_start)) = current_section.replace((label, line_end)) {
                sections.push(MultipleScriptSection {
                    label: previous_label,
                    body: &content[body_start..line_start],
                });
            } else {
                preamble_end = Some(line_start);
            }
        }

        line_start = line_end;
    }

    let Some((label, body_start)) = current_section else {
        return Err(SplitMultipleScriptError::MissingLabelLine);
    };
    sections.push(MultipleScriptSection {
        label,
        body: &content[body_start..],
    });

    Ok(MultipleScriptSections {
        preamble: &content[..preamble_end.expect("a label line sets the preamble end")],
        sections,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_single_script_with_entire_content() {
        let content = "first\r\nsecond\nlast";

        let output = resolve_direct_script(Path::new("scripts/effect.anm2"), content).unwrap();

        assert!(output.preamble.is_empty());
        assert_eq!(output.logical_scripts.len(), 1);
        assert_eq!(output.logical_scripts[0].name, "effect");
        assert_eq!(output.logical_scripts[0].body, content);
        assert_eq!(output.logical_scripts[0].body.as_ptr(), content.as_ptr());
    }

    #[test]
    fn resolves_empty_single_script() {
        let output = resolve_direct_script(Path::new("empty.obj2"), "").unwrap();

        assert!(output.preamble.is_empty());
        assert_eq!(output.logical_scripts.len(), 1);
        assert_eq!(output.logical_scripts[0].name, "empty");
        assert!(output.logical_scripts[0].body.is_empty());
    }

    #[test]
    fn unsupported_extension_is_an_error() {
        for path in ["legacy.anm", "script.lua", "script"] {
            assert_eq!(
                resolve_direct_script(Path::new(path), "").unwrap_err(),
                DirectScriptError::UnsupportedScriptExtension {
                    path: PathBuf::from(path)
                }
            );
        }
    }

    #[test]
    fn resolves_multiple_script_with_lf() {
        let content = "preamble\n@Part 1\nbody 1\n@Part 2\nbody 2\n";

        let output = resolve_direct_script(Path::new("@combined.anm2"), content).unwrap();

        assert_eq!(output.preamble, "preamble\n");
        assert_eq!(output.logical_scripts.len(), 2);
        assert_eq!(output.logical_scripts[0].name, "Part 1@combined");
        assert_eq!(output.logical_scripts[0].body, "body 1\n");
        assert_eq!(output.logical_scripts[1].name, "Part 2@combined");
        assert_eq!(output.logical_scripts[1].body, "body 2\n");
    }

    #[test]
    fn resolves_multiple_script_with_crlf_without_changing_line_endings() {
        let content = "preamble\r\n@Part 1\r\nbody 1\r\n@Part 2\r\nbody 2\r\n";

        let output = resolve_direct_script(Path::new("@combined.obj2"), content).unwrap();

        assert_eq!(output.preamble, "preamble\r\n");
        assert_eq!(output.logical_scripts[0].name, "Part 1@combined");
        assert_eq!(output.logical_scripts[0].body, "body 1\r\n");
        assert_eq!(output.logical_scripts[1].name, "Part 2@combined");
        assert_eq!(output.logical_scripts[1].body, "body 2\r\n");
    }

    #[test]
    fn preserves_mixed_line_endings_without_normalization() {
        let content = "pre\r\n@First\nbody 1\r\n@Second\r\nbody 2\n";

        let output = resolve_direct_script(Path::new("@mixed.scn2"), content).unwrap();

        assert_eq!(output.preamble, "pre\r\n");
        assert_eq!(output.logical_scripts[0].body, "body 1\r\n");
        assert_eq!(output.logical_scripts[1].body, "body 2\n");
    }

    #[test]
    fn does_not_treat_lone_cr_as_a_line_ending() {
        let output = resolve_direct_script(Path::new("@combined.anm2"), "@Label\r").unwrap();

        assert_eq!(output.logical_scripts[0].name, "Label\r@combined");
        assert!(output.logical_scripts[0].body.is_empty());
    }

    #[test]
    fn resolves_label_and_body_without_trailing_newline() {
        let content = "@Part\nbody";

        let output = resolve_direct_script(Path::new("@combined.cam2"), content).unwrap();

        assert!(output.preamble.is_empty());
        assert_eq!(output.logical_scripts[0].name, "Part@combined");
        assert_eq!(output.logical_scripts[0].body, "body");
    }

    #[test]
    fn indented_at_signs_are_not_label_lines() {
        let content = " @not-label\n\t@also-not-label\n@Part\nbody\n @still-body\n";

        let output = resolve_direct_script(Path::new("@combined.tra2"), content).unwrap();

        assert_eq!(output.preamble, " @not-label\n\t@also-not-label\n");
        assert_eq!(output.logical_scripts[0].body, "body\n @still-body\n");
    }

    #[test]
    fn label_detection_does_not_consider_lua_syntax() {
        let content = concat!(
            "-- @not a label\n",
            "local value = [[\n",
            "@Inside string\n",
            "string body\n",
            "]]\n",
            "--[[\n",
            "@Inside comment\n",
            "comment body\n",
            "]]\n",
        );

        let output = resolve_direct_script(Path::new("@combined.anm2"), content).unwrap();

        assert_eq!(output.preamble, "-- @not a label\nlocal value = [[\n");
        assert_eq!(output.logical_scripts[0].name, "Inside string@combined");
        assert_eq!(output.logical_scripts[0].body, "string body\n]]\n--[[\n");
        assert_eq!(output.logical_scripts[1].name, "Inside comment@combined");
        assert_eq!(output.logical_scripts[1].body, "comment body\n]]\n");
    }

    #[test]
    fn empty_label_is_an_error_for_each_supported_line_ending() {
        for (content, expected_line) in [("@\n", 1), ("preamble\r\n@\r\n", 2), ("@", 1)] {
            assert_eq!(
                resolve_direct_script(Path::new("@combined.anm2"), content).unwrap_err(),
                DirectScriptError::EmptyLabel {
                    path: PathBuf::from("@combined.anm2"),
                    line_number: expected_line,
                }
            );
        }
    }

    #[test]
    fn whitespace_only_label_is_valid_and_not_trimmed() {
        let output = resolve_direct_script(Path::new("@combined.anm2"), "@ \r\n").unwrap();

        assert_eq!(output.logical_scripts[0].name, " @combined");
        assert!(output.logical_scripts[0].body.is_empty());

        let output = resolve_direct_script(Path::new("@combined.anm2"), "@ Label \n").unwrap();
        assert_eq!(output.logical_scripts[0].name, " Label @combined");
    }

    #[test]
    fn consecutive_and_final_labels_create_empty_bodies() {
        for content in ["@First\n@Second\n", "@First\n@Second"] {
            let output = resolve_direct_script(Path::new("@combined.anm2"), content).unwrap();

            assert_eq!(output.logical_scripts.len(), 2);
            assert_eq!(output.logical_scripts[0].name, "First@combined");
            assert!(output.logical_scripts[0].body.is_empty());
            assert_eq!(output.logical_scripts[1].name, "Second@combined");
            assert!(output.logical_scripts[1].body.is_empty());
        }
    }

    #[test]
    fn multiple_script_without_label_line_is_an_error() {
        for content in ["", "preamble\n", " @not-label\n"] {
            assert_eq!(
                resolve_direct_script(Path::new("@combined.anm2"), content).unwrap_err(),
                DirectScriptError::MissingLabelLine {
                    path: PathBuf::from("@combined.anm2")
                }
            );
        }
    }

    #[test]
    fn duplicate_logical_script_name_is_an_error() {
        let content = "@Part\nfirst\n@Part\nsecond\n";

        assert_eq!(
            resolve_direct_script(Path::new("@combined.anm2"), content).unwrap_err(),
            DirectScriptError::DuplicateLogicalScriptName {
                name: "Part@combined".to_string()
            }
        );
    }

    #[test]
    fn duplicate_detection_is_case_sensitive_and_does_not_normalize_unicode() {
        let content = "@Name\n@name\n@é\n@e\u{301}\n";

        let output = resolve_direct_script(Path::new("@combined.anm2"), content).unwrap();

        assert_eq!(output.logical_scripts.len(), 4);
        assert_eq!(output.logical_scripts[0].name, "Name@combined");
        assert_eq!(output.logical_scripts[1].name, "name@combined");
        assert_eq!(output.logical_scripts[2].name, "é@combined");
        assert_eq!(output.logical_scripts[3].name, "e\u{301}@combined");
    }

    #[test]
    fn reuses_multiple_script_container_resolution() {
        let output = resolve_direct_script(Path::new("@@foo.ANM2"), "@Label\n").unwrap();

        assert_eq!(output.logical_scripts[0].name, "Label@@foo");
    }

    #[test]
    fn script_identity_errors_are_wrapped() {
        assert!(matches!(
            resolve_direct_script(Path::new("@.anm2"), "@Label\n"),
            Err(DirectScriptError::ScriptIdentity(
                ScriptIdentityError::EmptyContainer { .. }
            ))
        ));
    }
}
