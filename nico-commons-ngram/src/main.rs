mod learn;
mod ngram;
mod model;

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
    /// Predict
    Predict {
        /// Title to classify
        title: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Learn => learn::learn()?,
        Commands::Predict { title } => learn::predict(title)?,
    }
    Ok(())
}
