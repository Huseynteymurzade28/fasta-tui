//! Open reading frame detection across all six frames.
//!
//! An ORF here is an `ATG` … stop-codon run in one frame of one strand. The
//! scan is *outermost-start*: nested `ATG`s inside an open ORF do not spawn
//! extra entries, so each stop codon terminates at most one reported ORF.

use crate::fasta::translate::codon_to_aa;
use crate::fasta::Record;

/// Which strand an ORF was found on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strand {
    Forward,
    Reverse,
}

impl Strand {
    pub fn symbol(self) -> char {
        match self {
            Strand::Forward => '+',
            Strand::Reverse => '-',
        }
    }
}

/// One open reading frame. `start`/`end` are always **forward-strand**
/// coordinates with `start < end`, so the reader can highlight them directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orf {
    pub strand: Strand,
    /// Reading frame (0/1/2) on the ORF's own strand.
    pub frame: usize,
    /// Forward-strand index of the first base, inclusive.
    pub start: usize,
    /// Forward-strand index one past the last base, exclusive. Includes the
    /// stop codon when there is one.
    pub end: usize,
    /// Amino acids encoded, excluding the stop codon.
    pub aa_len: usize,
    /// False when the ORF ran off the end of the sequence without a stop.
    pub has_stop: bool,
}

impl Orf {
    /// Length in nucleotides, stop codon included.
    pub fn nt_len(&self) -> usize {
        self.end - self.start
    }

    /// The ORF's own bases, read 5'→3' along its strand.
    pub fn bases(&self, record: &[u8]) -> Vec<u8> {
        let slice = &record[self.start.min(record.len())..self.end.min(record.len())];
        match self.strand {
            Strand::Forward => slice.to_vec(),
            Strand::Reverse => slice.iter().rev().map(|&b| Record::complement(b)).collect(),
        }
    }

    /// The encoded protein, stop codon included as `*`.
    pub fn protein(&self, record: &[u8]) -> String {
        self.bases(record)
            .chunks_exact(3)
            .map(codon_to_aa)
            .collect()
    }

    /// `"+1"` / `"-2"` style strand-and-frame label.
    pub fn frame_label(&self) -> String {
        format!("{}{}", self.strand.symbol(), self.frame + 1)
    }
}

/// Find every ORF of at least `min_aa` amino acids across all six frames,
/// sorted longest first.
pub fn find_orfs(bases: &[u8], min_aa: usize) -> Vec<Orf> {
    let mut orfs = Vec::new();
    let n = bases.len();

    scan_strand(bases, Strand::Forward, n, min_aa, &mut orfs);
    let rev: Vec<u8> = bases.iter().rev().map(|&b| Record::complement(b)).collect();
    scan_strand(&rev, Strand::Reverse, n, min_aa, &mut orfs);

    // Longest first; ties broken by position so the order is stable.
    orfs.sort_by(|a, b| b.aa_len.cmp(&a.aa_len).then(a.start.cmp(&b.start)));
    orfs
}

/// Scan the three frames of one already-oriented strand. `total` is the record
/// length, used to map reverse-strand offsets back to forward coordinates.
fn scan_strand(seq: &[u8], strand: Strand, total: usize, min_aa: usize, out: &mut Vec<Orf>) {
    for frame in 0..3 {
        let mut open: Option<usize> = None; // start offset on this strand
        let mut i = frame;
        while i + 3 <= seq.len() {
            let codon = &seq[i..i + 3];
            match open {
                None => {
                    if codon == b"ATG" {
                        open = Some(i);
                    }
                }
                Some(start) => {
                    if matches!(codon, b"TAA" | b"TAG" | b"TGA") {
                        push_orf(out, seq, strand, total, frame, start, i + 3, true, min_aa);
                        open = None;
                    }
                }
            }
            i += 3;
        }
        // A run that reaches the end of the strand is still worth reporting;
        // trim it to whole codons so the translation stays in frame.
        if let Some(start) = open {
            let end = start + (seq.len() - start) / 3 * 3;
            push_orf(out, seq, strand, total, frame, start, end, false, min_aa);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_orf(
    out: &mut Vec<Orf>,
    seq: &[u8],
    strand: Strand,
    total: usize,
    frame: usize,
    start: usize,
    end: usize,
    has_stop: bool,
    min_aa: usize,
) {
    debug_assert!(end <= seq.len());
    let codons = (end - start) / 3;
    let aa_len = if has_stop { codons - 1 } else { codons };
    if aa_len < min_aa {
        return;
    }
    // Reverse-strand offset p covers forward index total-1-p, so the interval
    // [start, end) maps to [total-end, total-start).
    let (fwd_start, fwd_end) = match strand {
        Strand::Forward => (start, end),
        Strand::Reverse => (total - end, total - start),
    };
    out.push(Orf {
        strand,
        frame,
        start: fwd_start,
        end: fwd_end,
        aa_len,
        has_stop,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_simple_forward_orf() {
        //              ATG AAA CCC TAA
        let orfs = find_orfs(b"GGATGAAACCCTAAGG", 1);
        let fwd: Vec<_> = orfs.iter().filter(|o| o.strand == Strand::Forward).collect();
        assert_eq!(fwd.len(), 1);
        let orf = fwd[0];
        assert_eq!((orf.start, orf.end), (2, 14));
        assert_eq!(orf.aa_len, 3);
        assert!(orf.has_stop);
        assert_eq!(orf.frame, 2);
        assert_eq!(orf.protein(b"GGATGAAACCCTAAGG"), "MKP*");
    }

    #[test]
    fn reverse_orf_maps_back_to_forward_coordinates() {
        // Reverse complement of ATGAAATAA, so the ORF lives on the minus strand.
        let seq = b"TTATTTCAT";
        let orfs = find_orfs(seq, 1);
        let rev: Vec<_> = orfs.iter().filter(|o| o.strand == Strand::Reverse).collect();
        assert_eq!(rev.len(), 1);
        assert_eq!((rev[0].start, rev[0].end), (0, 9));
        assert_eq!(rev[0].protein(seq), "MK*");
    }

    #[test]
    fn nested_starts_do_not_duplicate() {
        // Two ATGs before one stop → a single outermost ORF.
        let orfs = find_orfs(b"ATGATGAAATAA", 1);
        let fwd: Vec<_> = orfs.iter().filter(|o| o.strand == Strand::Forward).collect();
        assert_eq!(fwd.len(), 1);
        assert_eq!(fwd[0].start, 0);
    }

    #[test]
    fn min_length_filters() {
        assert!(find_orfs(b"ATGAAATAA", 100).is_empty());
    }

    #[test]
    fn unterminated_orf_is_reported_without_stop() {
        let orfs = find_orfs(b"ATGAAACCC", 1);
        let fwd: Vec<_> = orfs.iter().filter(|o| o.strand == Strand::Forward).collect();
        assert_eq!(fwd.len(), 1);
        assert!(!fwd[0].has_stop);
        assert_eq!(fwd[0].aa_len, 3);
    }
}
