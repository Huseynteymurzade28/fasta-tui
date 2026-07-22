//! Left/right split of the Reader view: GC gauge + sequence text + helix.

use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use super::helix;
use crate::app::{App, ReaderMetrics};
use crate::fasta::stats;
use crate::theme::Palette;

/// Bases grouped together with a blank between groups, like a genome browser.
const GROUP: usize = 10;
/// Width of the left position gutter: `"%6d "` number + `"│ "` separator.
const GUTTER: usize = 9;

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
    // The selection panel only claims height while a selection exists.
    let sel_height = if app.selection.active { 3 } else { 0 };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(sel_height),
        ])
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
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(gauge_title),
        )
        .gauge_style(Style::default().fg(palette.accent))
        .ratio(gc)
        .label(format!("{:.1}%  ({} bases)", gc * 100.0, app.record_len()));
    frame.render_widget(gauge, rows[0]);

    // --- Sequence reader ----------------------------------------------------
    // Size each row to the available width so it never wraps (wrapping would
    // throw off the row-based scroll below). We pack as many 10-base groups as
    // fit: a row of `g` groups is `GUTTER + 11*g - 1` columns wide. One inner
    // column is reserved on the right for the scrollbar.
    let inner_w = (rows[1].width.saturating_sub(2) as usize).saturating_sub(1);
    let groups = inner_w.saturating_sub(GUTTER - 1) / (GROUP + 1);
    let bases_per_row = groups.max(1) * GROUP;
    let visible_rows = rows[1].height.saturating_sub(2) as usize; // minus borders

    // Publish the measured geometry so navigation keys can move by row / page.
    app.metrics.set(ReaderMetrics {
        bases_per_row,
        visible_rows,
    });

    let lines = build_reader_lines(app, &palette, bases_per_row);
    let total_rows = lines.len();
    let title = " Sequence  (A·T·G·C colored, ATG=start, TAA/TAG/TGA=stop, motif=highlight) ";
    let reader = Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(title)
                .border_style(Style::default().fg(palette.rung)),
        )
        .scroll((app.scroll_top as u16, 0));
    frame.render_widget(reader, rows[1]);

    // Scrollbar tracking the cursor's position through the whole record.
    if total_rows > visible_rows {
        let mut sb_state = ScrollbarState::new(total_rows)
            .viewport_content_length(visible_rows)
            .position(app.scroll_top);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .thumb_style(Style::default().fg(palette.accent))
            .track_style(Style::default().fg(palette.rung_back));
        frame.render_stateful_widget(
            scrollbar,
            rows[1].inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut sb_state,
        );
    }

    if app.selection.active {
        render_selection_panel(frame, rows[2], app);
    }
}

/// The oligo panel under the reader: length, composition and melting
/// temperature for whatever the user has selected.
fn render_selection_panel(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let Some((lo, hi)) = app.selection.range(app.cursor) else {
        return;
    };
    let hi = hi.min(app.record_len());

    let title = format!(
        " Selection — {}..{} on the {} strand  [y: copy  Esc: clear] ",
        lo + 1,
        hi,
        if app.reverse { "reverse" } else { "forward" }
    );

    let line = match app.selection_stats() {
        Some(s) => {
            let key = |t: &str| {
                Span::styled(
                    format!(" {t} "),
                    Style::default().fg(palette.accent).add_modifier(Modifier::DIM),
                )
            };
            let val = |t: String| {
                Span::styled(t, Style::default().fg(palette.text).add_modifier(Modifier::BOLD))
            };
            // The nearest-neighbour Tm is the one to trust; Wallace is shown as
            // a sanity check and is all we have for ambiguous sequence.
            let tm = match s.tm_nn {
                Some(tm) => format!("{tm:.1} °C (NN)"),
                None => format!("{:.1} °C (Wallace)", s.tm_wallace),
            };
            let mut spans = vec![
                key("len"),
                val(format!("{} nt", s.len)),
                key("  GC"),
                val(format!("{:.1}%", s.gc * 100.0)),
                key("  Tm"),
                val(tm),
                key("  MW"),
                val(format!("{:.1} kDa", s.mw / 1000.0)),
            ];
            if s.ambiguous > 0 {
                spans.push(key("  ambiguous"));
                spans.push(val(format!("{}", s.ambiguous)));
            }
            Line::from(spans)
        }
        None => Line::from(Span::styled(
            " empty selection",
            Style::default().fg(palette.text),
        )),
    };

    let panel = Paragraph::new(line).block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(palette.match_bg)),
    );
    frame.render_widget(panel, area);
}

/// Build codon- and motif-highlighted rows for the current record.
fn build_reader_lines(app: &App, palette: &Palette, bases_per_row: usize) -> Vec<Line<'static>> {
    use ratatui::style::Color;

    let seq = &app.record().bases;
    if seq.is_empty() {
        return vec![Line::from("<empty sequence>")];
    }

    // 1. Start every base from its nucleotide color so A/T/G/C read apart at a
    //    glance (the same palette the helix uses). Highlights layer on top.
    let mut styles: Vec<Style> = seq
        .iter()
        .map(|&b| Style::default().fg(palette.base_color(b as char)))
        .collect();

    // 2. Codon highlighting via a non-overlapping left-to-right scan.
    let mut i = 0;
    while i + 3 <= seq.len() {
        match &seq[i..i + 3] {
            b"ATG" => {
                styles[i..i + 3].fill(
                    Style::default()
                        .bg(palette.start_bg)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                );
                i += 3;
            }
            b"TAA" | b"TAG" | b"TGA" => {
                styles[i..i + 3].fill(
                    Style::default()
                        .bg(palette.stop_bg)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
                i += 3;
            }
            _ => i += 1,
        }
    }

    // 3. Motif matches override codon backgrounds. Minus-strand hits are
    //    underlined so they read apart from plus-strand ones.
    let active = app.search.active_match();
    for m in &app.search.matches {
        let end = m.end().min(seq.len());
        let mut style = Style::default()
            .bg(palette.match_bg)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD);
        if m.revcomp {
            style = style.add_modifier(Modifier::UNDERLINED);
        }
        // The hit n/N is currently sitting on gets the accent background.
        if Some(*m) == active {
            style = style.bg(palette.accent);
        }
        styles[m.start..end].fill(style);
    }

    // 4. The selection dims everything it does not cover, so the picked range
    //    stands out without repainting it.
    if let Some((lo, hi)) = app.selection.range(app.cursor) {
        let hi = hi.min(seq.len());
        for (i, style) in styles.iter_mut().enumerate() {
            if i < lo || i >= hi {
                *style = style.add_modifier(Modifier::DIM);
            } else {
                *style = style.add_modifier(Modifier::UNDERLINED);
            }
        }
    }

    // 5. Cursor marker.
    if app.cursor < seq.len() {
        styles[app.cursor] = styles[app.cursor].add_modifier(Modifier::REVERSED);
    }

    // 6. Emit fixed-width rows: a position gutter, then 10-base groups split by
    //    a thin space so the eye can count along the strand.
    let gutter_style = Style::default().fg(palette.accent).add_modifier(Modifier::DIM);
    let sep_style = Style::default().fg(palette.rung);

    let mut lines = Vec::with_capacity(seq.len() / bases_per_row + 1);
    for start in (0..seq.len()).step_by(bases_per_row) {
        let end = (start + bases_per_row).min(seq.len());
        let mut spans = Vec::with_capacity(bases_per_row + bases_per_row / GROUP + 2);

        // Left gutter: 1-based position of the row's first base.
        spans.push(Span::styled(format!("{:>6} ", start + 1), gutter_style));
        spans.push(Span::styled("│ ", sep_style));

        for k in start..end {
            if k > start && (k - start) % GROUP == 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled((seq[k] as char).to_string(), styles[k]));
        }
        lines.push(Line::from(spans));
    }
    lines
}
