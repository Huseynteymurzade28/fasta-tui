//! Truecolor effects: blending, gradients, animated borders, fades and
//! sub-cell bars. Everything here works on `Color::Rgb`; named colors are
//! mapped to their usual xterm values so blends still behave.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
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

/// Like [`gradient`] but periodic: `t` wraps and the last stop blends back
/// into the first, so the result can scroll forever without a seam.
pub fn cycle(stops: &[Color], t: f64) -> Color {
    let n = stops.len();
    if n == 0 {
        return Color::Reset;
    }
    let x = t.rem_euclid(1.0) * n as f64;
    let i = x.floor() as usize % n;
    lerp(stops[i], stops[(i + 1) % n], x - x.floor())
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

/// A smooth 0..1..0 pulse with the given period in ticks.
pub fn pulse(tick: u64, period: u64) -> f64 {
    let phase = (tick % period.max(1)) as f64 / period.max(1) as f64;
    0.5 - 0.5 * (phase * std::f64::consts::TAU).cos()
}

/// Text whose letters run through a scrolling gradient.
pub fn gradient_text(text: &str, stops: &[Color], phase: f64, modifier: Modifier) -> Line<'static> {
    let n = text.chars().count().max(1) as f64;
    Line::from(
        text.chars()
            .enumerate()
            .map(|(i, ch)| {
                let color = cycle(stops, i as f64 / n * 0.6 + phase);
                Span::styled(ch.to_string(), Style::default().fg(color).add_modifier(modifier))
            })
            .collect::<Vec<_>>(),
    )
}

/// Recolor the border ring of `area` with a gradient that travels around the
/// perimeter. Title text on the border keeps its own colors.
pub fn paint_border(buf: &mut Buffer, area: Rect, stops: &[Color], phase: f64) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    let (w, h) = (area.width as usize, area.height as usize);
    let perimeter = (2 * (w + h - 2)) as f64;
    let mut paint = |x: u16, y: u16, i: usize| {
        if let Some(cell) = buf.cell_mut((x, y)) {
            if is_border_glyph(cell.symbol()) {
                cell.set_fg(cycle(stops, i as f64 / perimeter + phase));
            }
        }
    };
    let (x0, y0) = (area.x, area.y);
    let (x1, y1) = (area.right() - 1, area.bottom() - 1);
    let mut i = 0;
    for x in x0..=x1 {
        paint(x, y0, i);
        i += 1;
    }
    for y in y0 + 1..=y1 {
        paint(x1, y, i);
        i += 1;
    }
    for x in (x0..x1).rev() {
        paint(x, y1, i);
        i += 1;
    }
    for y in (y0 + 1..y1).rev() {
        paint(x0, y, i);
        i += 1;
    }
}

fn is_border_glyph(s: &str) -> bool {
    matches!(
        s,
        "─" | "│" | "╭" | "╮" | "╰" | "╯" | "┌" | "┐" | "└" | "┘" | "━" | "┃" | "┏" | "┓" | "┗"
            | "┛" | "═" | "║" | "╔" | "╗" | "╚" | "╝"
    )
}

/// Blend every cell of `area` toward `toward` by `amount` (0 = untouched,
/// 1 = fully replaced). Used for view transitions and fading toasts.
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
/// resolution and a gradient running from `from` to `to` along its length.
pub fn hbar(width: usize, frac: f64, from: Color, to: Color, track: Option<Color>) -> Vec<Span<'static>> {
    let eighths = (frac.clamp(0.0, 1.0) * width as f64 * 8.0).round() as usize;
    let (full, part) = (eighths / 8, eighths % 8);
    let mut spans = Vec::with_capacity(width);
    for i in 0..width {
        let color = gradient(&[from, to], i as f64 / width.max(1) as f64);
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

/// A fixed perceptual-ish heat ramp (deep violet → magenta → amber → cream)
/// shared by every heatmap so they read alike across themes.
pub fn heat(t: f64) -> Color {
    gradient(
        &[
            Color::Rgb(22, 16, 50),
            Color::Rgb(84, 28, 120),
            Color::Rgb(176, 46, 120),
            Color::Rgb(240, 110, 70),
            Color::Rgb(252, 210, 120),
            Color::Rgb(252, 250, 220),
        ],
        t,
    )
}

/// A diverging ramp centred on `mid` (t = 0.5): cool below, warm above.
pub fn diverging(t: f64, mid: Color) -> Color {
    gradient(
        &[
            Color::Rgb(40, 110, 230),
            Color::Rgb(90, 190, 240),
            mid,
            Color::Rgb(250, 160, 80),
            Color::Rgb(240, 60, 80),
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
        assert_eq!(cycle(&s, 1.0), s[0]);
    }

    #[test]
    fn hbar_resolution() {
        let c = Color::Rgb(1, 2, 3);
        let bar = hbar(4, 0.5, c, c, None);
        let s: String = bar.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(s, "██  ");
        let bar = hbar(2, 0.25 + 1.0 / 16.0, c, c, None);
        let s: String = bar.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(s, "▋ ");
    }
}
