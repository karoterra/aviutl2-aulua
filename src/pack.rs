use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::build::build_all;
use crate::config::{PackConfig, ResolvedConfig, ResolvedPackageMessage};

const INSTALL_ROOT_DIRS: &[&str] = &[
    "Plugin",
    "Script",
    "Language",
    "Alias",
    "Figure",
    "Transition",
    "Preset",
    "Default",
];

pub fn pack_project(config: &ResolvedConfig) -> Result<PathBuf> {
    let pack = config.package_for_pack()?;

    build_all(config, &config.build.out_dir)?;

    fs::create_dir_all(&pack.out_dir)?;
    let out_path = pack.out_dir.join(&pack.file_name);

    let file = File::create(&out_path).map_err(|e| {
        anyhow!(
            "パッケージファイルの作成に失敗しました: {}: {}",
            out_path.display(),
            e
        )
    })?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let mut seen_paths = HashSet::new();

    // package.ini
    let package_ini = render_package_ini(&pack);
    add_text_to_zip(
        &mut zip,
        &mut seen_paths,
        "package.ini",
        &package_ini,
        options,
    )?;

    // package.txt
    if let Some(message) = &pack.message {
        let package_txt = read_package_message(message)?;
        let package_txt = to_crlf(&package_txt);
        add_text_to_zip(
            &mut zip,
            &mut seen_paths,
            "package.txt",
            &package_txt,
            options,
        )?;
    }

    // scripts
    for script in &config.scripts {
        let built_path = config.build.out_dir.join(&script.name);

        if !built_path.exists() {
            return Err(anyhow!(
                "ビルド成果物が見つかりません: {}",
                built_path.display()
            ));
        }

        let archive_path = join_archive_paths(&["Script", &pack.script_sub_dir, &script.name])?;
        validate_install_archive_path(&archive_path)?;

        add_file_to_zip(
            &mut zip,
            &mut seen_paths,
            &built_path,
            &archive_path,
            options,
        )?;
    }

    // package.assets
    for asset in &pack.assets {
        if !asset.src.exists() {
            return Err(anyhow!(
                "package.assets の入力ファイルが見つかりません: {}",
                asset.src.display()
            ));
        }

        let archive_path = normalize_archive_path(&asset.dest)?;
        validate_install_archive_path(&archive_path)?;

        add_file_to_zip(
            &mut zip,
            &mut seen_paths,
            &asset.src,
            &archive_path,
            options,
        )?;
    }

    zip.finish()
        .map_err(|e| anyhow!("zip ファイルの書き込み完了に失敗しました: {}", e))?;

    println!("✅ パッケージ作成完了: {}", out_path.display());

    Ok(out_path)
}

fn render_package_ini(pack: &PackConfig) -> String {
    format!(
        "[package]\r\nid={}\r\nname={}\r\ninformation={}\r\n",
        pack.id, pack.name, pack.information
    )
}

fn to_crlf(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                out.push_str("\r\n");
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
            }
            '\n' => {
                out.push_str("\r\n");
            }
            _ => out.push(c),
        }
    }

    out
}

fn read_package_message(message: &ResolvedPackageMessage) -> Result<String> {
    match message {
        ResolvedPackageMessage::Text(text) => Ok(text.clone()),
        ResolvedPackageMessage::File(path) => fs::read_to_string(path).map_err(|e| {
            anyhow!(
                "package.message.file の読み込みに失敗しました: {}: {}",
                path.display(),
                e
            )
        }),
    }
}

fn add_text_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    seen_paths: &mut HashSet<String>,
    archive_path: &str,
    content: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    ensure_unique_archive_path(seen_paths, archive_path)?;

    zip.start_file(archive_path, options).map_err(|e| {
        anyhow!(
            "zip 内ファイルの開始に失敗しました: {}: {}",
            archive_path,
            e
        )
    })?;
    zip.write_all(content.as_bytes())
        .map_err(|e| anyhow!("zip への書き込みに失敗しました: {}: {}", archive_path, e))?;

    Ok(())
}

fn add_file_to_zip<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    seen_paths: &mut HashSet<String>,
    src_path: &Path,
    archive_path: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    ensure_unique_archive_path(seen_paths, archive_path)?;

    let mut file = File::open(src_path).map_err(|e| {
        anyhow!(
            "入力ファイルを開けませんでした: {}: {}",
            src_path.display(),
            e
        )
    })?;

    zip.start_file(archive_path, options).map_err(|e| {
        anyhow!(
            "zip 内ファイルの開始に失敗しました: {}: {}",
            archive_path,
            e
        )
    })?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| {
        anyhow!(
            "入力ファイルの読み込みに失敗しました: {}: {}",
            src_path.display(),
            e
        )
    })?;

    zip.write_all(&buffer)
        .map_err(|e| anyhow!("zip への書き込みに失敗しました: {}: {}", archive_path, e))?;

    Ok(())
}

fn ensure_unique_archive_path(seen_paths: &mut HashSet<String>, archive_path: &str) -> Result<()> {
    if !seen_paths.insert(archive_path.to_string()) {
        return Err(anyhow!(
            "zip 内に同じパスが複数回追加されています: {}",
            archive_path
        ));
    }
    Ok(())
}

fn join_archive_paths(parts: &[&str]) -> Result<String> {
    let mut joined = String::new();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        if !joined.is_empty() {
            joined.push('/');
        }
        joined.push_str(part);
    }

    normalize_archive_path(&joined)
}

fn normalize_archive_path(path: &str) -> Result<String> {
    let path = path.replace('\\', "/");

    if path.trim().is_empty() {
        return Err(anyhow!("zip 内パスが空です。"));
    }

    if path.starts_with('/') {
        return Err(anyhow!("zip 内パスに絶対パスは使用できません: {}", path));
    }

    let mut normalized_parts = Vec::new();

    for part in path.split('/') {
        if part.is_empty() {
            return Err(anyhow!(
                "zip 内パスに空の要素を含めることはできません: {}",
                path
            ));
        }
        if part == "." {
            return Err(anyhow!(
                "zip 内パスに '.' を含めることはできません: {}",
                path
            ));
        }
        if part == ".." {
            return Err(anyhow!(
                "zip 内パスに '..' を含めることはできません: {}",
                path
            ));
        }
        if part.contains(':') {
            return Err(anyhow!(
                "zip 内パスにドライブレター風の要素を含めることはできません: {}",
                path
            ));
        }

        normalized_parts.push(part);
    }

    Ok(normalized_parts.join("/"))
}

fn validate_install_archive_path(path: &str) -> Result<()> {
    let normalized = normalize_archive_path(path)?;

    match normalized.as_str() {
        "package.ini" | "package.txt" => {
            return Err(anyhow!(
                "{} は package 専用ファイルです。 assets や scripts の配置先には使えません。",
                normalized
            ));
        }
        _ => {}
    }

    let first = normalized
        .split('/')
        .next()
        .ok_or_else(|| anyhow!("zip 内パスの解釈に失敗しました: {}", normalized))?;

    if !INSTALL_ROOT_DIRS.contains(&first) {
        return Err(anyhow!(
            "zip 内パスの先頭ディレクトリが不正です: {} (許可: {:?})",
            normalized,
            INSTALL_ROOT_DIRS
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_crlf_converts_lf_to_crlf() {
        let input = "a\nb\nc";
        let actual = to_crlf(input);
        assert_eq!(actual, "a\r\nb\r\nc");
    }

    #[test]
    fn to_crlf_keeps_existing_crlf() {
        let input = "a\r\nb\r\nc";
        let actual = to_crlf(input);
        assert_eq!(actual, "a\r\nb\r\nc");
    }

    #[test]
    fn to_crlf_normalizes_mixed_newlines() {
        let input = "a\nb\r\nc\rd";
        let actual = to_crlf(input);
        assert_eq!(actual, "a\r\nb\r\nc\r\nd");
    }

    #[test]
    fn render_package_ini_uses_crlf_and_expected_keys() {
        let pack = PackConfig {
            id: "karoterra.example".to_string(),
            name: "Example".to_string(),
            information: "Example package".to_string(),
            version: Some("1.2.3".to_string()),
            out_dir: PathBuf::from("dist"),
            file_name: "karoterra.example-v1.2.3.au2pkg.zip".to_string(),
            script_sub_dir: "karoterra.example".to_string(),
            message: None,
            assets: vec![],
        };

        let ini = render_package_ini(&pack);

        assert_eq!(
            ini,
            "[package]\r\nid=karoterra.example\r\nname=Example\r\ninformation=Example package\r\n"
        );
    }

    #[test]
    fn normalize_archive_path_replaces_backslashes() {
        let actual = normalize_archive_path(r"Script\pkg\main.anm2").unwrap();
        assert_eq!(actual, "Script/pkg/main.anm2");
    }

    #[test]
    fn normalize_archive_path_rejects_empty() {
        assert!(normalize_archive_path("").is_err());
        assert!(normalize_archive_path("   ").is_err());
    }

    #[test]
    fn normalize_archive_path_rejects_absolute_path() {
        assert!(normalize_archive_path("/Script/pkg/main.anm2").is_err());
    }

    #[test]
    fn normalize_archive_path_rejects_parent_dir() {
        assert!(normalize_archive_path("Script/../main.anm2").is_err());
    }

    #[test]
    fn normalize_archive_path_rejects_current_dir() {
        assert!(normalize_archive_path("Script/./main.anm2").is_err());
    }

    #[test]
    fn normalize_archive_path_rejects_empty_component() {
        assert!(normalize_archive_path("Script//main.anm2").is_err());
    }

    #[test]
    fn normalize_archive_path_rejects_drive_letter_like_component() {
        assert!(normalize_archive_path("Script/C:/main.anm2").is_err());
    }

    #[test]
    fn validate_install_archive_path_accepts_script_path() {
        assert!(validate_install_archive_path("Script/pkg/main.anm2").is_ok());
    }

    #[test]
    fn validate_install_archive_path_accepts_language_path() {
        assert!(validate_install_archive_path("Language/English.aul2").is_ok());
    }

    #[test]
    fn validate_install_archive_path_rejects_package_ini() {
        assert!(validate_install_archive_path("package.ini").is_err());
    }

    #[test]
    fn validate_install_archive_path_rejects_package_txt() {
        assert!(validate_install_archive_path("package.txt").is_err());
    }

    #[test]
    fn validate_install_archive_path_rejects_unknown_root() {
        assert!(validate_install_archive_path("Docs/README.md").is_err());
    }

    #[test]
    fn join_archive_paths_skips_empty_parts() {
        let actual = join_archive_paths(&["Script", "", "pkg", "main.anm2"]).unwrap();
        assert_eq!(actual, "Script/pkg/main.anm2");
    }

    #[test]
    fn ensure_unique_archive_path_rejects_duplicates() {
        let mut seen = HashSet::new();

        ensure_unique_archive_path(&mut seen, "Script/pkg/main.anm2").unwrap();
        assert!(ensure_unique_archive_path(&mut seen, "Script/pkg/main.anm2").is_err());
    }

    #[test]
    fn read_package_message_from_text_returns_text_as_is() {
        let message = ResolvedPackageMessage::Text("hello\nworld".to_string());
        let actual = read_package_message(&message).unwrap();
        assert_eq!(actual, "hello\nworld");
    }

    #[test]
    fn read_package_message_from_file_reads_file_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("package.txt");
        fs::write(&path, "line1\nline2").unwrap();

        let message = ResolvedPackageMessage::File(path);
        let actual = read_package_message(&message).unwrap();

        assert_eq!(actual, "line1\nline2");
    }
}
