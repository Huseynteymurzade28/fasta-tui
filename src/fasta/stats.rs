//! Compositional statistics over a nucleotide sequence.

/// Counts of A, C, G, T (in that order).
#[derive(Debug, Clone, Copy, Default)]
pub struct BaseCounts {
    pub a: usize,
    pub c: usize,
    pub g: usize,
    pub t: usize,
}

impl BaseCounts {
    pub fn total(&self) -> usize {
        self.a + self.c + self.g + self.t
    }

}

/// Tally each base in the sequence.
pub fn base_counts(bases: &[u8]) -> BaseCounts {
    let mut counts = BaseCounts::default();
    for &b in bases {
        match b {
            b'A' => counts.a += 1,
            b'C' => counts.c += 1,
            b'G' => counts.g += 1,
            b'T' => counts.t += 1,
            _ => {}
        }
    }
    counts
}

/// Overall GC content in the range `0.0..=1.0`.
pub fn gc_content(bases: &[u8]) -> f64 {
    if bases.is_empty() {
        return 0.0;
    }
    let gc = bases.iter().filter(|&&b| b == b'G' || b == b'C').count();
    gc as f64 / bases.len() as f64
}

/// GC content sampled over a sliding window, returned as `(position, percent)`
/// points suitable for a line chart. `window` is clamped to the sequence size.
pub fn gc_sliding_window(bases: &[u8], window: usize, samples: usize) -> Vec<(f64, f64)> {
    if bases.is_empty() || samples == 0 {
        return Vec::new();
    }
    let window = window.clamp(1, bases.len());
    let last_start = bases.len().saturating_sub(window);

    // Prefix sums of GC counts for O(1) window queries.
    let mut prefix = Vec::with_capacity(bases.len() + 1);
    prefix.push(0usize);
    for &b in bases {
        let inc = (b == b'G' || b == b'C') as usize;
        prefix.push(prefix.last().unwrap() + inc);
    }

    let mut points = Vec::with_capacity(samples.min(last_start + 1));
    for i in 0..samples {
        let start = if samples == 1 {
            0
        } else {
            last_start * i / (samples - 1)
        };
        let gc = prefix[start + window] - prefix[start];
        let percent = gc as f64 / window as f64 * 100.0;
        points.push((start as f64, percent));
    }
    points
}

/// Prefix sums of G+C counts: `prefix[i]` is the number of G/C bases in
/// `bases[..i]`, so any window's GC count is one subtraction.
pub fn gc_prefix(bases: &[u8]) -> Vec<u32> {
    let mut prefix = Vec::with_capacity(bases.len() + 1);
    let mut acc = 0u32;
    prefix.push(0);
    for &b in bases {
        acc += (b == b'G' || b == b'C') as u32;
        prefix.push(acc);
    }
    prefix
}

/// GC fraction of `bases[lo..hi]` read from a [`gc_prefix`] table.
pub fn gc_between(prefix: &[u32], lo: usize, hi: usize) -> f64 {
    if hi <= lo {
        return 0.0;
    }
    (prefix[hi] - prefix[lo]) as f64 / (hi - lo) as f64
}

/// GC skew `(G - C) / (G + C)` over a sliding window, sampled like
/// [`gc_sliding_window`]. Windows without any G or C score 0.
pub fn gc_skew_window(bases: &[u8], window: usize, samples: usize) -> Vec<(f64, f64)> {
    if bases.is_empty() || samples == 0 {
        return Vec::new();
    }
    let window = window.clamp(1, bases.len());
    let last_start = bases.len() - window;
    let mut g = vec![0i32; bases.len() + 1];
    let mut c = vec![0i32; bases.len() + 1];
    for (i, &b) in bases.iter().enumerate() {
        g[i + 1] = g[i] + (b == b'G') as i32;
        c[i + 1] = c[i] + (b == b'C') as i32;
    }
    (0..samples)
        .map(|i| {
            let start = if samples == 1 { 0 } else { last_start * i / (samples - 1) };
            let gs = g[start + window] - g[start];
            let cs = c[start + window] - c[start];
            let skew = if gs + cs == 0 {
                0.0
            } else {
                (gs - cs) as f64 / (gs + cs) as f64
            };
            (start as f64, skew)
        })
        .collect()
}

/// The four plain bases in the order every table here uses.
pub const BASES: [u8; 4] = [b'T', b'C', b'A', b'G'];

fn base_index(b: u8) -> Option<usize> {
    BASES.iter().position(|&x| x == b)
}

/// Dinucleotide observed/expected ratios, `oe[first][second]` indexed by
/// [`BASES`]. 1.0 means the pair occurs as often as base composition
/// predicts; CpG depletion in vertebrate DNA shows up as `oe[C][G]` ≪ 1.
pub fn dinucleotide_oe(bases: &[u8]) -> [[f64; 4]; 4] {
    let mut mono = [0usize; 4];
    let mut di = [[0usize; 4]; 4];
    for &b in bases {
        if let Some(i) = base_index(b) {
            mono[i] += 1;
        }
    }
    for w in bases.windows(2) {
        if let (Some(i), Some(j)) = (base_index(w[0]), base_index(w[1])) {
            di[i][j] += 1;
        }
    }
    let n: usize = mono.iter().sum();
    let mut oe = [[0.0; 4]; 4];
    if n < 2 {
        return oe;
    }
    for i in 0..4 {
        for j in 0..4 {
            let expected = mono[i] as f64 * mono[j] as f64 / n as f64;
            if expected > 0.0 {
                oe[i][j] = di[i][j] as f64 / expected;
            }
        }
    }
    oe
}

/// Codon counts for one reading frame, indexed `16*first + 4*second + third`
/// over [`BASES`] — the layout of the textbook codon table. Codons containing
/// ambiguity codes are skipped.
pub fn codon_usage(bases: &[u8], frame: usize) -> [usize; 64] {
    let mut counts = [0usize; 64];
    let frame = frame % 3;
    if bases.len() <= frame {
        return counts;
    }
    for codon in bases[frame..].chunks_exact(3) {
        if let (Some(a), Some(b), Some(c)) = (
            base_index(codon[0]),
            base_index(codon[1]),
            base_index(codon[2]),
        ) {
            counts[a * 16 + b * 4 + c] += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gc_content_basic() {
        assert!((gc_content(b"GCGC") - 1.0).abs() < 1e-9);
        assert!((gc_content(b"ATAT") - 0.0).abs() < 1e-9);
        assert!((gc_content(b"ATGC") - 0.5).abs() < 1e-9);
        assert_eq!(gc_content(b""), 0.0);
    }

    #[test]
    fn counts_each_base() {
        let c = base_counts(b"AACGT");
        assert_eq!((c.a, c.c, c.g, c.t), (2, 1, 1, 1));
        assert_eq!(c.total(), 5);
    }

    #[test]
    fn sliding_window_endpoints() {
        let pts = gc_sliding_window(b"GGGGAAAA", 4, 2);
        assert_eq!(pts.len(), 2);
        assert!((pts[0].1 - 100.0).abs() < 1e-9); // first window all GC
        assert!((pts[1].1 - 0.0).abs() < 1e-9);   // last window all AT
    }

    #[test]
    fn prefix_windows() {
        let p = gc_prefix(b"ATGC");
        assert_eq!(p, vec![0, 0, 0, 1, 2]);
        assert!((gc_between(&p, 2, 4) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn skew_sign() {
        let s = gc_skew_window(b"GGGG", 4, 1);
        assert!((s[0].1 - 1.0).abs() < 1e-9);
        let s = gc_skew_window(b"CCCC", 4, 1);
        assert!((s[0].1 + 1.0).abs() < 1e-9);
    }

    #[test]
    fn dinucleotides_flag_missing_pairs() {
        let oe = dinucleotide_oe(b"ACACACACAC");
        let (a, c, g) = (2, 1, 3);
        assert!(oe[a][c] > 1.5);
        assert_eq!(oe[c][g], 0.0);
    }

    #[test]
    fn codon_table_layout() {
        let u = codon_usage(b"TTTATGGGG", 0);
        assert_eq!(u[0], 1); // TTT
        assert_eq!(u[2 * 16 + 3 * 4 + 3], 0); // AGG absent
        assert_eq!(u[2 * 16 + 0 * 4 + 3], 1); // ATG
        assert_eq!(u[63], 1); // GGG
    }
}
