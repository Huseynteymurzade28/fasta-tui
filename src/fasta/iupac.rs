//! IUPAC-aware nucleotide pattern matching.
//!
//! Every IUPAC code stands for a *set* of bases, encoded here as a 4-bit mask.
//! A pattern byte matches a sequence byte when the two sets overlap, so `N`
//! matches anything, `R` (A/G) matches an `A`, and a sequence `N` is matched by
//! any pattern base.

use crate::fasta::Record;

const A: u8 = 0b0001;
const C: u8 = 0b0010;
const G: u8 = 0b0100;
const T: u8 = 0b1000;

/// The set of concrete bases an IUPAC code stands for, or `0` if unknown.
pub fn mask(code: u8) -> u8 {
    match code.to_ascii_uppercase() {
        b'A' => A,
        b'C' => C,
        b'G' => G,
        b'T' | b'U' => T,
        b'R' => A | G,
        b'Y' => C | T,
        b'S' => G | C,
        b'W' => A | T,
        b'K' => G | T,
        b'M' => A | C,
        b'B' => C | G | T,
        b'D' => A | G | T,
        b'H' => A | C | T,
        b'V' => A | C | G,
        b'N' => A | C | G | T,
        _ => 0,
    }
}

/// True when `code` is a nucleotide or ambiguity code we understand.
pub fn is_code(code: u8) -> bool {
    mask(code) != 0
}

/// True when a pattern byte and a sequence byte can denote the same base.
pub fn matches(pattern: u8, base: u8) -> bool {
    mask(pattern) & mask(base) != 0
}

/// Start indices of every place `pattern` matches inside `hay`.
pub fn find_all(hay: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() || pattern.len() > hay.len() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    'outer: for start in 0..=hay.len() - pattern.len() {
        for (k, &p) in pattern.iter().enumerate() {
            if !matches(p, hay[start + k]) {
                continue 'outer;
            }
        }
        hits.push(start);
    }
    hits
}

/// The reverse complement of a pattern, ambiguity codes included.
pub fn revcomp(pattern: &[u8]) -> Vec<u8> {
    pattern
        .iter()
        .rev()
        .map(|&b| Record::complement(b))
        .collect()
}

/// True when a pattern reads the same on both strands (most type-II
/// restriction sites are palindromic, so searching one strand is enough).
pub fn is_palindrome(pattern: &[u8]) -> bool {
    revcomp(pattern) == pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambiguity_codes_overlap() {
        assert!(matches(b'N', b'A'));
        assert!(matches(b'R', b'G'));
        assert!(!matches(b'R', b'C'));
        assert!(matches(b'A', b'N')); // an N in the sequence matches anything
        assert!(!matches(b'A', b'C'));
    }

    #[test]
    fn finds_ambiguous_pattern() {
        //            0123456789
        let hay = b"GAATCGACTC";
        // G-A-N-T-C matches GAATC at 0 and GACTC at 5.
        assert_eq!(find_all(hay, b"GANTC"), vec![0, 5]);
        assert_eq!(find_all(hay, b"GAATC"), vec![0]);
        assert!(find_all(hay, b"").is_empty());
    }

    #[test]
    fn palindromes() {
        assert!(is_palindrome(b"GAATTC")); // EcoRI
        assert!(is_palindrome(b"GTYRAC")); // HincII, ambiguous but palindromic
        assert!(is_palindrome(b"GANTC")); // N is self-complementary, so this is too
        assert!(!is_palindrome(b"ATG"));
        assert_eq!(revcomp(b"ATG"), b"CAT".to_vec());
        assert_eq!(revcomp(b"GGATCC"), b"GGATCC".to_vec());
    }
}
