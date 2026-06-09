//! Left/right split of the Reader view: GC gauge + sequence text + helix.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Gauge, Paragraph, Wrap},
    Frame,
};

use super::helix;
use crate::app::App;
use crate::fasta::stats;
use crate::theme::Palette;

/// Bases shown per row in the reader.
const BASES_PER_ROW: usize = 60;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_left(frame, cols[0], app);
    helix::render(frame, cols[1], app);
}

fn render_left(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    // --- GC content gauge ---------------------------------------------------
    let gc = stats::gc_content(&app.record().bases);
    let desc = &app.record().description;
    let gauge_title = if desc.is_empty() {
        " GC Content ".to_string()
    } else {
        format!(" GC Content — {desc} ")
    };
    let gauge = Gauge::default()
        .block(Block::bordered().title(gauge_title))
        .gauge_style(Style::default().fg(palette.accent))
        .ratio(gc)
        .label(format!("{:.1}%  ({} bases)", gc * 100.0, app.record_len()));
    frame.render_widget(gauge, rows[0]);

    // --- Sequence reader ----------------------------------------------------
    let lines = build_reader_lines(app, &palette);
    let scroll_row = (app.cursor / BASES_PER_ROW) as u16;
    let title = " Sequence  (ATG=start, TAA/TAG/TGA=stop, motif=highlight) ";
    let reader = Paragraph::new(lines)
        .block(Block::bordered().title(title))
        .wrap(Wrap { trim: false })
        .scroll((scroll_row, 0));
    frame.render_widget(reader, rows[1]);
}

/// Build codon- and motif-highlighted rows for the current record.
fn build_reader_lines(app: &App, palette: &Palette) -> Vec<Line<'static>> {
    let seq = &app.record().bases;
    if seq.is_empty() {
        return vec![Line::from("<empty sequence>")];
    }

    let base = Style::default().fg(palette.text);
    let mut styles = vec![base; seq.len()];

    // 1. Codon highlighting via a non-overlapping left-to-right scan.
    let mut i = 0;
    while i + 3 <= seq.len() {
        match &seq[i..i + 3] {
            b"ATG" => {
                styles[i..i + 3]
                    .fill(Style::default().bg(palette.start_bg).fg(ratatui::style::Color::Black));
                i += 3;
            }
            b"TAA" | b"TAG" | b"TGA" => {
                styles[i..i + 3]
                    .fill(Style::default().bg(palette.stop_bg).fg(ratatui::style::Color::White));
                i += 3;
            }
            _ => i += 1,
        }
    }

    // 2. Motif matches override codon backgrounds.
    let qlen = app.search.query_len();
    if qlen > 0 {
        for &start in &app.search.matches {
            let end = (start + qlen).min(seq.len());
            styles[start..end].fill(
                Style::default()
                    .bg(palette.match_bg)
                    .fg(ratatui::style::Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        }
    }

    // 3. Cursor marker.
    if app.cursor < seq.len() {
        styles[app.cursor] = styles[app.cursor].add_modifier(Modifier::REVERSED);
    }

    // 4. Wrap into fixed-width rows.
    let mut lines = Vec::with_capacity(seq.len() / BASES_PER_ROW + 1);
    for start in (0..seq.len()).step_by(BASES_PER_ROW) {
        let end = (start + BASES_PER_ROW).min(seq.len());
        let spans: Vec<Span> = (start..end)
            .map(|k| Span::styled((seq[k] as char).to_string(), styles[k]))
            .collect();
        lines.push(Line::from(spans));
    }
    lines
}
