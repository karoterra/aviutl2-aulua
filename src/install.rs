use std::fs;
use std::path::Path;

use crate::config::Config;
pub fn install_all(
    config: &Config,
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
    use crate::config::Script;

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

        let config = Config {
            project: None,
            build: None,
            install: None,
            scripts: vec![Script {
                name: script_name.to_string(),
                sources: vec![],
            }],
        };

        install_all(&config, tmp_src.path(), tmp_dst.path(), true).unwrap();

        let dst_file = tmp_dst.path().join(script_name);
        assert!(
            !dst_file.exists(),
            "dry-run ではファイルはコピーされないはず"
        );
    }
}
