//! Streaming FASTA parser supporting multi-record files.

use std::fs;
use std::io;
use std::path::Path;

use super::record::Record;

/// Parse a FASTA file into one [`Record`] per `>` header.
///
/// Header (`>`) lines start a new record; every following line contributes its
/// `A`/`T`/`G`/`C` bytes (case-insensitively) until the next header. Whitespace
/// and ambiguity codes are ignored. Sequence data before any header is gathered
/// into an anonymous record so raw, header-less files still work.
pub fn parse_file(path: &Path) -> io::Result<Vec<Record>> {
    let raw = fs::read_to_string(path)?;
    Ok(parse_str(&raw))
}

/// Parse FASTA text already held in memory.
pub fn parse_str(text: &str) -> Vec<Record> {
    let mut records: Vec<Record> = Vec::new();

    for line in text.lines() {
        if let Some(header) = line.strip_prefix('>') {
            records.push(Record::new(header));
            continue;
        }

        // Sequence line — ensure we have a record to append to.
        if records.is_empty() {
            records.push(Record::new("unnamed"));
        }
        let current = records.last_mut().expect("record present");
        for ch in line.bytes() {
            match ch.to_ascii_uppercase() {
                b @ (b'A' | b'T' | b'G' | b'C') => current.bases.push(b),
                _ => {}
            }
        }
    }

    // Drop a leading anonymous record if it never received any bases.
    records.retain(|r| !r.is_empty());
    if records.is_empty() {
        records.push(Record::new("empty"));
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_multiple_records_and_cleans() {
        let recs = parse_str(">a desc one\nATGC\nat gc\n>b\nTTTT\n");
        assert_eq!(recs.len(), 2);
        assert_eq!(recs[0].id, "a");
        assert_eq!(recs[0].description, "desc one");
        assert_eq!(recs[0].bases, b"ATGCATGC"); // whitespace ignored, lowercase upper-cased
        assert_eq!(recs[1].id, "b");
        assert_eq!(recs[1].bases, b"TTTT");
    }

    #[test]
    fn headerless_input_still_parses() {
        let recs = parse_str("ATGCATGC\n");
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].bases, b"ATGCATGC");
    }
}
