//! FASTA domain layer: parsing, the record model, statistics, translation,
//! ORF detection, restriction mapping and oligo thermodynamics.

pub mod enzymes;
pub mod iupac;
pub mod oligo;
pub mod orf;
pub mod parse;
pub mod record;
pub mod stats;
pub mod translate;

pub use orf::Strand;
pub use parse::parse_file;
pub use record::Record;
