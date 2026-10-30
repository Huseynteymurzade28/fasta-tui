//! Color themes shared across every widget.

use ratatui::style::Color;

use crate::fasta::aa::AaClass;

/// Selectable color schemes. Cycle with the `t` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Neon,
    Aurora,
    Pale,
    HighContrast,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Theme::Neon => Theme::Aurora,
            Theme::Aurora => Theme::Pale,
            Theme::Pale => Theme::HighContrast,
            Theme::HighContrast => Theme::Neon,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Neon => "Neon",
            Theme::Aurora => "Aurora",
            Theme::Pale => "Pale",
            Theme::HighContrast => "HighContrast",
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Theme::Neon => Palette {
                bg: Color::Rgb(20, 17, 31),
                surface: Color::Rgb(34, 29, 52),
                surface_hi: Color::Rgb(52, 44, 80),
                border: Color::Rgb(70, 62, 104),
                muted: Color::Rgb(108, 100, 140),
                grad: [
                    Color::Rgb(0, 255, 200),
                    Color::Rgb(255, 70, 200),
                    Color::Rgb(189, 147, 249),
                ],
                accent: Color::Rgb(255, 121, 198),
                strand_a: Color::Rgb(0, 255, 200),
                strand_b: Color::Rgb(255, 70, 200),
                base_a: Color::Rgb(80, 250, 123),
                base_t: Color::Rgb(255, 85, 85),
                base_g: Color::Rgb(255, 184, 108),
                base_c: Color::Rgb(139, 233, 253),
                base_n: Color::Rgb(189, 147, 249),
                start_bg: Color::Rgb(40, 160, 80),
                stop_bg: Color::Rgb(200, 50, 70),
                match_bg: Color::Rgb(241, 250, 140),
                text: Color::Rgb(220, 216, 236),
            },
            Theme::Aurora => Palette {
                bg: Color::Rgb(11, 22, 30),
                surface: Color::Rgb(20, 38, 50),
                surface_hi: Color::Rgb(30, 58, 74),
                border: Color::Rgb(44, 78, 96),
                muted: Color::Rgb(96, 128, 146),
                grad: [
                    Color::Rgb(80, 255, 160),
                    Color::Rgb(60, 170, 255),
                    Color::Rgb(190, 110, 255),
                ],
                accent: Color::Rgb(80, 220, 255),
                strand_a: Color::Rgb(90, 255, 170),
                strand_b: Color::Rgb(170, 120, 255),
                base_a: Color::Rgb(110, 240, 150),
                base_t: Color::Rgb(255, 110, 130),
                base_g: Color::Rgb(255, 205, 110),
                base_c: Color::Rgb(100, 190, 255),
                base_n: Color::Rgb(200, 150, 255),
                start_bg: Color::Rgb(30, 150, 100),
                stop_bg: Color::Rgb(190, 60, 90),
                match_bg: Color::Rgb(255, 230, 120),
                text: Color::Rgb(206, 226, 236),
            },
            Theme::Pale => Palette {
                bg: Color::Rgb(30, 34, 42),
                surface: Color::Rgb(40, 45, 56),
                surface_hi: Color::Rgb(54, 60, 74),
                border: Color::Rgb(70, 78, 94),
                muted: Color::Rgb(118, 126, 142),
                grad: [
                    Color::Rgb(136, 192, 208),
                    Color::Rgb(180, 142, 173),
                    Color::Rgb(163, 190, 140),
                ],
                accent: Color::Rgb(136, 192, 208),
                strand_a: Color::Rgb(120, 190, 230),
                strand_b: Color::Rgb(200, 160, 220),
                base_a: Color::Rgb(163, 190, 140),
                base_t: Color::Rgb(220, 150, 150),
                base_g: Color::Rgb(235, 203, 139),
                base_c: Color::Rgb(143, 188, 220),
                base_n: Color::Rgb(180, 160, 200),
                start_bg: Color::Rgb(80, 130, 80),
                stop_bg: Color::Rgb(150, 75, 80),
                match_bg: Color::Rgb(235, 203, 139),
                text: Color::Rgb(216, 222, 233),
            },
            Theme::HighContrast => Palette {
                bg: Color::Rgb(0, 0, 0),
                surface: Color::Rgb(28, 28, 28),
                surface_hi: Color::Rgb(56, 56, 56),
                border: Color::Rgb(140, 140, 140),
                muted: Color::Rgb(170, 170, 170),
                grad: [
                    Color::Rgb(255, 255, 255),
                    Color::Rgb(0, 255, 255),
                    Color::Rgb(255, 0, 255),
                ],
                accent: Color::Rgb(255, 255, 0),
                strand_a: Color::Rgb(0, 255, 255),
                strand_b: Color::Rgb(255, 0, 255),
                base_a: Color::Rgb(0, 255, 0),
                base_t: Color::Rgb(255, 60, 60),
                base_g: Color::Rgb(255, 255, 0),
                base_c: Color::Rgb(0, 200, 255),
                base_n: Color::Rgb(255, 0, 255),
                start_bg: Color::Rgb(0, 160, 0),
                stop_bg: Color::Rgb(200, 0, 0),
                match_bg: Color::Rgb(255, 255, 0),
                text: Color::Rgb(255, 255, 255),
            },
        }
    }
}

/// Concrete colors resolved from a [`Theme`]. Every color is truecolor RGB so
/// the effects in `ui::fx` can blend between them.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Screen background.
    pub bg: Color,
    /// Raised surfaces: the cursor row, zebra stripes, popups.
    pub surface: Color,
    /// A step above `surface`, for hover-like emphasis.
    pub surface_hi: Color,
    /// Inactive panel borders.
    pub border: Color,
    /// Secondary text: labels, hints, gutters.
    pub muted: Color,
    /// Stops of the theme's signature gradient (logo, focused borders).
    pub grad: [Color; 3],
    pub accent: Color,
    pub strand_a: Color,
    pub strand_b: Color,
    pub base_a: Color,
    pub base_t: Color,
    pub base_g: Color,
    pub base_c: Color,
    pub base_n: Color,
    pub start_bg: Color,
    pub stop_bg: Color,
    pub match_bg: Color,
    pub text: Color,
}

impl Palette {
    /// Color for a base letter, keyed by nucleotide. IUPAC ambiguity codes
    /// share the `base_n` hue so they stand out from the four plain bases.
    pub fn base_color(&self, base: char) -> Color {
        match base {
            'A' => self.base_a,
            'T' => self.base_t,
            'G' => self.base_g,
            'C' => self.base_c,
            'N' | 'R' | 'Y' | 'S' | 'W' | 'K' | 'M' | 'B' | 'D' | 'H' | 'V' => self.base_n,
            _ => self.text,
        }
    }

    /// Color for a residue's physicochemical class.
    pub fn aa_color(&self, class: AaClass) -> Color {
        match class {
            AaClass::Hydrophobic => self.base_g,
            AaClass::Aromatic => self.base_n,
            AaClass::Polar => self.base_a,
            AaClass::Positive => self.base_c,
            AaClass::Negative => self.base_t,
            AaClass::Special => self.match_bg,
            AaClass::Stop => self.stop_bg,
            AaClass::Unknown => self.muted,
        }
    }
}
