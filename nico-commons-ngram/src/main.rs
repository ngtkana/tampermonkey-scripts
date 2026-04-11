mod learn;
mod model;
mod ngram;
mod classifier;

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
    /// Export pruned model as JavaScript
    Export {
        /// Weight threshold for pruning
        #[arg(short, long, default_value = "0.05")]
        threshold: f64,
        /// Output file path
        #[arg(short, long, default_value = "annotate/model.js")]
        output: String,
    },
    /// Compare rule-based vs neural model
    Compare,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Learn => learn::learn()?,
        Commands::Predict { title } => learn::predict(title)?,
        Commands::Tune => learn::tune()?,
        Commands::CrossVal { k } => learn::cross_validate(*k)?,
        Commands::Export { threshold, output } => learn::export(*threshold, output)?,
        Commands::Compare => learn::compare()?,
    }
    Ok(())
}
