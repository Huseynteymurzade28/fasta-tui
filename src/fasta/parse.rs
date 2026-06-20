//! Streaming FASTA parser supporting multi-record files.

use std::fs;
use std::io::{self, Read};
use std::path::Path;

use super::record::Record;

/// Read and parse a FASTA source into one [`Record`] per `>` header.
///
/// The `path` may be:
/// - a regular `.fasta` / `.fa` file,
/// - a gzip-compressed file (detected by magic bytes, so the `.gz` extension is
///   optional), or
/// - `-`, meaning read from standard input (also gzip-aware).
///
/// Header (`>`) lines start a new record; every following line contributes its
/// nucleotide bytes (case-insensitively) until the next header. `U`/`u` is read
/// as `T` so RNA is accepted, IUPAC ambiguity codes (`N`, `R`, `Y`, …) are kept,
/// and whitespace, gaps and digits are ignored. Sequence data before any header
/// is gathered into an anonymous record so raw, header-less files still work.
pub fn parse_file(path: &Path) -> io::Result<Vec<Record>> {
    let bytes = if path == Path::new("-") {
        let mut buf = Vec::new();
        io::stdin().lock().read_to_end(&mut buf)?;
        buf
    } else {
        fs::read(path)?
    };
    let bytes = maybe_gunzip(bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(parse_str(&text))
}

/// Transparently decompress gzip input, leaving plain text untouched. Gzip is
/// recognized by its two magic bytes rather than the file name.
fn maybe_gunzip(bytes: Vec<u8>) -> io::Result<Vec<u8>> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(&bytes[..]).read_to_end(&mut out)?;
        Ok(out)
    } else {
        Ok(bytes)
    }
}

/// Normalize a raw sequence byte to a stored base, or `None` to drop it.
///
/// Uppercases the byte, maps `U` (RNA) to `T`, and keeps the IUPAC nucleotide
/// alphabet — the four bases plus ambiguity codes. Everything else (whitespace,
/// gaps, digits, stray punctuation) is discarded.
fn clean_base(byte: u8) -> Option<u8> {
    match byte.to_ascii_uppercase() {
        b'U' => Some(b'T'),
        b @ (b'A' | b'T' | b'G' | b'C' | b'N' | b'R' | b'Y' | b'S' | b'W' | b'K' | b'M'
        | b'B' | b'D' | b'H' | b'V') => Some(b),
        _ => None,
    }
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
        current.bases.extend(line.bytes().filter_map(clean_base));
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

    #[test]
    fn rna_is_read_as_dna_and_ambiguity_kept() {
        // U -> T, ambiguity codes preserved, gaps/digits dropped.
        let recs = parse_str(">r\nAUGN-RYK..123\n");
        assert_eq!(recs[0].bases, b"ATGNRYK");
    }
}
