//! Translation of nucleotide codons into amino acids (standard genetic code).

/// Translate a single 3-base codon into its one-letter amino-acid code.
/// Returns `*` for stop codons and `X` for anything unrecognized.
pub fn codon_to_aa(codon: &[u8]) -> char {
    match codon {
        b"TTT" | b"TTC" => 'F',
        b"TTA" | b"TTG" | b"CTT" | b"CTC" | b"CTA" | b"CTG" => 'L',
        b"ATT" | b"ATC" | b"ATA" => 'I',
        b"ATG" => 'M',
        b"GTT" | b"GTC" | b"GTA" | b"GTG" => 'V',
        b"TCT" | b"TCC" | b"TCA" | b"TCG" | b"AGT" | b"AGC" => 'S',
        b"CCT" | b"CCC" | b"CCA" | b"CCG" => 'P',
        b"ACT" | b"ACC" | b"ACA" | b"ACG" => 'T',
        b"GCT" | b"GCC" | b"GCA" | b"GCG" => 'A',
        b"TAT" | b"TAC" => 'Y',
        b"TAA" | b"TAG" | b"TGA" => '*', // stop
        b"CAT" | b"CAC" => 'H',
        b"CAA" | b"CAG" => 'Q',
        b"AAT" | b"AAC" => 'N',
        b"AAA" | b"AAG" => 'K',
        b"GAT" | b"GAC" => 'D',
        b"GAA" | b"GAG" => 'E',
        b"TGT" | b"TGC" => 'C',
        b"TGG" => 'W',
        b"CGT" | b"CGC" | b"CGA" | b"CGG" | b"AGA" | b"AGG" => 'R',
        b"GGT" | b"GGC" | b"GGA" | b"GGG" => 'G',
        _ => 'X',
    }
}

/// Translate an entire strand starting at reading `frame` (0, 1 or 2).
pub fn translate(bases: &[u8], frame: usize) -> Vec<char> {
    let frame = frame % 3;
    bases[frame.min(bases.len())..]
        .chunks_exact(3)
        .map(codon_to_aa)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_start_and_stop() {
        assert_eq!(codon_to_aa(b"ATG"), 'M');
        assert_eq!(codon_to_aa(b"TAA"), '*');
        assert_eq!(codon_to_aa(b"TGA"), '*');
        assert_eq!(codon_to_aa(b"GGG"), 'G');
        assert_eq!(codon_to_aa(b"NNN"), 'X');
    }

    #[test]
    fn frame_offset_shifts_reading() {
        // ATGTAA -> M*  ; frame 1 reads TGT,AA(incomplete) -> C
        assert_eq!(translate(b"ATGTAA", 0), vec!['M', '*']);
        assert_eq!(translate(b"ATGTAA", 1), vec!['C']);
    }
}
