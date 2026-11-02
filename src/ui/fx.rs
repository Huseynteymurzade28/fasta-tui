//! Truecolor helpers: blending, heatmap ramps, fades and sub-cell bars.
//! Everything here works on `Color::Rgb`; named colors are mapped to their
//! usual xterm values so blends still behave.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
};

/// RGB components of any color.
pub fn rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Black => (0, 0, 0),
        Color::Red => (205, 49, 49),
        Color::Green => (13, 188, 121),
        Color::Yellow => (229, 229, 16),
        Color::Blue => (36, 114, 200),
        Color::Magenta => (188, 63, 188),
        Color::Cyan => (17, 168, 205),
        Color::Gray => (204, 204, 204),
        Color::DarkGray => (102, 102, 102),
        Color::LightRed => (241, 76, 76),
        Color::LightGreen => (35, 209, 139),
        Color::LightYellow => (245, 245, 67),
        Color::LightBlue => (59, 142, 234),
        Color::LightMagenta => (214, 112, 214),
        Color::LightCyan => (41, 184, 219),
        Color::White => (255, 255, 255),
        _ => (128, 128, 128),
    }
}

/// Linear blend from `a` (t = 0) to `b` (t = 1).
pub fn lerp(a: Color, b: Color, t: f64) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);
    let mix = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    Color::Rgb(mix(ar, br), mix(ag, bg), mix(ab, bb))
}

/// Sample a multi-stop gradient at `t` in `0..=1` (clamped).
pub fn gradient(stops: &[Color], t: f64) -> Color {
    match stops.len() {
        0 => Color::Reset,
        1 => stops[0],
        n => {
            let x = t.clamp(0.0, 1.0) * (n - 1) as f64;
            let i = (x.floor() as usize).min(n - 2);
            lerp(stops[i], stops[i + 1], x - i as f64)
        }
    }
}

/// Relative luminance in `0..=1`, for choosing readable text over a color.
pub fn luminance(c: Color) -> f64 {
    let (r, g, b) = rgb(c);
    (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64) / 255.0
}

/// Black or white, whichever reads better on `bg`.
pub fn contrast_text(bg: Color) -> Color {
    if luminance(bg) > 0.55 {
        Color::Rgb(16, 16, 20)
    } else {
        Color::Rgb(245, 245, 250)
    }
}

/// Cubic ease-out: fast start, gentle landing.
pub fn ease_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Blend every cell of `area` toward `toward` by `amount` (0 = untouched,
/// 1 = fully replaced). Used for popup backdrops and fading toasts.
pub fn fade(buf: &mut Buffer, area: Rect, toward: Color, amount: f64) {
    if amount <= 0.0 {
        return;
    }
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                if cell.fg != Color::Reset {
                    cell.set_fg(lerp(cell.fg, toward, amount));
                }
                if cell.bg != Color::Reset {
                    cell.set_bg(lerp(cell.bg, toward, amount));
                }
            }
        }
    }
}

/// Eighth-block glyphs, from empty to a full cell.
const EIGHTHS: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

/// A horizontal bar `width` cells long filled to `frac`, with 1/8-cell
/// resolution, in a single flat color.
pub fn hbar(width: usize, frac: f64, color: Color, track: Option<Color>) -> Vec<Span<'static>> {
    let eighths = (frac.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
    let (full, part) = (eighths / 8, eighths % 8);
    let mut spans = Vec::with_capacity(width);
    for i in 0..width {
        let mut style = Style::default().fg(color);
        if let Some(t) = track {
            style = style.bg(t);
        }
        let glyph = if i < full {
            "█"
        } else if i == full && part > 0 {
            EIGHTHS[part]
        } else {
            " "
        };
        spans.push(Span::styled(glyph, style));
    }
    spans
}

/// A calm sequential ramp (slate → teal → sage → sand) shared by every
/// heatmap so they read alike across themes. Kept low in saturation so a
/// full screen of it stays easy on the eyes.
pub fn heat(t: f64) -> Color {
    gradient(
        &[
            Color::Rgb(30, 40, 58),
            Color::Rgb(38, 78, 96),
            Color::Rgb(56, 118, 124),
            Color::Rgb(112, 160, 138),
            Color::Rgb(186, 196, 150),
            Color::Rgb(232, 222, 184),
        ],
        t,
    )
}

/// A diverging ramp centred on `mid` (t = 0.5): cool below, warm above.
pub fn diverging(t: f64, mid: Color) -> Color {
    gradient(
        &[
            Color::Rgb(72, 112, 168),
            Color::Rgb(112, 148, 186),
            mid,
            Color::Rgb(196, 150, 112),
            Color::Rgb(184, 92, 96),
        ],
        t,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_endpoints() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(200, 100, 50);
        assert_eq!(lerp(a, b, 0.0), a);
        assert_eq!(lerp(a, b, 1.0), b);
        assert_eq!(lerp(a, b, 0.5), Color::Rgb(100, 50, 25));
    }

    #[test]
    fn gradient_hits_stops() {
        let s = [Color::Rgb(0, 0, 0), Color::Rgb(100, 0, 0), Color::Rgb(100, 100, 0)];
        assert_eq!(gradient(&s, 0.0), s[0]);
        assert_eq!(gradient(&s, 0.5), s[1]);
        assert_eq!(gradient(&s, 1.0), s[2]);
    }

    #[test]
    fn hbar_resolution() {
        let c = Color::Rgb(1, 2, 3);
        let bar = hbar(4, 0.5, c, None);
        let s: String = bar.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(s, "██  ");
        let bar = hbar(2, 0.25 + 1.0 / 16.0, c, None);
        let s: String = bar.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(s, "▋ ");
    }
}
