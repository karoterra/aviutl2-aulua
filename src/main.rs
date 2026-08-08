use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use aulua::build::build_all;
use aulua::config_loader::load_config;
use aulua::init::init_project;
use aulua::install::install_all;
use aulua::language::update_language;
use aulua::pack::pack_project;
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
    /// au2pkg パッケージを作成する
    Pack,
    /// language fileを管理する
    Language(LanguageArgs),
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

#[derive(Args)]
struct LanguageArgs {
    #[command(subcommand)]
    command: LanguageCommand,
}

#[derive(Subcommand)]
enum LanguageCommand {
    /// language fileを作成・更新する
    Update(LanguageUpdateArgs),
}

#[derive(Args)]
struct LanguageUpdateArgs {
    /// 解析するbuild済みスクリプトを指定する
    #[arg(long, value_name = "path")]
    script: Vec<PathBuf>,
    /// 更新するlanguage fileを上書き指定する
    #[arg(long, value_name = "path")]
    output: Option<PathBuf>,
    /// 使用されていないkeyを削除する
    #[arg(long)]
    prune: bool,
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
        Commands::Pack => {
            let config = load_config("aulua.yaml").expect("設定ファイルの読み込みに失敗しました");
            pack_project(&config).expect("パッケージ作成に失敗しました");
        }
        Commands::Language(LanguageArgs {
            command: LanguageCommand::Update(args),
        }) => {
            update_language(&args.script, args.output.as_deref(), args.prune)
                .expect("language fileの更新に失敗しました");
        }
        Commands::Init { dir } => {
            init_project(&dir).expect("プロジェクトの初期化に失敗しました");
        }
        Commands::Schema { output } => {
            generate_config_schema(&output).expect("スキーマ生成に失敗しました");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_update(args: &[&str]) -> LanguageUpdateArgs {
        let cli = Cli::try_parse_from(args).unwrap();
        let Commands::Language(LanguageArgs {
            command: LanguageCommand::Update(args),
        }) = cli.command
        else {
            panic!("language updateとしてparseされませんでした");
        };
        args
    }

    #[test]
    fn parses_language_update_with_defaults() {
        let args = parse_update(&["aulua", "language", "update"]);

        assert!(args.script.is_empty());
        assert_eq!(args.output, None);
        assert!(!args.prune);
    }

    #[test]
    fn parses_repeated_scripts_in_order() {
        let args = parse_update(&[
            "aulua", "language", "update", "--script", "a.anm2", "--script", "b.obj2",
        ]);

        assert_eq!(
            args.script,
            vec![PathBuf::from("a.anm2"), PathBuf::from("b.obj2")]
        );
    }

    #[test]
    fn parses_output_override() {
        let args = parse_update(&[
            "aulua",
            "language",
            "update",
            "--output",
            "Language/English.aul2",
        ]);

        assert_eq!(args.output, Some(PathBuf::from("Language/English.aul2")));
    }

    #[test]
    fn parses_prune_flag() {
        let args = parse_update(&["aulua", "language", "update", "--prune"]);

        assert!(args.prune);
    }

    #[test]
    fn rejects_language_update_options_on_other_commands() {
        assert!(Cli::try_parse_from(["aulua", "build", "--prune"]).is_err());
        assert!(Cli::try_parse_from(["aulua", "pack", "--output", "English.aul2"]).is_err());
    }
}
