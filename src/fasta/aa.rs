//! Amino-acid properties: physicochemical classes and hydropathy.

/// Coarse physicochemical grouping used to color protein sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AaClass {
    /// Aliphatic and sulfur-containing side chains: A V L I M C.
    Hydrophobic,
    /// Ring side chains: F W Y.
    Aromatic,
    /// Uncharged polar: S T N Q.
    Polar,
    /// Basic: K R H.
    Positive,
    /// Acidic: D E.
    Negative,
    /// Backbone-shaping oddities: G P.
    Special,
    Stop,
    Unknown,
}

impl AaClass {
    pub const LEGEND: [AaClass; 6] = [
        AaClass::Hydrophobic,
        AaClass::Aromatic,
        AaClass::Polar,
        AaClass::Positive,
        AaClass::Negative,
        AaClass::Special,
    ];

    pub fn of(aa: char) -> Self {
        match aa {
            'A' | 'V' | 'L' | 'I' | 'M' | 'C' => AaClass::Hydrophobic,
            'F' | 'W' | 'Y' => AaClass::Aromatic,
            'S' | 'T' | 'N' | 'Q' => AaClass::Polar,
            'K' | 'R' | 'H' => AaClass::Positive,
            'D' | 'E' => AaClass::Negative,
            'G' | 'P' => AaClass::Special,
            '*' => AaClass::Stop,
            _ => AaClass::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AaClass::Hydrophobic => "hydrophobic",
            AaClass::Aromatic => "aromatic",
            AaClass::Polar => "polar",
            AaClass::Positive => "basic +",
            AaClass::Negative => "acidic −",
            AaClass::Special => "G/P",
            AaClass::Stop => "stop",
            AaClass::Unknown => "unknown",
        }
    }
}

/// Kyte–Doolittle hydropathy index (1982). Unknown residues and stops score 0.
pub fn hydropathy(aa: char) -> f64 {
    match aa {
        'I' => 4.5,
        'V' => 4.2,
        'L' => 3.8,
        'F' => 2.8,
        'C' => 2.5,
        'M' => 1.9,
        'A' => 1.8,
        'G' => -0.4,
        'T' => -0.7,
        'S' => -0.8,
        'W' => -0.9,
        'Y' => -1.3,
        'P' => -1.6,
        'H' => -3.2,
        'E' | 'Q' | 'D' | 'N' => -3.5,
        'K' => -3.9,
        'R' => -4.5,
        _ => 0.0,
    }
}

/// Mean hydropathy over a centred sliding window, one `(index, score)` point
/// per residue that has a full window around it.
pub fn hydropathy_profile(residues: &[char], window: usize) -> Vec<(f64, f64)> {
    let window = window.max(1);
    if residues.len() < window {
        return Vec::new();
    }
    let mut sum: f64 = residues[..window].iter().map(|&a| hydropathy(a)).sum();
    let half = window / 2;
    let mut points = Vec::with_capacity(residues.len() - window + 1);
    points.push((half as f64, sum / window as f64));
    for i in window..residues.len() {
        sum += hydropathy(residues[i]) - hydropathy(residues[i - window]);
        points.push(((i + 1 - window + half) as f64, sum / window as f64));
    }
    points
}

/// Average residue mass in daltons, a standard approximation for protein MW.
pub const AVG_RESIDUE_DA: f64 = 110.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes() {
        assert_eq!(AaClass::of('L'), AaClass::Hydrophobic);
        assert_eq!(AaClass::of('K'), AaClass::Positive);
        assert_eq!(AaClass::of('*'), AaClass::Stop);
        assert_eq!(AaClass::of('X'), AaClass::Unknown);
    }

    #[test]
    fn profile_is_windowed_mean() {
        let r: Vec<char> = "IIIRRR".chars().collect();
        let p = hydropathy_profile(&r, 3);
        assert_eq!(p.len(), 4);
        assert!((p[0].1 - 4.5).abs() < 1e-9);
        assert!((p[3].1 + 4.5).abs() < 1e-9);
        assert!(hydropathy_profile(&r, 10).is_empty());
    }
}
