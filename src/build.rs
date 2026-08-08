use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::config::{ResolvedConfig, ResolvedScript};
use crate::embed::process_embeds;
use crate::language_directive::remove_language_directives;
use crate::source_processing::{
    build_source_variables, expand_source_includes, expand_variables, load_source_text,
};
use crate::ui_control::{apply_ui_blocks, parse_ui_blocks};

pub fn build_all(config: &ResolvedConfig, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir)?;

    for script in &config.scripts {
        build_script(script, config, out_dir)?;
    }

    Ok(())
}

fn build_script(
    script: &ResolvedScript,
    config: &ResolvedConfig,
    out_dir: &Path,
) -> anyhow::Result<()> {
    let mut combined = String::new();

    for source in &script.sources {
        let src_path = PathBuf::from(&source.path);
        let content = load_source_text(&src_path)?;

        // ラベル
        if let Some(label) = &source.label {
            combined.push_str(&format!("@{label}\n"));
        }

        // ファイルインクルード
        let content = expand_source_includes(&src_path, &content)?;

        // 埋め込み
        let content = process_embeds(
            &content,
            src_path.parent().unwrap_or(Path::new("")),
            &config.build.embed_search_dirs,
        )?;

        // 変数
        let variables = build_source_variables(config, source)?;
        let expansion = expand_variables(&content, &variables);

        for warning in &expansion.undefined_variables {
            eprintln!(
                "⚠️ 未定義の変数: ${} （{} 内）",
                warning,
                src_path.display()
            );
        }

        let content = remove_language_directives(&expansion.text).with_context(|| {
            format!(
                "languageディレクティブの処理に失敗しました: {}",
                src_path.display()
            )
        })?;

        // UI Control
        let ui_blocks = parse_ui_blocks(&content);
        let content = apply_ui_blocks(&content, &ui_blocks);

        combined.push_str(&content);
        combined.push('\n');
    }

    let out_path = out_dir.join(&script.name);
    fs::write(&out_path, combined)?;
    println!("✅ ビルド完了: {}", out_path.display());

    Ok(())
}
