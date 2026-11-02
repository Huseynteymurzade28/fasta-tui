//! The Protein view: a hydropathy profile above the translated sequence,
//! residues colored by physicochemical class and tied to the reader cursor.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

use super::{fx, focus, panel, scroll_window, thousands};
use crate::app::{App, ReaderMetrics, Translation};
use crate::fasta::aa::{self, AaClass};
use crate::theme::Palette;

/// Residues grouped like the reader groups bases.
const GROUP: usize = 10;
/// Hydropathy window, in residues (Kyte & Doolittle's usual choice).
const KD_WINDOW: usize = 9;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let tr = app.translation(KD_WINDOW);
    let residues = &tr.residues;
    let cursor = app.cursor_residue().filter(|&r| r < residues.len());

    let chart_h = if area.height >= 24 { 11 } else { 0 };
    let [chart, body] =
        Layout::vertical([Constraint::Length(chart_h), Constraint::Min(0)]).areas(area);
    if chart_h > 0 {
        render_hydropathy(frame, chart, residues, &tr.hydropathy, cursor, &p);
    }
    render_sequence(frame, body, app, &tr, cursor, &p);
}

fn render_hydropathy(
    frame: &mut Frame,
    area: Rect,
    residues: &[char],
    raw: &[(f64, f64)],
    cursor: Option<usize>,
    p: &Palette,
) {
    // Resample to the chart's Braille resolution so the bars form a solid
    // area instead of a sparse comb.
    let cols = (area.width as usize).saturating_sub(10) * 2;
    let profile: Vec<(f64, f64)> = if raw.is_empty() || cols == 0 {
        raw.to_vec()
    } else {
        let max_x = residues.len() as f64;
        (0..cols)
            .map(|i| {
                let x = i as f64 / cols as f64 * max_x;
                let j = raw.partition_point(|(rx, _)| *rx < x).min(raw.len() - 1);
                (x, raw[j].1)
            })
            .collect()
    };
    // Split into two series so hydrophobic stretches read warm.
    let (pos, neg): (Vec<_>, Vec<_>) = profile.iter().partition(|(_, v)| *v >= 0.0);
    let max_x = residues.len().max(1) as f64;
    let cursor_line = cursor.map(|c| [(c as f64, -4.5), (c as f64, 4.5)]);
    let zero = [(0.0, 0.0), (max_x, 0.0)];

    let mut datasets = vec![
        Dataset::default()
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.border))
            .data(&zero),
        Dataset::default()
            .name("hydrophobic")
            .marker(Marker::Braille)
            .graph_type(GraphType::Bar)
            .style(Style::default().fg(p.base_g))
            .data(&pos),
        Dataset::default()
            .name("hydrophilic")
            .marker(Marker::Braille)
            .graph_type(GraphType::Bar)
            .style(Style::default().fg(p.base_c))
            .data(&neg),
    ];
    if let Some(line) = &cursor_line {
        datasets.push(
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(p.accent))
                .data(line),
        );
    }

    let block = panel("∿", format!("Hydropathy · Kyte–Doolittle, {KD_WINDOW}-residue window"), p);
    let chart = Chart::new(datasets)
        .block(block)
        .style(Style::default().bg(p.bg))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(p.muted))
                .bounds([0.0, max_x])
                .labels(vec![Span::raw("1"), Span::raw(thousands(residues.len()))]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(p.muted))
                .bounds([-4.5, 4.5])
                .labels(vec![Span::raw("-4.5"), Span::raw("0"), Span::raw("+4.5")]),
        );
    frame.render_widget(chart, area);
}

fn render_sequence(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    tr: &Translation,
    cursor: Option<usize>,
    p: &Palette,
) {
    let residues = &tr.residues;
    let (stops, hydrophobic) = (tr.stops, tr.hydrophobic);
    let pct = |n: usize| n as f64 / residues.len().max(1) as f64 * 100.0;

    let mut legend = vec![Span::raw(" ")];
    for class in AaClass::LEGEND {
        legend.push(Span::styled("■ ", Style::default().fg(p.aa_color(class))));
        legend.push(Span::styled(format!("{}  ", class.label()), Style::default().fg(p.muted)));
    }
    legend.push(Span::styled(" M ", Style::default().fg(fx::contrast_text(p.start_bg)).bg(p.start_bg)));
    legend.push(Span::styled(" start  ", Style::default().fg(p.muted)));
    legend.push(Span::styled(" * ", Style::default().fg(fx::contrast_text(p.stop_bg)).bg(p.stop_bg)));
    legend.push(Span::styled(" stop ", Style::default().fg(p.muted)));

    let block = panel(
        "⬡",
        format!(
            "Protein · {} strand · frame {}",
            if app.reverse { "reverse" } else { "forward" },
            app.frame + 1
        ),
        p,
    )
    .title(
        Line::from(Span::styled(
            format!(
                " {} aa · {} stops · {:.0}% hydrophobic · ~{:.1} kDa ",
                thousands(residues.len()),
                stops,
                pct(hydrophobic),
                residues.len() as f64 * aa::AVG_RESIDUE_DA / 1000.0
            ),
            Style::default().fg(p.muted),
        ))
        .right_aligned(),
    )
    .title_bottom(Line::from(legend));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    focus(frame, area, app);

    if residues.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("<sequence too short to translate>", Style::default().fg(p.muted))),
            inner,
        );
        return;
    }

    let digits = residues.len().to_string().len().max(4);
    let usable = (inner.width as usize).saturating_sub(digits + 3);
    let groups = ((usable + 1) / (GROUP + 1)).max(1);
    let per_row = groups * GROUP;
    let prev = app.metrics.get();
    app.metrics.set(ReaderMetrics { residues_per_row: per_row, ..prev });

    let total_rows = residues.len().div_ceil(per_row);
    let cursor_row = cursor.map_or(0, |c| c / per_row);
    let (first, last) = scroll_window(cursor_row, total_rows, inner.height as usize);

    let mut lines = Vec::with_capacity(last - first);
    for row in first..last {
        let start = row * per_row;
        let end = (start + per_row).min(residues.len());
        let on_row = row == cursor_row;
        let mut spans = vec![
            Span::styled(
                format!("{:>digits$} ", start + 1),
                Style::default().fg(if on_row { p.accent } else { p.muted }),
            ),
            Span::styled("│ ", Style::default().fg(p.border)),
        ];
        for (k, &res) in residues.iter().enumerate().take(end).skip(start) {
            if k > start && (k - start) % GROUP == 0 {
                spans.push(Span::raw(" "));
            }
            let mut style = residue_style(res, p);
            if Some(k) == cursor {
                style = Style::default().fg(fx::contrast_text(p.accent)).bg(p.accent).add_modifier(Modifier::BOLD);
            } else if on_row && style.bg.is_none() {
                style = style.bg(p.surface);
            }
            spans.push(Span::styled(res.to_string(), style));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

/// Class color for a residue; start and stop get solid backgrounds.
pub fn residue_style(aa: char, p: &Palette) -> Style {
    match aa {
        'M' => Style::default()
            .bg(p.start_bg)
            .fg(fx::contrast_text(p.start_bg))
            .add_modifier(Modifier::BOLD),
        '*' => Style::default()
            .bg(p.stop_bg)
            .fg(fx::contrast_text(p.stop_bg))
            .add_modifier(Modifier::BOLD),
        _ => Style::default().fg(p.aa_color(AaClass::of(aa))),
    }
}
