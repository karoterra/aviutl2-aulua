use clap::{Parser, Subcommand};

use aulua::build::build_all;
use aulua::config_loader::load_config;
use aulua::install::install_all;

#[derive(Parser)]
#[command(version, about, long_about = None)]
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
            build_all(&config, &config.build_out_dir()).expect("ビルド処理に失敗しました");
        }
        Commands::Install { dry_run } => {
            let config = load_config("aulua.yaml").expect("設定ファイルの読み込みに失敗しました");
            install_all(
                &config,
                &config.build_out_dir(),
                &config.install_out_dir(),
                dry_run,
            )
            .expect("インストールに失敗しました");
        }
    }
}
