use std::path::PathBuf;

use clap::{Parser, Subcommand};

use aulua::build::build_all;
use aulua::config_loader::load_config;
use aulua::init::init_project;
use aulua::install::install_all;
use aulua::schema::generate_config_schema;

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
    /// auluaプロジェクトを作成する
    Init {
        /// プロジェクトを作成するフォルダを指定する
        #[arg(value_name = "dir", default_value = ".")]
        dir: PathBuf,
    },
    /// スキーマファイルを生成する
    Schema {
        /// 出力先ファイルのパスを指定する
        #[arg(short, long, value_name = "file", default_value = "aulua.schema.json")]
        output: PathBuf,
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
        Commands::Init { dir } => {
            init_project(&dir).expect("プロジェクトの初期化に失敗しました");
        }
        Commands::Schema { output } => {
            generate_config_schema(&output).expect("スキーマ生成に失敗しました");
        }
    }
}
