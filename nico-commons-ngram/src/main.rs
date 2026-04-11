mod learn;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Learn
    Learn,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Learn => learn::learn(),
    }
}
