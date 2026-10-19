mod bin_parser;
mod error;
mod extractor;
mod file_table;
mod formats;
mod verification;
mod cli;

use cli::{run, Cli};
use clap::Parser;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}