mod download;
mod analyze;
mod extract;
mod annotate;
mod compare;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Download Vocaloid song titles from NicoVideo API
    Download,
    /// Analyze title patterns in the dataset
    Analyze,
    /// Extract song titles using rule-based approach
    Extract,
    /// Generate annotations using Claude API
    #[command(name = "annotate")]
    Annotate {
        /// Number of titles to annotate (default: 200)
        #[arg(short, long, default_value = "200")]
        count: usize,
    },
    /// Compare rule-based and LLM extractions
    Compare,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Download) => download::download(),
        Some(Commands::Analyze) => analyze::analyze_patterns(),
        Some(Commands::Extract) => extract_all_titles(),
        Some(Commands::Annotate { count }) => annotate::annotate_titles(count),
        Some(Commands::Compare) => compare::compare_methods(),
        None => println!("No command specified. Use --help for usage information."),
    }
}

fn extract_all_titles() {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};

    let input_file = match File::open("data/nico_api_result.tsv") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open input file: {}", e);
            return;
        }
    };

    let reader = BufReader::new(input_file);
    let mut output_file = match File::create("data/nico_api_extracted.jsonl") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to create output file: {}", e);
            return;
        }
    };

    let mut count = 0;
    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            continue; // Skip header
        }

        match line {
            Ok(l) => {
                let parts: Vec<&str> = l.split('\t').collect();
                if parts.len() >= 2 {
                    let content_id = parts[0];
                    let title = parts[1];
                    let extracted = extract::extract_song_title(title);

                    let json_output = serde_json::json!({
                        "content_id": content_id,
                        "title": title,
                        "extracted_title": extracted
                    });

                    if let Err(e) = writeln!(output_file, "{}", json_output.to_string()) {
                        eprintln!("Failed to write line: {}", e);
                        return;
                    }
                    count += 1;
                }
            }
            Err(e) => {
                eprintln!("Error reading line: {}", e);
            }
        }
    }

    println!("Successfully extracted {} titles to data/nico_api_extracted.jsonl", count);
}
