mod download;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// does testing things
    Download,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Download => download::download(),
    }
}
