use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use thiserror::Error;

const SUPPORTED_SCRIPT_EXTENSIONS: &[&str] = &["anm2", "obj2", "scn2", "cam2", "tra2"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptFileKind {
    Single,
    Multiple { container: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFileIdentity {
    pub file_name: String,
    pub extension: String,
    pub file_stem: String,
    pub kind: ScriptFileKind,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScriptIdentityError {
    #[error("スクリプトのファイル名を取得できません: {}", path.display())]
    MissingFileName { path: PathBuf },
    #[error("スクリプトのファイル名をUTF-8文字列として扱えません: {}", path.display())]
    NonUtf8FileName { path: PathBuf },
    #[error("スクリプトのfile stemを取得できません: {}", path.display())]
    MissingFileStem { path: PathBuf },
    #[error("スクリプトのfile stemが空です: {}", path.display())]
    EmptyFileStem { path: PathBuf },
    #[error("複数スクリプト形式のcontainerが空です: {}", path.display())]
    EmptyContainer { path: PathBuf },
    #[error("複数スクリプト形式のlabelが指定されていません")]
    MissingLabel,
    #[error("スクリプトのlabelをUTF-8文字列として扱えません")]
    NonUtf8Label,
    #[error("複数スクリプト形式のlabelが空です")]
    EmptyLabel,
}

pub fn is_supported_script_extension(extension: &str) -> bool {
    SUPPORTED_SCRIPT_EXTENSIONS
        .iter()
        .any(|supported| extension.eq_ignore_ascii_case(supported))
}

pub fn resolve_script_file_identity(
    path: &Path,
) -> Result<Option<ScriptFileIdentity>, ScriptIdentityError> {
    let Some(extension) = path.extension() else {
        return Ok(None);
    };
    let extension = extension
        .to_str()
        .ok_or_else(|| ScriptIdentityError::NonUtf8FileName {
            path: path.to_path_buf(),
        })?;

    if !is_supported_script_extension(extension) {
        return Ok(None);
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| ScriptIdentityError::MissingFileName {
            path: path.to_path_buf(),
        })?
        .to_str()
        .ok_or_else(|| ScriptIdentityError::NonUtf8FileName {
            path: path.to_path_buf(),
        })?;

    let file_stem = path
        .file_stem()
        .ok_or_else(|| ScriptIdentityError::MissingFileStem {
            path: path.to_path_buf(),
        })?
        .to_str()
        .ok_or_else(|| ScriptIdentityError::NonUtf8FileName {
            path: path.to_path_buf(),
        })?;

    if file_stem.is_empty() {
        return Err(ScriptIdentityError::EmptyFileStem {
            path: path.to_path_buf(),
        });
    }

    let kind = if let Some(container) = file_stem.strip_prefix('@') {
        if container.is_empty() {
            return Err(ScriptIdentityError::EmptyContainer {
                path: path.to_path_buf(),
            });
        }
        ScriptFileKind::Multiple {
            container: container.to_string(),
        }
    } else {
        ScriptFileKind::Single
    };

    Ok(Some(ScriptFileIdentity {
        file_name: file_name.to_string(),
        extension: extension.to_string(),
        file_stem: file_stem.to_string(),
        kind,
    }))
}

pub fn resolve_logical_script_name(
    script: &ScriptFileIdentity,
    label: Option<&OsStr>,
) -> Result<String, ScriptIdentityError> {
    match &script.kind {
        ScriptFileKind::Single => Ok(script.file_stem.clone()),
        ScriptFileKind::Multiple { container } => {
            let label = label.ok_or(ScriptIdentityError::MissingLabel)?;
            let label = label.to_str().ok_or(ScriptIdentityError::NonUtf8Label)?;
            if label.is_empty() {
                return Err(ScriptIdentityError::EmptyLabel);
            }
            Ok(format!("{label}@{container}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn supported_extensions_are_ascii_case_insensitive() {
        for extension in ["anm2", "obj2", "scn2", "cam2", "tra2"] {
            assert!(is_supported_script_extension(extension));
            assert!(is_supported_script_extension(
                &extension.to_ascii_uppercase()
            ));
        }
        assert!(!is_supported_script_extension("anm"));
        assert!(!is_supported_script_extension("lua"));
        assert!(!is_supported_script_extension("anm2.bak"));
    }

    #[test]
    fn resolves_single_script_file() {
        let identity = resolve_script_file_identity(Path::new("scripts/My.Effect.ANM2"))
            .unwrap()
            .unwrap();

        assert_eq!(identity.file_name, "My.Effect.ANM2");
        assert_eq!(identity.extension, "ANM2");
        assert_eq!(identity.file_stem, "My.Effect");
        assert_eq!(identity.kind, ScriptFileKind::Single);
        assert_eq!(
            resolve_logical_script_name(&identity, Some(OsStr::new("ignored"))).unwrap(),
            "My.Effect"
        );
    }

    #[test]
    fn resolves_multiple_script_container_and_logical_name() {
        let identity = resolve_script_file_identity(Path::new("scripts/@combined.anm2"))
            .unwrap()
            .unwrap();

        assert_eq!(identity.file_stem, "@combined");
        assert_eq!(
            identity.kind,
            ScriptFileKind::Multiple {
                container: "combined".to_string()
            }
        );
        assert_eq!(
            resolve_logical_script_name(&identity, Some(OsStr::new("Combined Part 1"))).unwrap(),
            "Combined Part 1@combined"
        );
    }

    #[test]
    fn removes_only_one_leading_at_sign_from_container() {
        let identity = resolve_script_file_identity(Path::new("@@foo.anm2"))
            .unwrap()
            .unwrap();

        assert_eq!(identity.file_stem, "@@foo");
        assert_eq!(
            identity.kind,
            ScriptFileKind::Multiple {
                container: "@foo".to_string()
            }
        );
        assert_eq!(
            resolve_logical_script_name(&identity, Some(OsStr::new("label"))).unwrap(),
            "label@@foo"
        );
    }

    #[test]
    fn unsupported_extension_returns_none() {
        assert_eq!(
            resolve_script_file_identity(Path::new("legacy.anm")).unwrap(),
            None
        );
        assert_eq!(
            resolve_script_file_identity(Path::new("script.anm2.bak")).unwrap(),
            None
        );
        assert_eq!(
            resolve_script_file_identity(Path::new("script")).unwrap(),
            None
        );
        assert_eq!(
            resolve_script_file_identity(Path::new("@.lua")).unwrap(),
            None
        );
        let non_utf8_stem = PathBuf::from(non_utf8_file_name_with_extension("lua"));
        assert_eq!(resolve_script_file_identity(&non_utf8_stem).unwrap(), None);
    }

    #[test]
    fn empty_container_is_an_error() {
        assert!(matches!(
            resolve_script_file_identity(Path::new("@.anm2")),
            Err(ScriptIdentityError::EmptyContainer { .. })
        ));
    }

    #[test]
    fn path_without_extension_is_not_supported() {
        assert_eq!(resolve_script_file_identity(Path::new("")).unwrap(), None);
    }

    #[test]
    fn multiple_script_requires_non_empty_label() {
        let identity = resolve_script_file_identity(Path::new("@foo.anm2"))
            .unwrap()
            .unwrap();

        assert_eq!(
            resolve_logical_script_name(&identity, None),
            Err(ScriptIdentityError::MissingLabel)
        );
        assert_eq!(
            resolve_logical_script_name(&identity, Some(OsStr::new(""))),
            Err(ScriptIdentityError::EmptyLabel)
        );
    }

    #[test]
    fn label_is_not_trimmed() {
        let identity = resolve_script_file_identity(Path::new("@foo.anm2"))
            .unwrap()
            .unwrap();

        assert_eq!(
            resolve_logical_script_name(&identity, Some(OsStr::new(" label "))).unwrap(),
            " label @foo"
        );
        assert_eq!(
            resolve_logical_script_name(&identity, Some(OsStr::new(" \t"))).unwrap(),
            " \t@foo"
        );
    }

    #[test]
    fn non_utf8_file_name_is_an_error() {
        let path = PathBuf::from(non_utf8_file_name_with_extension("anm2"));
        assert!(matches!(
            resolve_script_file_identity(&path),
            Err(ScriptIdentityError::NonUtf8FileName { .. })
        ));
    }

    #[test]
    fn non_utf8_extension_is_an_error() {
        let path = PathBuf::from(file_name_with_non_utf8_extension());
        assert!(matches!(
            resolve_script_file_identity(&path),
            Err(ScriptIdentityError::NonUtf8FileName { .. })
        ));
    }

    #[test]
    fn non_utf8_label_is_an_error() {
        let identity = resolve_script_file_identity(Path::new("@foo.anm2"))
            .unwrap()
            .unwrap();
        let label = non_utf8_os_string();

        assert_eq!(
            resolve_logical_script_name(&identity, Some(&label)),
            Err(ScriptIdentityError::NonUtf8Label)
        );
    }

    #[test]
    fn single_script_ignores_non_utf8_label() {
        let identity = resolve_script_file_identity(Path::new("foo.anm2"))
            .unwrap()
            .unwrap();
        let label = non_utf8_os_string();

        assert_eq!(
            resolve_logical_script_name(&identity, Some(&label)).unwrap(),
            "foo"
        );
    }

    #[cfg(unix)]
    fn non_utf8_os_string() -> OsString {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![0xff])
    }

    #[cfg(unix)]
    fn non_utf8_file_name_with_extension(extension: &str) -> OsString {
        use std::os::unix::ffi::OsStringExt;
        let mut bytes = vec![0xff, b'.'];
        bytes.extend_from_slice(extension.as_bytes());
        OsString::from_vec(bytes)
    }

    #[cfg(unix)]
    fn file_name_with_non_utf8_extension() -> OsString {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![b'f', b'o', b'o', b'.', 0xff])
    }

    #[cfg(windows)]
    fn non_utf8_os_string() -> OsString {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[0xd800])
    }

    #[cfg(windows)]
    fn non_utf8_file_name_with_extension(extension: &str) -> OsString {
        use std::os::windows::ffi::OsStringExt;
        let mut wide = vec![0xd800, u16::from(b'.')];
        wide.extend(extension.encode_utf16());
        OsString::from_wide(&wide)
    }

    #[cfg(windows)]
    fn file_name_with_non_utf8_extension() -> OsString {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[
            u16::from(b'f'),
            u16::from(b'o'),
            u16::from(b'o'),
            u16::from(b'.'),
            0xd800,
        ])
    }
}
