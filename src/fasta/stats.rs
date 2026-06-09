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

    /// As `[(label, count); 4]` for bar-chart rendering.
    pub fn as_pairs(&self) -> [(&'static str, u64); 4] {
        [
            ("A", self.a as u64),
            ("C", self.c as u64),
            ("G", self.g as u64),
            ("T", self.t as u64),
        ]
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
}
