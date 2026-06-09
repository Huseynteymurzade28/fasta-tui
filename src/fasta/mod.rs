//! FASTA domain layer: parsing, the record model, statistics and translation.

pub mod parse;
pub mod record;
pub mod stats;
pub mod translate;

pub use parse::parse_file;
pub use record::Record;
