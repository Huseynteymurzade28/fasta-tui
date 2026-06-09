//! A single FASTA record: a header plus its nucleotide sequence.

/// One FASTA entry. Bases are stored as uppercase ASCII (`A`, `T`, `G`, `C`).
#[derive(Debug, Clone)]
pub struct Record {
    /// First whitespace-delimited token of the header (the accession / id).
    pub id: String,
    /// Remainder of the header line, if any.
    pub description: String,
    /// Cleaned sequence: only `A`/`T`/`G`/`C` bytes.
    pub bases: Vec<u8>,
}

impl Record {
    pub fn new(header: &str) -> Self {
        let header = header.trim();
        let (id, description) = match header.split_once(char::is_whitespace) {
            Some((id, rest)) => (id.to_string(), rest.trim().to_string()),
            None => (header.to_string(), String::new()),
        };
        Record {
            id,
            description,
            bases: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.bases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    /// The base at `index`, or a space when out of range (used by the helix).
    pub fn base_at(&self, index: usize) -> char {
        self.bases.get(index).map(|&b| b as char).unwrap_or(' ')
    }

    /// Watson–Crick complement of a single base byte.
    pub fn complement(base: u8) -> u8 {
        match base {
            b'A' => b'T',
            b'T' => b'A',
            b'G' => b'C',
            b'C' => b'G',
            other => other,
        }
    }

    /// The reverse-complement strand (read 3'→5').
    pub fn reverse_complement(&self) -> Vec<u8> {
        self.bases
            .iter()
            .rev()
            .map(|&b| Self::complement(b))
            .collect()
    }

    /// Return the bases for the requested orientation without copying when
    /// possible: forward borrows, reverse allocates.
    pub fn oriented(&self, reverse: bool) -> std::borrow::Cow<'_, [u8]> {
        if reverse {
            std::borrow::Cow::Owned(self.reverse_complement())
        } else {
            std::borrow::Cow::Borrowed(&self.bases)
        }
    }
}
