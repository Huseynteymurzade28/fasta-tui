//! Command-line interface definition.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Visualize a FASTA DNA sequence in the terminal")]
pub struct Cli {
    /// Path to the .fasta / .fa file to open.
    pub file: PathBuf,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
