//! Physical properties of a short stretch of DNA: melting temperature,
//! molecular weight and composition — the numbers you want when a selection in
//! the reader is a candidate primer.

/// Primer concentration assumed by the nearest-neighbour model (0.5 µM).
const OLIGO_CONC: f64 = 0.5e-6;
/// Monovalent salt concentration assumed by the salt correction (50 mM Na⁺).
const NA_CONC: f64 = 0.05;
/// Gas constant, cal/(mol·K).
const R: f64 = 1.987;

/// Everything the selection panel reports about a stretch of sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OligoStats {
    pub len: usize,
    /// GC fraction over unambiguous bases, `0.0..=1.0`.
    pub gc: f64,
    /// Wallace ("2+4") rule — a rough guide, best under ~20 nt.
    pub tm_wallace: f64,
    /// Nearest-neighbour Tm (SantaLucia 1998) in °C. `None` when the sequence
    /// is shorter than two bases or contains ambiguity codes.
    pub tm_nn: Option<f64>,
    /// Single-stranded molecular weight in daltons.
    pub mw: f64,
    /// Bases that are not A/C/G/T.
    pub ambiguous: usize,
}

/// Nearest-neighbour ΔH (kcal/mol) and ΔS (cal/mol·K) for a dinucleotide,
/// SantaLucia (1998) unified parameters.
fn nn_params(pair: &[u8]) -> Option<(f64, f64)> {
    let p = match pair {
        b"AA" | b"TT" => (-7.9, -22.2),
        b"AT" => (-7.2, -20.4),
        b"TA" => (-7.2, -21.3),
        b"CA" | b"TG" => (-8.5, -22.7),
        b"GT" | b"AC" => (-8.4, -22.4),
        b"CT" | b"AG" => (-7.8, -21.0),
        b"GA" | b"TC" => (-8.2, -22.2),
        b"CG" => (-10.6, -27.2),
        b"GC" => (-9.8, -24.4),
        b"GG" | b"CC" => (-8.0, -19.9),
        _ => return None,
    };
    Some(p)
}

/// Initiation ΔH/ΔS for one helix end, which differs for A·T and G·C.
fn init_params(end: u8) -> (f64, f64) {
    match end {
        b'G' | b'C' => (0.1, -2.8),
        _ => (2.3, 4.1),
    }
}

/// Nearest-neighbour melting temperature in °C, or `None` for sequences that
/// are too short or contain codes the model has no parameters for.
pub fn tm_nearest_neighbour(bases: &[u8]) -> Option<f64> {
    if bases.len() < 2 {
        return None;
    }
    let (mut dh, mut ds) = (0.0, 0.0);
    for pair in bases.windows(2) {
        let (h, s) = nn_params(pair)?;
        dh += h;
        ds += s;
    }
    for &end in &[bases[0], bases[bases.len() - 1]] {
        if !matches!(end, b'A' | b'C' | b'G' | b'T') {
            return None;
        }
        let (h, s) = init_params(end);
        dh += h;
        ds += s;
    }

    // SantaLucia's entropic salt correction, then Tm = ΔH / (ΔS + R·ln(Ct/4)).
    ds += 0.368 * (bases.len() - 1) as f64 * NA_CONC.ln();
    let denom = ds + R * (OLIGO_CONC / 4.0).ln();
    if denom == 0.0 {
        return None;
    }
    Some(dh * 1000.0 / denom - 273.15)
}

/// The Wallace rule: `2·(A+T) + 4·(G+C)` below 14 nt, the GC formula above.
pub fn tm_wallace(bases: &[u8]) -> f64 {
    let (mut at, mut gc) = (0usize, 0usize);
    for &b in bases {
        match b {
            b'A' | b'T' | b'U' => at += 1,
            b'G' | b'C' => gc += 1,
            _ => {}
        }
    }
    let n = at + gc;
    if n == 0 {
        return 0.0;
    }
    if n < 14 {
        (2 * at + 4 * gc) as f64
    } else {
        64.9 + 41.0 * (gc as f64 - 16.4) / n as f64
    }
}

/// Single-stranded molecular weight in daltons (anhydrous, 5'-OH).
pub fn molecular_weight(bases: &[u8]) -> f64 {
    if bases.is_empty() {
        return 0.0;
    }
    let mut mw = 0.0;
    for &b in bases {
        mw += match b {
            b'A' => 313.21,
            b'C' => 289.18,
            b'G' => 329.21,
            b'T' | b'U' => 304.20,
            // Ambiguity codes contribute the average of the four bases.
            _ => 308.95,
        };
    }
    mw - 61.96
}

/// Analyse a stretch of sequence for the selection panel.
pub fn analyze(bases: &[u8]) -> OligoStats {
    let gc_n = bases.iter().filter(|&&b| b == b'G' || b == b'C').count();
    let known = bases
        .iter()
        .filter(|&&b| matches!(b, b'A' | b'C' | b'G' | b'T' | b'U'))
        .count();
    OligoStats {
        len: bases.len(),
        gc: if known == 0 {
            0.0
        } else {
            gc_n as f64 / known as f64
        },
        tm_wallace: tm_wallace(bases),
        tm_nn: tm_nearest_neighbour(bases),
        mw: molecular_weight(bases),
        ambiguous: bases.len() - known,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallace_matches_the_hand_rule() {
        // 4 bases: 2·(A+T) + 4·(G+C) = 2·2 + 4·2 = 12
        assert!((tm_wallace(b"ATGC") - 12.0).abs() < 1e-9);
        assert_eq!(tm_wallace(b""), 0.0);
    }

    #[test]
    fn nearest_neighbour_lands_in_a_sane_range() {
        // A typical 20-mer primer melts somewhere around 50–65 °C.
        let tm = tm_nearest_neighbour(b"ATGCGTACGTTAGCCTAGCA").expect("pure ACGT");
        assert!((45.0..70.0).contains(&tm), "tm was {tm}");
        // GC-rich melts hotter than AT-rich at equal length.
        let gc_rich = tm_nearest_neighbour(b"GCGCGCGCGCGCGCGCGCGC").unwrap();
        let at_rich = tm_nearest_neighbour(b"ATATATATATATATATATAT").unwrap();
        assert!(gc_rich > at_rich);
    }

    #[test]
    fn nearest_neighbour_declines_ambiguous_or_tiny_input() {
        assert!(tm_nearest_neighbour(b"A").is_none());
        assert!(tm_nearest_neighbour(b"ATGNCC").is_none());
    }

    #[test]
    fn analyze_reports_composition() {
        let s = analyze(b"GGCCAT");
        assert!((s.gc - 4.0 / 6.0).abs() < 1e-9);
        assert_eq!(s.len, 6);
        assert_eq!(s.ambiguous, 0);
        assert!(s.mw > 1500.0);

        let amb = analyze(b"GGNN");
        assert_eq!(amb.ambiguous, 2);
        assert!(amb.tm_nn.is_none());
    }
}
