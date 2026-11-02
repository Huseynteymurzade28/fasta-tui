//! Color themes shared across every widget.

use ratatui::style::Color;

use crate::fasta::aa::AaClass;

/// Selectable color schemes. Cycle with the `t` key. `Slate` is the default:
/// low-saturation colors that stay comfortable over long reading sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Slate,
    Neon,
    Aurora,
    Pale,
    HighContrast,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Theme::Slate => Theme::Neon,
            Theme::Neon => Theme::Aurora,
            Theme::Aurora => Theme::Pale,
            Theme::Pale => Theme::HighContrast,
            Theme::HighContrast => Theme::Slate,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Slate => "Slate",
            Theme::Neon => "Neon",
            Theme::Aurora => "Aurora",
            Theme::Pale => "Pale",
            Theme::HighContrast => "HighContrast",
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Theme::Slate => Palette {
                bg: Color::Rgb(24, 26, 33),
                surface: Color::Rgb(32, 35, 44),
                surface_hi: Color::Rgb(45, 49, 61),
                border: Color::Rgb(60, 65, 80),
                muted: Color::Rgb(132, 138, 156),
                accent: Color::Rgb(126, 170, 218),
                strand_a: Color::Rgb(112, 186, 176),
                strand_b: Color::Rgb(190, 152, 204),
                base_a: Color::Rgb(136, 192, 132),
                base_t: Color::Rgb(218, 124, 124),
                base_g: Color::Rgb(222, 188, 124),
                base_c: Color::Rgb(122, 168, 220),
                base_n: Color::Rgb(178, 150, 208),
                start_bg: Color::Rgb(76, 128, 92),
                stop_bg: Color::Rgb(156, 74, 84),
                match_bg: Color::Rgb(226, 198, 126),
                text: Color::Rgb(212, 216, 226),
            },
            Theme::Neon => Palette {
                bg: Color::Rgb(22, 19, 32),
                surface: Color::Rgb(33, 29, 48),
                surface_hi: Color::Rgb(48, 42, 70),
                border: Color::Rgb(66, 58, 96),
                muted: Color::Rgb(140, 132, 170),
                accent: Color::Rgb(236, 132, 196),
                strand_a: Color::Rgb(96, 220, 190),
                strand_b: Color::Rgb(226, 120, 196),
                base_a: Color::Rgb(112, 222, 140),
                base_t: Color::Rgb(238, 110, 116),
                base_g: Color::Rgb(240, 184, 118),
                base_c: Color::Rgb(130, 210, 236),
                base_n: Color::Rgb(184, 150, 236),
                start_bg: Color::Rgb(52, 142, 86),
                stop_bg: Color::Rgb(176, 60, 82),
                match_bg: Color::Rgb(232, 232, 150),
                text: Color::Rgb(222, 218, 236),
            },
            Theme::Aurora => Palette {
                bg: Color::Rgb(13, 23, 31),
                surface: Color::Rgb(21, 37, 48),
                surface_hi: Color::Rgb(31, 54, 68),
                border: Color::Rgb(46, 76, 92),
                muted: Color::Rgb(124, 152, 168),
                accent: Color::Rgb(104, 198, 226),
                strand_a: Color::Rgb(110, 214, 164),
                strand_b: Color::Rgb(166, 136, 226),
                base_a: Color::Rgb(122, 214, 150),
                base_t: Color::Rgb(232, 124, 138),
                base_g: Color::Rgb(234, 198, 122),
                base_c: Color::Rgb(112, 178, 234),
                base_n: Color::Rgb(186, 150, 232),
                start_bg: Color::Rgb(40, 134, 100),
                stop_bg: Color::Rgb(170, 68, 92),
                match_bg: Color::Rgb(236, 214, 128),
                text: Color::Rgb(208, 226, 234),
            },
            Theme::Pale => Palette {
                bg: Color::Rgb(30, 34, 42),
                surface: Color::Rgb(40, 45, 56),
                surface_hi: Color::Rgb(54, 60, 74),
                border: Color::Rgb(70, 78, 94),
                muted: Color::Rgb(140, 148, 164),
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
                muted: Color::Rgb(180, 180, 180),
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
    /// The one highlight color: logo, active tab, focused border, cursor.
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
