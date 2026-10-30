//! A virtual agarose gel: fragment sizes laid out on a log scale beside a
//! ladder, rendered at double vertical resolution with half-block cells and
//! glowing, mass-weighted bands. Bands run down from the wells whenever the
//! digest changes.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use super::fx;
use crate::theme::Palette;

/// One lane of the gel.
pub struct Lane {
    pub label: String,
    /// Fragment sizes in bp; duplicates stack into a brighter band.
    pub bands: Vec<usize>,
    pub ladder: bool,
}

/// Pick a ladder that brackets a molecule of `len` bp.
pub fn ladder_for(len: usize) -> Vec<usize> {
    if len <= 3000 {
        vec![3000, 2000, 1500, 1000, 800, 600, 500, 400, 300, 200, 100]
    } else {
        vec![10000, 8000, 6000, 5000, 4000, 3000, 2000, 1500, 1000, 750, 500, 250]
    }
}

/// Sub-row (half-cell) position of a band of `size` bp, on a log scale
/// between `hi` (top) and `lo` (bottom) across `rows` sub-rows.
pub fn band_row(size: usize, lo: f64, hi: f64, rows: usize) -> usize {
    let s = (size.max(1) as f64).clamp(lo, hi);
    let t = (hi.ln() - s.ln()) / (hi.ln() - lo.ln());
    ((t * (rows - 1) as f64).round() as usize).min(rows - 1)
}

/// Draw the gel into `area`. `run` in `0..=1` is how far the bands have
/// migrated from the wells.
pub fn render(buf: &mut Buffer, area: Rect, lanes: &[Lane], run: f64, p: &Palette) {
    if lanes.is_empty() || area.width < 10 || area.height < 6 {
        return;
    }
    let gel_bg = fx::lerp(p.bg, Color::Rgb(14, 24, 52), 0.8);
    let glow = Color::Rgb(255, 196, 110);
    let ladder_glow = Color::Rgb(150, 200, 255);

    // Size labels on the left, lane labels on the top row.
    let label_w = 6u16;
    let gel = Rect {
        x: area.x + label_w,
        y: area.y + 1,
        width: area.width - label_w,
        height: area.height - 1,
    };
    for y in gel.top()..gel.bottom() {
        for x in gel.left()..gel.right() {
            if let Some(c) = buf.cell_mut((x, y)) {
                c.set_symbol(" ").set_bg(gel_bg);
            }
        }
    }

    let all: Vec<usize> = lanes.iter().flat_map(|l| l.bands.iter().copied()).collect();
    let hi = all.iter().copied().max().unwrap_or(1000) as f64 * 1.25;
    let lo = all.iter().copied().min().unwrap_or(100).max(1) as f64 * 0.8;
    // Row 0 holds the wells; bands migrate below it.
    let rows = gel.height as usize * 2;
    let well = 1usize;
    let span = rows - well - 1;

    let lane_w = (gel.width / lanes.len() as u16).max(3);
    for (li, lane) in lanes.iter().enumerate() {
        let x0 = gel.x + li as u16 * lane_w + 1;
        let w = lane_w.saturating_sub(2).max(1);

        let mut label = lane.label.clone();
        label.truncate(lane_w as usize);
        let lx = x0 + w.saturating_sub(label.len() as u16) / 2;
        buf.set_string(lx, area.y, &label, Style::default().fg(p.text).add_modifier(Modifier::BOLD));

        // Accumulate intensity per sub-row. Mass scales with fragment length,
        // so long fragments glow brighter, as with a real stain.
        let mut intensity = vec![0.0f64; rows];
        let max_band = lane.bands.iter().copied().max().unwrap_or(1).max(1) as f64;
        for &size in &lane.bands {
            let target = well + 1 + band_row(size, lo, hi, span);
            let y = well + ((target - well) as f64 * run).round() as usize;
            let mass = if lane.ladder { 0.75 } else { 0.45 + 0.55 * (size as f64 / max_band).sqrt() };
            intensity[y] += mass;
            if y > 0 {
                intensity[y - 1] += mass * 0.3;
            }
            if y + 1 < rows {
                intensity[y + 1] += mass * 0.3;
            }
            if lane.ladder && run >= 1.0 && li == 0 {
                let cy = gel.y + (target / 2) as u16;
                let txt = super::thousands(size);
                let tx = area.x + label_w.saturating_sub(txt.len() as u16 + 1);
                buf.set_string(tx, cy, &txt, Style::default().fg(p.muted));
            }
        }
        let tint = if lane.ladder { ladder_glow } else { glow };
        let shade = |v: f64| fx::lerp(gel_bg, tint, v.min(1.0));
        for row in 0..gel.height as usize {
            let (top, bottom) = (intensity[row * 2], intensity[row * 2 + 1]);
            for dx in 0..w {
                // Band edges are a touch dimmer, like a real lane.
                let edge = if dx == 0 || dx + 1 == w { 0.7 } else { 1.0 };
                if let Some(c) = buf.cell_mut((x0 + dx, gel.y + row as u16)) {
                    c.set_symbol("▀").set_fg(shade(top * edge)).set_bg(shade(bottom * edge));
                }
            }
        }
        // The well itself.
        for dx in 0..w {
            if let Some(c) = buf.cell_mut((x0 + dx, gel.y)) {
                c.set_fg(fx::lerp(gel_bg, Color::Rgb(0, 0, 0), 0.7));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigger_fragments_run_shorter() {
        let (lo, hi) = (100.0, 10000.0);
        let big = band_row(5000, lo, hi, 40);
        let small = band_row(200, lo, hi, 40);
        assert!(big < small);
        assert_eq!(band_row(10000, lo, hi, 40), 0);
        assert_eq!(band_row(100, lo, hi, 40), 39);
    }
}
