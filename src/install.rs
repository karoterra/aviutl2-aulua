use std::fs;
use std::path::Path;

use crate::config::ResolvedConfig;

pub fn install_all(
    config: &ResolvedConfig,
    build_dir: &Path,
    out_dir: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    for script in &config.scripts {
        let src_path = build_dir.join(&script.name);
        let dst_path = out_dir.join(&script.name);

        if dry_run {
            println!(
                "[dry-run] '{}' -> '{}'",
                src_path.display(),
                dst_path.display()
            );
            continue;
        }
        println!(
            "Copying '{}' -> '{}'",
            src_path.display(),
            dst_path.display()
        );
        fs::copy(&src_path, &dst_path).map_err(|e| {
            anyhow::anyhow!("{} をコピーできませんでした: {}", src_path.display(), e)
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_install_all_dry_run() {
        let tmp_src = tempdir().unwrap();
        let tmp_dst = tempdir().unwrap();

        let script_name = "example.lua";
        let src_file = tmp_src.path().join(script_name);
        fs::write(&src_file, "-- dummy script").unwrap();

        let config = ResolvedConfig {
            project: crate::config::ResolvedProject {
                variables: std::collections::HashMap::new(),
            },
            build: crate::config::ResolvedBuild {
                out_dir: tmp_src.path().to_path_buf(),
                embed_search_dirs: vec![],
            },
            install: crate::config::ResolvedInstall {
                out_dir: tmp_dst.path().to_path_buf(),
            },
            package: None,
            scripts: vec![crate::config::ResolvedScript {
                name: script_name.to_string(),
                sources: vec![],
            }],
            config_dir: Path::new(".").to_path_buf(),
        };

        install_all(
            &config,
            &config.build.out_dir,
            &config.install.out_dir,
            true,
        )
        .unwrap();

        let dst_file = config.install.out_dir.join(script_name);
        assert!(
            !dst_file.exists(),
            "dry-run ではファイルはコピーされないはず"
        );
    }
}
