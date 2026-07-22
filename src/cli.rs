//! Command-line interface definition.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Visualize a FASTA DNA sequence in the terminal")]
pub struct Cli {
    /// Path to the .fasta / .fa file to open. Gzip (.gz) is detected
    /// automatically; use `-` to read from standard input.
    pub file: PathBuf,

    /// Translate every record to protein and write FASTA to this path, then
    /// exit without opening the TUI. Use `-` to write to standard output.
    #[arg(long, value_name = "FILE")]
    pub export_protein: Option<PathBuf>,

    /// Reading frame (0, 1 or 2) used when translating for --export-protein.
    #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u8).range(0..=2))]
    pub frame: u8,

    /// Translate the reverse-complement strand for --export-protein.
    #[arg(long)]
    pub reverse: bool,

    /// Find ORFs in all six frames and write them as a protein FASTA file,
    /// then exit without opening the TUI. Use `-` for standard output.
    #[arg(long, value_name = "FILE")]
    pub export_orfs: Option<PathBuf>,

    /// Shortest ORF to report, in amino acids.
    #[arg(long, default_value_t = crate::app::DEFAULT_MIN_ORF_AA, value_name = "AA")]
    pub min_orf: usize,

    /// Print a restriction map (one TSV row per enzyme) and exit.
    #[arg(long)]
    pub restriction_map: bool,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
