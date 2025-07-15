use std::path::Path;

use clap::{Parser, Subcommand};

use aulua::build::build_all;
use aulua::config_loader::load_config;

#[derive(Parser)]
#[command(about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// スクリプトをビルドする
    Build,
    /// スクリプトをインストールする
    Install,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            let config = load_config("aulua.yaml").expect("設定ファイルの読み込みに失敗しました");
            build_all(&config, Path::new("build")).expect("ビルド処理に失敗しました");
        }
        Commands::Install => {
            println!("インストールします");
        }
    }
}
