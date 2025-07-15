use std::fs;
use std::path::{Path, PathBuf};

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

        if let Some(label) = &source.label {
            combined.push_str(&format!("@{}\n", label));
        }

        combined.push_str(&content);
        combined.push_str("\n");
    }

    let out_path = out_dir.join(&script.name);
    fs::write(&out_path, combined)?;
    println!("✅ ビルド完了: {}", out_path.display());

    Ok(())
}
