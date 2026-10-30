//! Rendering layer: top-level layout, header, status line, toasts, popups
//! and the shared panel chrome.

mod command;
mod enzymes;
pub mod fx;
mod gel;
mod helix;
mod help;
mod orf;
mod overview;
mod protein;
mod reader;
mod stats;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use crate::app::{App, HitMap, ToastKind, View, TOAST_TICKS};
use crate::theme::Palette;

/// Entry point used by the event loop each frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let p = app.theme.palette();
    *app.hits.borrow_mut() = HitMap::default();

    // A painted background makes every theme look the same in any terminal,
    // and gives the fades below a known color to blend toward.
    frame.render_widget(Block::default().style(Style::default().bg(p.bg).fg(p.text)), frame.area());

    let [header, content, status] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, header, app, &p);

    match app.view {
        View::Reader => reader::render(frame, content, app),
        View::Stats => stats::render(frame, content, app),
        View::Protein => protein::render(frame, content, app),
        View::Orf => orf::render(frame, content, app),
        View::Enzymes => enzymes::render(frame, content, app),
    }

    // Views fade in from the background when switched to.
    let t = app.view_progress();
    if t < 1.0 {
        fx::fade(frame.buffer_mut(), content, p.bg, 1.0 - fx::ease_out(t));
    }

    render_status(frame, status, app, &p);
    render_toasts(frame, content, app, &p);

    if app.show_help {
        backdrop(frame, &p);
        help::render(frame, app);
    }
    if app.palette.open {
        backdrop(frame, &p);
        command::render(frame, app);
    }
}

/// Dim whatever is behind a popup so the popup reads as the focus.
fn backdrop(frame: &mut Frame, p: &Palette) {
    let area = frame.area();
    fx::fade(frame.buffer_mut(), area, p.bg, 0.55);
}

// --- Shared chrome ---------------------------------------------------------

/// The standard panel: rounded border in the theme's border color and an
/// `icon title` heading.
pub fn panel<'a>(icon: &'a str, title: impl Into<String>, p: &Palette) -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(p.border))
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(icon, Style::default().fg(p.accent)),
            Span::raw(" "),
            Span::styled(title.into(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
}

/// Key hints as `key desc` pairs, for panel footers and the status line.
pub fn hints(pairs: &[(&str, &str)], p: &Palette) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (i, (k, d)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default()));
        }
        spans.push(Span::styled(
            k.to_string(),
            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {d}"), Style::default().fg(p.muted)));
    }
    spans.push(Span::raw(" "));
    Line::from(spans)
}

/// Run the theme gradient around a panel's border — reserved for the panel
/// that has the user's attention.
pub fn glow(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    fx::paint_border(frame.buffer_mut(), area, &p.grad, app.tick as f64 * 0.004);
}

/// A colored pill: ` label ` with a solid background.
pub fn chip(label: impl Into<String>, bg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", label.into()),
        Style::default().fg(fx::contrast_text(bg)).bg(bg).add_modifier(Modifier::BOLD),
    )
}

/// `1234567` → `1,234,567`.
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

// --- Header ----------------------------------------------------------------

fn render_header(frame: &mut Frame, area: Rect, app: &App, p: &Palette) {
    frame.render_widget(Block::default().style(Style::default().bg(p.surface)), area);

    // Animated logo: the theme gradient scrolls through the letters.
    let logo = " ▞▚ fasta-tui ";
    let phase = app.tick as f64 * 0.01;
    let logo_line = fx::gradient_text(logo, &p.grad, phase, Modifier::BOLD);
    let logo_w = logo.chars().count() as u16;
    frame.render_widget(logo_line, Rect { width: logo_w.min(area.width), ..area });

    // Tabs, each registered as a click target.
    let mut x = area.x + logo_w + 1;
    for (i, view) in View::ALL.iter().enumerate() {
        let active = *view == app.view;
        let label = format!(" {} {} {} ", view.icon(), i + 1, view.title());
        let w = label.chars().count() as u16;
        if x + w > area.right() {
            break;
        }
        let rect = Rect { x, y: area.y, width: w, height: 1 };
        let line = if active {
            // The active tab carries the gradient as its background.
            let n = label.chars().count().max(1) as f64;
            Line::from(
                label
                    .chars()
                    .enumerate()
                    .map(|(j, ch)| {
                        let bg = fx::gradient(&p.grad, j as f64 / n);
                        Span::styled(
                            ch.to_string(),
                            Style::default()
                                .bg(bg)
                                .fg(fx::contrast_text(bg))
                                .add_modifier(Modifier::BOLD),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            Line::from(vec![
                Span::styled(format!(" {} ", view.icon()), Style::default().fg(p.muted)),
                Span::styled(format!("{} ", i + 1), Style::default().fg(p.accent)),
                Span::styled(format!("{} ", view.title()), Style::default().fg(p.text)),
            ])
        };
        frame.render_widget(line, rect);
        app.hits.borrow_mut().tabs.push((rect, *view));
        x += w + 1;
    }

    // Record indicator: dots for each record (when few), id and length.
    let record = app.record();
    let mut spans = Vec::new();
    let n = app.records.len();
    if n > 1 && n <= 12 {
        for i in 0..n {
            let (glyph, color) = if i == app.current { ("●", p.accent) } else { ("○", p.muted) };
            spans.push(Span::styled(glyph, Style::default().fg(color)));
        }
        spans.push(Span::raw("  "));
    } else if n > 12 {
        spans.push(Span::styled(
            format!("{}/{}  ", app.current + 1, n),
            Style::default().fg(p.muted),
        ));
    }
    spans.push(Span::styled(
        record.id.clone(),
        Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::styled(
        format!(" · {} bp ", thousands(record.len())),
        Style::default().fg(p.muted),
    ));
    let right = Line::from(spans).right_aligned();
    let used = x.saturating_sub(area.x);
    if area.width > used {
        let rect = Rect { x, width: area.right() - x, ..area };
        frame.render_widget(right, rect);
    }
}

// --- Status line -----------------------------------------------------------

fn render_status(frame: &mut Frame, area: Rect, app: &App, p: &Palette) {
    frame.render_widget(Block::default().style(Style::default().bg(p.surface)), area);

    // While searching, the status line becomes the search prompt.
    if app.search.input_mode {
        let caret = if app.tick / 12 % 2 == 0 { "▏" } else { " " };
        let line = Line::from(vec![
            chip("SEARCH", p.match_bg),
            Span::styled(format!("  /{}", app.search.query), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
            Span::styled(caret, Style::default().fg(p.accent)),
            Span::styled(
                "   Enter confirm · Esc cancel · IUPAC codes A C G T N R Y … accepted",
                Style::default().fg(p.muted),
            ),
        ]);
        frame.render_widget(line, area);
        return;
    }

    let (mode, mode_color) = if app.palette.open {
        ("COMMAND", p.base_n)
    } else if app.selection.active {
        ("SELECT", p.match_bg)
    } else {
        ("NORMAL", p.accent)
    };
    let sep = || Span::styled("  ", Style::default());
    let muted = |s: String| Span::styled(s, Style::default().fg(p.muted));
    let strong = |s: String| Span::styled(s, Style::default().fg(p.text).add_modifier(Modifier::BOLD));

    let len = app.record_len();
    let mut spans = vec![chip(mode, mode_color), sep()];

    // Strand and frame.
    spans.push(Span::styled(
        if app.reverse { "3'←5' −" } else { "5'→3' +" },
        Style::default().fg(if app.reverse { p.strand_b } else { p.strand_a }).add_modifier(Modifier::BOLD),
    ));
    spans.push(muted(format!(" frame {}", app.frame + 1)));
    spans.push(sep());

    // Position plus a tiny progress bar through the record.
    spans.push(muted("pos ".into()));
    spans.push(strong(thousands(app.cursor + 1)));
    spans.push(muted(format!("/{} ", thousands(len))));
    let frac = if len > 1 { app.cursor as f64 / (len - 1) as f64 } else { 0.0 };
    spans.extend(fx::hbar(8, frac, p.grad[0], p.grad[1], Some(p.surface_hi)));
    spans.push(sep());

    // The base and codon under the cursor.
    if let Some(b) = app.cursor_base() {
        spans.push(chip(b.to_string(), p.base_color(b)));
        if let Some((start, aa)) = app.codon_at(app.cursor) {
            let codon = &app.record().bases[start..start + 3];
            let codon: String = if app.reverse {
                codon.iter().rev().map(|&c| crate::fasta::Record::complement(c) as char).collect()
            } else {
                String::from_utf8_lossy(codon).into_owned()
            };
            spans.push(muted(format!(" {codon}→")));
            spans.push(Span::styled(
                aa.to_string(),
                Style::default()
                    .fg(p.aa_color(crate::fasta::aa::AaClass::of(aa)))
                    .add_modifier(Modifier::BOLD),
            ));
        }
        spans.push(sep());
    }

    if let Some((lo, hi)) = app.selection.range(app.cursor) {
        spans.push(chip(format!("SEL {} nt", hi.min(len) - lo), p.match_bg));
        spans.push(sep());
    }
    if !app.search.matches.is_empty() {
        let label = app.search.label.as_deref().map(|l| format!("{l} ")).unwrap_or_default();
        let strands = if app.search.both_strands { "±" } else { "+" };
        spans.push(chip(
            format!(
                "{label}{}{strands} {}/{}",
                app.search.query,
                app.search.active + 1,
                app.search.matches.len()
            ),
            p.base_g,
        ));
        spans.push(sep());
    }
    frame.render_widget(Line::from(spans), area);

    // Context-sensitive key hints on the right.
    let keys: &[(&str, &str)] = match app.view {
        View::Reader => &[("/", "search"), ("v", "select"), ("c a", "tracks"), (":", "commands"), ("?", "help")],
        View::Stats => &[("f", "frame"), ("r", "strand"), (":", "commands"), ("?", "help")],
        View::Protein => &[("←→↑↓", "residues"), ("f", "frame"), ("r", "strand"), ("?", "help")],
        View::Orf => &[("↑↓", "select"), ("⏎", "open"), ("m M", "min"), ("y", "copy"), ("?", "help")],
        View::Enzymes => &[("↑↓", "select"), ("⏎", "find"), ("u", "cutters"), ("?", "help")],
    };
    frame.render_widget(hints(keys, p).right_aligned(), area);
}

// --- Toasts ----------------------------------------------------------------

/// Ticks a toast spends sliding in, and fading out at the end of its life.
const SLIDE_TICKS: u64 = 6;
const FADE_TICKS: u64 = 12;

fn render_toasts(frame: &mut Frame, area: Rect, app: &App, p: &Palette) {
    let mut bottom = area.bottom().saturating_sub(1);
    for toast in app.toasts.iter().rev() {
        let age = app.tick.saturating_sub(toast.born);
        let (icon, color) = match toast.kind {
            ToastKind::Info => ("ℹ", p.accent),
            ToastKind::Success => ("✔", p.base_a),
            ToastKind::Warn => ("⚠", p.base_t),
        };
        let width = (toast.msg.chars().count() as u16 + 7).min(area.width.saturating_sub(2));
        let height = 3;
        if bottom < area.y + height || width < 8 {
            break;
        }
        // Slide in from the right edge with an ease-out.
        let slide = 1.0 - fx::ease_out(age as f64 / SLIDE_TICKS as f64);
        let offset = (slide * (width + 2) as f64) as u16;
        let x = (area.right().saturating_sub(width + 1) + offset).min(area.right());
        let visible_w = width.min(area.right().saturating_sub(x));
        if visible_w < 2 {
            bottom = bottom.saturating_sub(height);
            continue;
        }
        let rect = Rect { x, y: bottom - height, width: visible_w, height };

        frame.render_widget(Clear, rect);
        let body = Paragraph::new(Line::from(vec![
            Span::styled(format!(" {icon} "), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::styled(toast.msg.clone(), Style::default().fg(p.text)),
        ]))
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(color))
                .style(Style::default().bg(p.surface)),
        );
        frame.render_widget(body, rect);

        let remaining = (toast.born + TOAST_TICKS).saturating_sub(app.tick);
        if remaining < FADE_TICKS {
            let amount = 1.0 - remaining as f64 / FADE_TICKS as f64;
            fx::fade(frame.buffer_mut(), rect, p.bg, amount);
        }
        bottom -= height;
    }
}

// --- Layout helpers --------------------------------------------------------

/// The half-open slice of a `total`-row list to draw so that `selected` stays
/// visible in a viewport `height` rows tall.
pub fn scroll_window(selected: usize, total: usize, height: usize) -> (usize, usize) {
    if height == 0 || total == 0 {
        return (0, 0);
    }
    // Keep the selection centred once the list is long enough to scroll.
    let first = selected.saturating_sub(height / 2).min(total.saturating_sub(height));
    (first, (first + height).min(total))
}

/// A centered rectangle `width` × `height` cells (clamped to `area`).
pub fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1234567), "1,234,567");
    }
}
