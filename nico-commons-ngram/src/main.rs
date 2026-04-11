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
    /// Grid search for hyperparameters
    Tune,
    /// Cross-validate with k-fold
    CrossVal {
        /// Number of folds (default: 5)
        #[arg(short, long, default_value = "5")]
        k: usize,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Learn => learn::learn()?,
        Commands::Predict { title } => learn::predict(title)?,
        Commands::Tune => learn::tune()?,
        Commands::CrossVal { k } => learn::cross_validate(*k)?,
    }
    Ok(())
}
