use std::env;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

use aulua::build::build_all;
use aulua::config_loader::load_config;
use aulua::install::install_all;

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
    Install {
        /// ファイルはコピーせず、処理内容だけ表示する
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            let config = load_config("aulua.yaml").expect("設定ファイルの読み込みに失敗しました");
            build_all(&config, Path::new("build")).expect("ビルド処理に失敗しました");
        }
        Commands::Install { dry_run } => {
            let config = load_config("aulua.yaml").expect("設定ファイルの読み込みに失敗しました");
            let program_data_dir =
                env::var("PROGRAMDATA").expect("PROGRAMDATAが設定されていません");
            let out_dir = PathBuf::from(program_data_dir)
                .join("aviutl2")
                .join("Script");
            install_all(&config, Path::new("build"), &out_dir, dry_run)
                .expect("インストールに失敗しました");
        }
    }
}
