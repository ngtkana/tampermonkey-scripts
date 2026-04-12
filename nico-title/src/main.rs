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
        Some(Commands::Download) => download::download(),
        None => println!("No command specified. Use --help for usage information."),
    }
}
