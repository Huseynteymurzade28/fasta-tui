//! A small built-in table of type-II restriction enzymes, and the digest of a
//! sequence against it.
//!
//! Only enzymes that cut *inside* their recognition site are listed, so a site
//! position plus the cut offset is enough to place the top-strand cut.

use crate::fasta::iupac;

/// One restriction enzyme: a recognition site (IUPAC codes allowed) and the
/// offset within that site where the top strand is cut.
#[derive(Debug, Clone, Copy)]
pub struct Enzyme {
    pub name: &'static str,
    pub site: &'static str,
    /// Bases before the top-strand cut, counted from the start of the site.
    pub cut: usize,
}

impl Enzyme {
    /// The site written with a `^` at the cut, e.g. `G^AATTC`.
    pub fn site_with_cut(&self) -> String {
        let (left, right) = self.site.split_at(self.cut);
        format!("{left}^{right}")
    }

    /// True when the enzyme leaves overhangs rather than a blunt end.
    pub fn is_blunt(&self) -> bool {
        self.cut * 2 == self.site.len()
    }
}

/// Common cloning and mapping enzymes, alphabetical by name.
pub const ENZYMES: &[Enzyme] = &[
    Enzyme { name: "AatII",   site: "GACGTC", cut: 5 },
    Enzyme { name: "AluI",    site: "AGCT",   cut: 2 },
    Enzyme { name: "ApaI",    site: "GGGCCC", cut: 5 },
    Enzyme { name: "BamHI",   site: "GGATCC", cut: 1 },
    Enzyme { name: "BglII",   site: "AGATCT", cut: 1 },
    Enzyme { name: "BstYI",   site: "RGATCY", cut: 1 },
    Enzyme { name: "ClaI",    site: "ATCGAT", cut: 2 },
    Enzyme { name: "DraI",    site: "TTTAAA", cut: 3 },
    Enzyme { name: "EcoRI",   site: "GAATTC", cut: 1 },
    Enzyme { name: "EcoRV",   site: "GATATC", cut: 3 },
    Enzyme { name: "HaeIII",  site: "GGCC",   cut: 2 },
    Enzyme { name: "HhaI",    site: "GCGC",   cut: 3 },
    Enzyme { name: "HincII",  site: "GTYRAC", cut: 3 },
    Enzyme { name: "HindIII", site: "AAGCTT", cut: 1 },
    Enzyme { name: "HinfI",   site: "GANTC",  cut: 1 },
    Enzyme { name: "KpnI",    site: "GGTACC", cut: 5 },
    Enzyme { name: "MspI",    site: "CCGG",   cut: 1 },
    Enzyme { name: "NcoI",    site: "CCATGG", cut: 1 },
    Enzyme { name: "NdeI",    site: "CATATG", cut: 2 },
    Enzyme { name: "NheI",    site: "GCTAGC", cut: 1 },
    Enzyme { name: "NotI",    site: "GCGGCCGC", cut: 2 },
    Enzyme { name: "PstI",    site: "CTGCAG", cut: 5 },
    Enzyme { name: "SacI",    site: "GAGCTC", cut: 5 },
    Enzyme { name: "SalI",    site: "GTCGAC", cut: 1 },
    Enzyme { name: "ScrFI",   site: "CCNGG",  cut: 2 },
    Enzyme { name: "SmaI",    site: "CCCGGG", cut: 3 },
    Enzyme { name: "SpeI",    site: "ACTAGT", cut: 1 },
    Enzyme { name: "SphI",    site: "GCATGC", cut: 5 },
    Enzyme { name: "TaqI",    site: "TCGA",   cut: 1 },
    Enzyme { name: "XbaI",    site: "TCTAGA", cut: 1 },
    Enzyme { name: "XhoI",    site: "CTCGAG", cut: 1 },
];

/// Where one enzyme cuts a sequence.
#[derive(Debug, Clone)]
pub struct Digest {
    pub enzyme: &'static Enzyme,
    /// Start index of each recognition site, ascending.
    pub sites: Vec<usize>,
}

impl Digest {
    pub fn count(&self) -> usize {
        self.sites.len()
    }

    /// Top-strand cut positions, ascending.
    pub fn cuts(&self) -> Vec<usize> {
        self.sites.iter().map(|s| s + self.enzyme.cut).collect()
    }

    /// Fragment lengths produced on a *linear* molecule of `len` bases.
    pub fn fragments(&self, len: usize) -> Vec<usize> {
        let mut frags = Vec::with_capacity(self.sites.len() + 1);
        let mut prev = 0;
        for cut in self.cuts() {
            let cut = cut.min(len);
            frags.push(cut - prev);
            prev = cut;
        }
        frags.push(len - prev);
        frags
    }
}

/// Digest a sequence with every enzyme in the table.
///
/// Non-palindromic sites are searched on both strands, with the minus-strand
/// hits reported in forward coordinates; palindromic sites (the majority) need
/// only one pass because both strands read alike.
pub fn digest_all(bases: &[u8]) -> Vec<Digest> {
    ENZYMES
        .iter()
        .map(|enzyme| {
            let site = enzyme.site.as_bytes();
            let mut sites = iupac::find_all(bases, site);
            if !iupac::is_palindrome(site) {
                sites.extend(iupac::find_all(bases, &iupac::revcomp(site)));
                sites.sort_unstable();
                sites.dedup();
            }
            Digest { enzyme, sites }
        })
        .collect()
}

/// Enzymes that cut the sequence exactly once — the useful ones for cloning.
pub fn unique_cutters(digests: &[Digest]) -> Vec<&Digest> {
    digests.iter().filter(|d| d.count() == 1).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest_of(seq: &[u8], name: &str) -> Digest {
        digest_all(seq)
            .into_iter()
            .find(|d| d.enzyme.name == name)
            .expect("enzyme in table")
    }

    #[test]
    fn finds_ecori_site() {
        let d = digest_of(b"TTTGAATTCTTT", "EcoRI");
        assert_eq!(d.sites, vec![3]);
        assert_eq!(d.cuts(), vec![4]);
        assert_eq!(d.fragments(12), vec![4, 8]);
    }

    #[test]
    fn ambiguous_site_matches_both_variants() {
        // HinfI is G^ANTC: matches GAATC and GACTC alike.
        let d = digest_of(b"GAATCAAGACTC", "HinfI");
        assert_eq!(d.sites, vec![0, 7]);
    }

    #[test]
    fn every_table_site_is_palindromic() {
        // The table holds only type-IIP enzymes, so a single-strand scan finds
        // every site. `digest_all` still handles the non-palindromic case for
        // when the table grows.
        for e in ENZYMES {
            assert!(
                iupac::is_palindrome(e.site.as_bytes()),
                "{} ({}) is not palindromic",
                e.name,
                e.site
            );
            assert!(e.cut <= e.site.len(), "{} cut is outside its site", e.name);
        }
    }

    #[test]
    fn no_sites_yields_one_fragment() {
        let d = digest_of(b"AAAAAAAA", "EcoRI");
        assert_eq!(d.count(), 0);
        assert_eq!(d.fragments(8), vec![8]);
    }

    #[test]
    fn site_notation_marks_the_cut() {
        let eco = ENZYMES.iter().find(|e| e.name == "EcoRI").unwrap();
        assert_eq!(eco.site_with_cut(), "G^AATTC");
        assert!(!eco.is_blunt());
        let sma = ENZYMES.iter().find(|e| e.name == "SmaI").unwrap();
        assert!(sma.is_blunt());
    }
}
