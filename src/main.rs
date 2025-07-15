use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
struct Cli {
    #[command(subcommand)]
    command: Commands
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
            println!("ビルドします。");
        },
        Commands::Install => {
            println!("インストールします");
        }
    }
}
