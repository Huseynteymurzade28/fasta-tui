//! The right-pane 3D Braille double helix.

use std::f64::consts::PI;

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, BorderType,
    },
    Frame,
};

use crate::app::App;
use crate::fasta::Record;
use crate::theme::Palette;

/// Number of base pairs visible in the helix at once.
const HELIX_RUNGS: usize = 18;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let phase = app.helix.phase;
    let twist = app.helix.twist;

    // Snapshot the visible window of bases so the paint closure owns its data.
    let visible: Vec<char> = (0..HELIX_RUNGS)
        .map(|r| app.record().base_at(app.cursor + r))
        .collect();

    let canvas = Canvas::default()
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" 3D Double Helix "),
        )
        .marker(Marker::Braille)
        .x_bounds([-1.7, 1.7])
        .y_bounds([0.0, 1.0])
        .paint(move |ctx| {
            let rungs = HELIX_RUNGS as f64;

            // 1. Smoothly sampled strands.
            let steps = 240usize;
            let (mut a_front, mut a_back) = (Vec::new(), Vec::new());
            let (mut b_front, mut b_back) = (Vec::new(), Vec::new());
            for s in 0..=steps {
                let frac = s as f64 / steps as f64;
                let y = 1.0 - frac;
                let t = frac * rungs * twist + phase;

                let (xa, za) = (t.sin(), t.cos());
                let (xb, zb) = ((t + PI).sin(), (t + PI).cos());

                if za >= 0.0 {
                    a_front.push((xa, y));
                } else {
                    a_back.push((xa, y));
                }
                if zb >= 0.0 {
                    b_front.push((xb, y));
                } else {
                    b_back.push((xb, y));
                }
            }

            // Back strands first, bright front strands over them.
            ctx.draw(&Points { coords: &a_back, color: palette.strand_back });
            ctx.draw(&Points { coords: &b_back, color: palette.strand_back });
            ctx.draw(&Points { coords: &a_front, color: palette.strand_a });
            ctx.draw(&Points { coords: &b_front, color: palette.strand_b });

            ctx.layer();

            // 2. Base-pair rungs and floating letters.
            for (r, &base) in visible.iter().enumerate() {
                if base == ' ' {
                    continue;
                }
                let frac = r as f64 / rungs;
                let y = 1.0 - frac;
                let t = r as f64 * twist + phase;

                let (xa, za) = (t.sin(), t.cos());
                let (xb, zb) = ((t + PI).sin(), (t + PI).cos());

                let rung_color = if za >= 0.0 { palette.rung } else { palette.rung_back };
                ctx.draw(&CanvasLine { x1: xa, y1: y, x2: xb, y2: y, color: rung_color });

                let comp = Record::complement(base as u8) as char;
                ctx.print(xa, y, letter(base, za, &palette));
                ctx.print(xb, y, letter(comp, zb, &palette));
            }
        });

    frame.render_widget(canvas, area);
}

/// A single floating base letter, bright in front and dim behind.
fn letter(base: char, z: f64, palette: &Palette) -> Line<'static> {
    let style = if z >= 0.0 {
        Style::default().fg(palette.base_color(base)).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    Line::from(Span::styled(base.to_string(), style))
}
