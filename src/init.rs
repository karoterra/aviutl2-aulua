use std::fs;
use std::path::Path;

const TEMPLATE_CONFIG: &str = include_str!("../template/aulua.yaml");
const TEMPLATE_GITIGNORE: &str = include_str!("../template/gitignore.txt");
const TEMPLATE_GITATTRIBUTES: &str = include_str!("../template/gitattributes.txt");
const TEMPLATE_EDITORCONFIG: &str = include_str!("../template/.editorconfig");
const TEMPLATE_SCRIPT: &str = include_str!("../template/SampleAnimationEffect.anm2");
const TEMPLATE_SHADER: &str = include_str!("../template/SampleShader.hlsl");

pub fn init_project(dir: &Path) -> anyhow::Result<()> {
    let script_dir = dir.join("script");
    fs::create_dir_all(&script_dir)?;

    let config_path = dir.join("aulua.yaml");
    create_file(&config_path, TEMPLATE_CONFIG)?;

    let gitignore_path = dir.join(".gitignore");
    create_file(&gitignore_path, TEMPLATE_GITIGNORE)?;

    let gitattributes_path = dir.join(".gitattributes");
    create_file(&gitattributes_path, TEMPLATE_GITATTRIBUTES)?;

    let editorconfig_path = dir.join(".editorconfig");
    create_file(&editorconfig_path, TEMPLATE_EDITORCONFIG)?;

    let script_path = script_dir.join("SampleAnimationEffect.anm2");
    create_file(&script_path, TEMPLATE_SCRIPT)?;

    let shader_path = script_dir.join("SampleShader.hlsl");
    create_file(&shader_path, TEMPLATE_SHADER)?;

    Ok(())
}

fn create_file(path: &Path, content: &str) -> anyhow::Result<()> {
    if path.exists() {
        eprintln!("'{}' はすでに存在します", path.display());
    } else {
        fs::write(path, content)?;
        println!("'{}' を作成しました", path.display());
    }
    Ok(())
}
