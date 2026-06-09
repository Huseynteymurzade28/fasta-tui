//! Color themes shared across every widget.

use ratatui::style::Color;

/// Selectable color schemes. Cycle with the `t` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Neon,
    Pale,
    HighContrast,
}

impl Theme {
    pub fn next(self) -> Self {
        match self {
            Theme::Neon => Theme::Pale,
            Theme::Pale => Theme::HighContrast,
            Theme::HighContrast => Theme::Neon,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Theme::Neon => "Neon",
            Theme::Pale => "Pale",
            Theme::HighContrast => "HighContrast",
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Theme::Neon => Palette {
                accent: Color::Rgb(255, 121, 198),
                strand_a: Color::Rgb(0, 255, 200),
                strand_b: Color::Rgb(255, 70, 200),
                strand_back: Color::DarkGray,
                rung: Color::Rgb(120, 120, 140),
                rung_back: Color::Rgb(45, 45, 55),
                base_a: Color::Rgb(80, 250, 123),
                base_t: Color::Rgb(255, 85, 85),
                base_g: Color::Rgb(255, 184, 108),
                base_c: Color::Rgb(139, 233, 253),
                start_bg: Color::Green,
                stop_bg: Color::Red,
                match_bg: Color::Rgb(241, 250, 140),
                text: Color::Gray,
            },
            Theme::Pale => Palette {
                accent: Color::Rgb(130, 170, 255),
                strand_a: Color::Rgb(120, 190, 230),
                strand_b: Color::Rgb(200, 160, 220),
                strand_back: Color::DarkGray,
                rung: Color::Rgb(110, 110, 120),
                rung_back: Color::Rgb(50, 50, 60),
                base_a: Color::Rgb(150, 200, 150),
                base_t: Color::Rgb(220, 150, 150),
                base_g: Color::Rgb(220, 190, 150),
                base_c: Color::Rgb(150, 190, 210),
                start_bg: Color::Rgb(80, 140, 80),
                stop_bg: Color::Rgb(160, 80, 80),
                match_bg: Color::Rgb(200, 200, 120),
                text: Color::Gray,
            },
            Theme::HighContrast => Palette {
                accent: Color::White,
                strand_a: Color::Cyan,
                strand_b: Color::Magenta,
                strand_back: Color::DarkGray,
                rung: Color::Gray,
                rung_back: Color::Rgb(60, 60, 60),
                base_a: Color::Green,
                base_t: Color::Red,
                base_g: Color::Yellow,
                base_c: Color::Cyan,
                start_bg: Color::Green,
                stop_bg: Color::Red,
                match_bg: Color::Yellow,
                text: Color::White,
            },
        }
    }
}

/// Concrete colors resolved from a [`Theme`].
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub accent: Color,
    pub strand_a: Color,
    pub strand_b: Color,
    pub strand_back: Color,
    pub rung: Color,
    pub rung_back: Color,
    pub base_a: Color,
    pub base_t: Color,
    pub base_g: Color,
    pub base_c: Color,
    pub start_bg: Color,
    pub stop_bg: Color,
    pub match_bg: Color,
    pub text: Color,
}

impl Palette {
    /// Neon color for a base letter, keyed by nucleotide.
    pub fn base_color(&self, base: char) -> Color {
        match base {
            'A' => self.base_a,
            'T' => self.base_t,
            'G' => self.base_g,
            'C' => self.base_c,
            _ => self.text,
        }
    }
}
