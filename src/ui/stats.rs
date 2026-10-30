//! The Stats view: base composition, dinucleotide observed/expected heatmap,
//! GC content and skew along the record, and a codon usage table.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Chart, Dataset, GraphType, LegendPosition, Paragraph},
    Frame,
};

use super::{fx, glow, panel, thousands};
use crate::app::App;
use crate::fasta::stats::BASES;
use crate::fasta::translate::codon_to_aa;

/// Window size (in bases) for the GC sliding-window plot.
const GC_WINDOW: usize = 50;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [top, bottom] = Layout::vertical([Constraint::Length(11), Constraint::Min(0)]).areas(area);
    let [comp, dinuc] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(top);
    // The codon table needs 56 columns; the chart takes the rest.
    let codon_w = if bottom.width >= 110 { 58 } else { 0 };
    let [chart, codons] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(codon_w)]).areas(bottom);

    render_composition(frame, comp, app);
    render_dinucleotides(frame, dinuc, app);
    render_gc_curve(frame, chart, app);
    if codon_w > 0 {
        render_codon_usage(frame, codons, app);
    }
}

fn render_composition(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let bases = &app.record().bases;
    let counts = app.cache.counts;
    let len = bases.len().max(1);
    let other = bases.len() - counts.total();

    let block = panel("▤", format!("Composition · {} bp", thousands(bases.len())), &p);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Bars grow in as the view opens.
    let grow = fx::ease_out(app.view_progress() * 1.4);
    let bar_w = (inner.width as usize).saturating_sub(22).max(4);
    let rows: [(char, usize); 5] = [
        ('A', counts.a),
        ('C', counts.c),
        ('G', counts.g),
        ('T', counts.t),
        ('N', other),
    ];
    let max = rows.iter().map(|r| r.1).max().unwrap_or(1).max(1);

    let mut lines = Vec::new();
    for (base, n) in rows {
        if base == 'N' && n == 0 {
            continue;
        }
        let color = p.base_color(base);
        let mut spans = vec![Span::styled(
            format!(" {base} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )];
        spans.extend(fx::hbar(
            bar_w,
            n as f64 / max as f64 * grow,
            fx::lerp(p.bg, color, 0.45),
            color,
            Some(p.surface),
        ));
        spans.push(Span::styled(format!(" {:>8}", thousands(n)), Style::default().fg(p.text)));
        spans.push(Span::styled(
            format!(" {:>5.1}%", n as f64 / len as f64 * 100.0),
            Style::default().fg(p.muted),
        ));
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(""));

    let gc = app.cache.gc;
    let skew = if counts.g + counts.c > 0 {
        (counts.g as f64 - counts.c as f64) / (counts.g + counts.c) as f64
    } else {
        0.0
    };
    let stat = |k: &str, v: String| {
        vec![
            Span::styled(format!(" {k} "), Style::default().fg(p.muted)),
            Span::styled(v, Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
        ]
    };
    let mut summary = Vec::new();
    summary.extend(stat("GC", format!("{:.1}%", gc * 100.0)));
    summary.extend(stat("AT", format!("{:.1}%", (1.0 - gc) * 100.0 - other as f64 / len as f64 * 100.0)));
    summary.extend(stat("GC skew", format!("{skew:+.3}")));
    lines.push(Line::from(summary));

    frame.render_widget(Paragraph::new(lines), inner);
}

/// A 4×4 grid of dinucleotide observed/expected ratios on a diverging ramp.
fn render_dinucleotides(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let block = panel("▦", "Dinucleotide obs/exp", &p).title(
        Line::from(Span::styled(" 5'→3' row·col ", Style::default().fg(p.muted))).right_aligned(),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width < 12 || inner.height < 5 {
        return;
    }
    let oe = app.cache.dinucleotides;

    let label_w = 2u16;
    let cell_w = ((inner.width - label_w) / 4).max(3);
    let cell_h = ((inner.height - 1) / 4).max(1);
    let buf = frame.buffer_mut();

    for (j, &b) in BASES.iter().enumerate() {
        let x = inner.x + label_w + j as u16 * cell_w + cell_w / 2;
        put(buf, x, inner.y, &(b as char).to_string(), Style::default().fg(p.base_color(b as char)).add_modifier(Modifier::BOLD));
    }
    for (i, &a) in BASES.iter().enumerate() {
        let y0 = inner.y + 1 + i as u16 * cell_h;
        put(buf, inner.x, y0 + (cell_h - 1) / 2, &(a as char).to_string(), Style::default().fg(p.base_color(a as char)).add_modifier(Modifier::BOLD));
        for (j, &b) in BASES.iter().enumerate() {
            let ratio = oe[i][j];
            // log2 ratio of ±1 (half / double) spans the ramp.
            let t = if ratio > 0.0 { 0.5 + ratio.log2() / 2.0 } else { 0.0 };
            let bg = fx::diverging(t, p.surface_hi);
            let fg = fx::contrast_text(bg);
            let x0 = inner.x + label_w + j as u16 * cell_w;
            for dy in 0..cell_h {
                for dx in 0..cell_w.saturating_sub(1) {
                    if let Some(c) = buf.cell_mut((x0 + dx, y0 + dy)) {
                        // The last row is a top half-block, leaving a half-row
                        // gutter between tiles.
                        if dy + 1 == cell_h && cell_h > 1 {
                            c.set_symbol("▀").set_fg(bg).set_bg(p.bg);
                        } else {
                            c.set_symbol(" ").set_bg(bg);
                        }
                    }
                }
            }
            let text = format!("{}{} {ratio:.2}", a as char, b as char);
            let text = if (text.len() as u16) < cell_w { text } else { format!("{ratio:.2}") };
            let tx = x0 + (cell_w.saturating_sub(1).saturating_sub(text.len() as u16)) / 2;
            let mut style = Style::default().fg(fg).bg(bg);
            // CpG is the dinucleotide people look for.
            if a == b'C' && b == b'G' {
                style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            }
            put(buf, tx, y0 + (cell_h - 1) / 2, &text, style);
        }
    }
}

fn put(buf: &mut Buffer, x: u16, y: u16, s: &str, style: Style) {
    if buf.area.contains(ratatui::layout::Position { x, y }) {
        buf.set_string(x, y, s, style);
    }
}

fn render_gc_curve(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let len = app.record_len();

    // One sample roughly per Braille column keeps the curve crisp.
    let samples = ((area.width as usize).saturating_sub(10) * 2).max(2);
    let (gc, skew) = app.gc_curves(GC_WINDOW, samples);
    // Skew lives in −1..1; map it onto the same 0..100 axis around 50.
    let skew: Vec<(f64, f64)> = skew
        .into_iter()
        .map(|(x, s)| (x, 50.0 + s * 50.0))
        .collect();
    let max_x = len.max(1) as f64;
    let mean = [(0.0, app.cache.gc * 100.0), (max_x, app.cache.gc * 100.0)];
    let cursor = [(app.cursor as f64, 0.0), (app.cursor as f64, 100.0)];

    let datasets = vec![
        Dataset::default()
            .name("mean")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.border))
            .data(&mean),
        Dataset::default()
            .name("skew")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(fx::lerp(p.bg, p.strand_b, 0.7)))
            .data(&skew),
        Dataset::default()
            .name("GC %")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.strand_a))
            .data(&gc),
        Dataset::default()
            .name("cursor")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(p.accent))
            .data(&cursor),
    ];

    let block = panel("∿", format!("GC content & skew · {GC_WINDOW} bp window"), &p);
    let axis_style = Style::default().fg(p.muted);
    let chart = Chart::new(datasets)
        .block(block)
        .style(Style::default().bg(p.bg))
        .legend_position(Some(LegendPosition::TopRight))
        .x_axis(
            Axis::default()
                .style(axis_style)
                .bounds([0.0, max_x])
                .labels(vec![
                    Span::raw("1"),
                    Span::raw(thousands(len / 2)),
                    Span::raw(thousands(len)),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(axis_style)
                .bounds([0.0, 100.0])
                .labels(vec![Span::raw("0"), Span::raw("50"), Span::raw("100")]),
        );
    frame.render_widget(chart, area);
    glow(frame, area, app);
}

/// The textbook codon table (first base by row block, second by column,
/// third within the block) shaded by how often each codon occurs.
fn render_codon_usage(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let usage = app.codon_usage();
    let total: usize = usage.iter().sum();
    let max = usage.iter().copied().max().unwrap_or(0).max(1);

    let block = panel("⌗", "Codon usage", &p).title(
        Line::from(Span::styled(
            format!(" {} · frame {} · {} codons ", if app.reverse { "−" } else { "+" }, app.frame + 1, thousands(total)),
            Style::default().fg(p.muted),
        ))
        .right_aligned(),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(
        std::iter::once(Span::raw("   "))
            .chain(BASES.iter().map(|&b| {
                Span::styled(
                    format!("{:^13}", b as char),
                    Style::default().fg(p.base_color(b as char)).add_modifier(Modifier::BOLD),
                )
            }))
            .collect::<Vec<_>>(),
    )];
    // Spacer rows between first-base blocks only when there is room.
    let spaced = inner.height >= 20;
    for (i, &first) in BASES.iter().enumerate() {
        for (k, &third) in BASES.iter().enumerate() {
            let label = if k == 1 { (first as char).to_string() } else { " ".into() };
            let mut spans = vec![Span::styled(
                format!(" {label} "),
                Style::default().fg(p.base_color(first as char)).add_modifier(Modifier::BOLD),
            )];
            for (j, &second) in BASES.iter().enumerate() {
                let n = usage[i * 16 + j * 4 + k];
                let codon = [first, second, third];
                let aa = codon_to_aa(&codon);
                let t = (n as f64 / max as f64).sqrt();
                let bg = if n == 0 { p.surface } else { fx::heat(0.15 + 0.85 * t) };
                let fg = if n == 0 { p.muted } else { fx::contrast_text(bg) };
                let mut style = Style::default().fg(fg).bg(bg);
                if aa == '*' || aa == 'M' {
                    style = style.add_modifier(Modifier::BOLD);
                }
                spans.push(Span::styled(
                    format!(" {} {} {:>4} ", String::from_utf8_lossy(&codon), aa, n),
                    style,
                ));
                spans.push(Span::raw(" "));
            }
            lines.push(Line::from(spans));
        }
        if spaced && i < 3 {
            lines.push(Line::from(""));
        }
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
