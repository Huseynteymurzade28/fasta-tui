//! The right-pane 3D Braille double helix.
//!
//! Depth is shaded continuously: every strand sample is bucketed by its z
//! coordinate and blended from the background toward the strand color, the
//! front half of the backbone is drawn thicker, and each rung is split into
//! its two bases' colors so a pair reads like a textbook cartoon.

use std::f64::consts::PI;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::canvas::{Canvas, Line as CanvasLine, Points},
    Frame,
};

use super::{fx, panel};
use crate::app::App;
use crate::fasta::Record;
use crate::theme::Palette;

/// Number of base pairs visible in the helix at once.
const HELIX_RUNGS: usize = 18;
/// Depth buckets used for shading.
const DEPTH_BANDS: usize = 8;
/// Ambient particles drifting up behind the helix.
const PARTICLES: usize = 36;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let phase = app.helix.phase;
    let twist = app.helix.twist;
    let tick = app.tick;

    // Snapshot the visible window of bases so the paint closure owns its data.
    let visible: Vec<char> = (0..HELIX_RUNGS)
        .map(|r| app.record().base_at(app.cursor + r))
        .collect();

    // Backbone thickness: roughly one Braille dot at this pane's resolution.
    let dot_w = 3.4 / (area.width.saturating_sub(2).max(1) as f64 * 2.0);

    let status = if app.helix.paused { "paused" } else { "spinning" };
    let block = panel("⧖", "Double Helix", &p)
        .title(
            Line::from(Span::styled(
                format!(" {} bp from {} ", HELIX_RUNGS, app.cursor + 1),
                Style::default().fg(p.muted),
            ))
            .right_aligned(),
        )
        .title_bottom(Line::from(vec![
            Span::styled(" ⟳ ", Style::default().fg(p.accent)),
            Span::styled(
                format!("{status} {:.2}  twist {:.2} ", app.helix.speed, twist),
                Style::default().fg(p.muted),
            ),
        ]));

    let canvas = Canvas::default()
        .block(block)
        .background_color(p.bg)
        .marker(Marker::Braille)
        .x_bounds([-1.7, 1.7])
        .y_bounds([-0.04, 1.04])
        .paint(move |ctx| {
            let rungs = HELIX_RUNGS as f64;

            // 0. Faint particles, like dust in a light beam.
            let mut dust = Vec::with_capacity(PARTICLES);
            for i in 0..PARTICLES {
                let seed = hash(i as u64);
                let x = (seed % 1000) as f64 / 1000.0 * 3.2 - 1.6;
                let speed = 0.0015 + (seed / 1000 % 100) as f64 / 100.0 * 0.003;
                let y = ((seed / 100_000 % 1000) as f64 / 1000.0 + tick as f64 * speed).fract();
                let wobble = (tick as f64 * 0.03 + i as f64).sin() * 0.03;
                dust.push((x + wobble, y));
            }
            ctx.draw(&Points { coords: &dust, color: fx::lerp(p.bg, p.muted, 0.35) });

            // 1. Smoothly sampled strands, bucketed by depth.
            let steps = 300usize;
            let mut bands: Vec<Vec<(f64, f64)>> = vec![Vec::new(); DEPTH_BANDS * 2];
            for s in 0..=steps {
                let frac = s as f64 / steps as f64;
                let y = 1.0 - frac;
                let t = frac * rungs * twist + phase;
                for (strand, offset) in [(0usize, 0.0), (1, PI)] {
                    let (x, z) = ((t + offset).sin(), (t + offset).cos());
                    let band = depth_band(z);
                    let slot = &mut bands[strand * DEPTH_BANDS + band];
                    slot.push((x, y));
                    // Thicken the nearest part of the backbone.
                    if z > 0.35 {
                        slot.push((x + dot_w, y));
                        slot.push((x - dot_w, y));
                    }
                }
            }
            // Back to front, so nearer bands overwrite farther ones.
            for band in 0..DEPTH_BANDS {
                for (strand, color) in [(0, p.strand_a), (1, p.strand_b)] {
                    let coords = &bands[strand * DEPTH_BANDS + band];
                    ctx.draw(&Points { coords, color: depth_color(color, band, &p) });
                }
            }

            ctx.layer();

            // 2. Base-pair rungs, each half in its base's color.
            for (r, &base) in visible.iter().enumerate() {
                if base == ' ' {
                    continue;
                }
                let y = 1.0 - r as f64 / rungs;
                let t = r as f64 * twist + phase;
                let (xa, za) = (t.sin(), t.cos());
                let (xb, zb) = ((t + PI).sin(), (t + PI).cos());
                let mid = (xa + xb) / 2.0;
                let comp = Record::complement(base as u8) as char;

                let shade = |c: Color, z: f64| fx::lerp(p.bg, c, 0.25 + 0.75 * (z + 1.0) / 2.0);
                // The cursor's pair, at the top, is lit in full.
                let (ca, cb) = if r == 0 {
                    (p.accent, p.accent)
                } else {
                    (shade(p.base_color(base), za), shade(p.base_color(comp), zb))
                };
                ctx.draw(&CanvasLine { x1: xa, y1: y, x2: mid, y2: y, color: ca });
                ctx.draw(&CanvasLine { x1: mid, y1: y, x2: xb, y2: y, color: cb });

                ctx.print(xa, y, letter(base, za, r == 0, &p));
                ctx.print(xb, y, letter(comp, zb, r == 0, &p));
            }

            // 3. Strand-end labels.
            let label = |s: &'static str| Line::from(Span::styled(s, Style::default().fg(p.muted)));
            ctx.print(-1.65, 1.02, label("5'"));
            ctx.print(1.5, 1.02, label("3'"));
            ctx.print(-1.65, -0.02, label("3'"));
            ctx.print(1.5, -0.02, label("5'"));
        });

    frame.render_widget(canvas, area);
}

/// Map z in `-1..=1` to a depth bucket, 0 = farthest.
fn depth_band(z: f64) -> usize {
    (((z + 1.0) / 2.0 * DEPTH_BANDS as f64) as usize).min(DEPTH_BANDS - 1)
}

/// A strand color faded toward the background with distance.
fn depth_color(color: Color, band: usize, p: &Palette) -> Color {
    let t = (band as f64 + 0.5) / DEPTH_BANDS as f64;
    fx::lerp(p.bg, color, 0.18 + 0.82 * t * t)
}

/// A single floating base letter, bright in front and dim behind.
fn letter(base: char, z: f64, focus: bool, p: &Palette) -> Line<'static> {
    let style = if focus {
        Style::default().fg(fx::contrast_text(p.accent)).bg(p.accent).add_modifier(Modifier::BOLD)
    } else if z >= 0.0 {
        Style::default().fg(p.base_color(base)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(fx::lerp(p.bg, p.base_color(base), 0.4))
    };
    Line::from(Span::styled(base.to_string(), style))
}

/// A cheap deterministic hash for particle placement (splitmix64).
fn hash(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}
