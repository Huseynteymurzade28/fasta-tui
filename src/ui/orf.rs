//! The ORF view: a sortable-looking table with inline length bars, a
//! six-frame map of every ORF along the record, and the selected ORF's
//! details and protein.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::{chip, fx, focus, hints, panel, protein::residue_style, scroll_window, thousands};
use crate::app::{App, ListHit};
use crate::fasta::aa::{self, AaClass};
use crate::fasta::orf::Orf;
use crate::fasta::Strand;
use crate::theme::Palette;

/// Residues per group in the protein pane.
const GROUP: usize = 10;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)]).areas(area);
    let [table, map] = Layout::vertical([Constraint::Min(0), Constraint::Length(8)]).areas(left);

    render_table(frame, table, app);
    render_six_frames(frame, map, app);
    render_detail(frame, right, app);
}

fn strand_color(orf: &Orf, p: &Palette) -> ratatui::style::Color {
    match orf.strand {
        Strand::Forward => p.strand_a,
        Strand::Reverse => p.strand_b,
    }
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let orfs = &app.orf.orfs;

    let block = panel("⇶", format!("ORFs · {} found · ≥{} aa", orfs.len(), app.orf.min_aa), &p)
        .title_bottom(hints(&[("⏎", "open in reader"), ("m/M", "min length"), ("y", "copy protein")], &p));
    let inner = block.inner(area);

    if orfs.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  no ORF meets the minimum length — press ", Style::default().fg(p.muted)),
                Span::styled("m", Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
                Span::styled(" to lower it", Style::default().fg(p.muted)),
            ]))
            .block(block),
            area,
        );
        return;
    }

    let fixed = 2 + 4 + 5 + 17 + 6 + 3 + 6;
    let bar_w = (inner.width as usize).saturating_sub(fixed).max(4);
    let max_aa = orfs.iter().map(|o| o.aa_len).max().unwrap_or(1).max(1);

    let body_h = inner.height.saturating_sub(1) as usize;
    let (first, last) = scroll_window(app.orf.selected, orfs.len(), body_h);
    let rows: Vec<Row> = orfs
        .iter()
        .enumerate()
        .take(last)
        .skip(first)
        .map(|(i, orf)| {
            let color = strand_color(orf, &p);
            let bar = fx::hbar(bar_w, orf.aa_len as f64 / max_aa as f64, fx::lerp(p.bg, color, 0.55), None);
            let zebra = if i % 2 == 0 { p.bg } else { fx::lerp(p.bg, p.surface, 0.6) };
            Row::new(vec![
                Cell::from(Span::styled(format!("{}", i + 1), Style::default().fg(p.muted))),
                Cell::from(Span::styled(
                    orf.frame_label(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )),
                Cell::from(format!("{}..{}", thousands(orf.start + 1), thousands(orf.end))),
                Cell::from(Line::from(format!("{}", orf.aa_len)).right_aligned()),
                Cell::from(Line::from(bar)),
                Cell::from(if orf.has_stop {
                    Span::styled("■", Style::default().fg(p.stop_bg))
                } else {
                    Span::styled("…", Style::default().fg(p.muted))
                }),
            ])
            .style(Style::default().bg(zebra).fg(p.text))
        })
        .collect();

    let header = Row::new(["#", "frame", "span", "aa", "length", "stop"])
        .style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD));
    let table = Table::new(
        rows,
        [
            Constraint::Length(4),
            Constraint::Length(5),
            Constraint::Length(17),
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Length(4),
        ],
    )
    .header(header)
    .block(block)
    .highlight_symbol(Span::styled("▶ ", Style::default().fg(p.accent)))
    .row_highlight_style(Style::default().bg(p.surface_hi).add_modifier(Modifier::BOLD));

    let mut state = TableState::default().with_selected(Some(app.orf.selected - first));
    frame.render_stateful_widget(table, area, &mut state);
    focus(frame, area, app);

    app.hits.borrow_mut().orf_table = Some(ListHit {
        body: Rect { y: inner.y + 1, height: inner.height.saturating_sub(1), ..inner },
        offset: first,
    });
}

/// Six lanes, one per frame, each ORF drawn as a bar with an arrowhead
/// pointing along its strand.
fn render_six_frames(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let len = app.record_len();
    let block = panel("☰", "Six-frame map", &p);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width < 8 || inner.height < 6 || len == 0 {
        return;
    }

    let labels = ["+1", "+2", "+3", "−1", "−2", "−3"];
    let lanes = Rect { x: inner.x + 4, width: inner.width - 4, height: 6, ..inner };
    app.hits.borrow_mut().six_frame = Some(lanes);
    let w = lanes.width as usize;
    let to_x = |pos: usize| (pos * w / len).min(w - 1);
    let cursor_x = to_x(app.cursor);
    let selected = app.orf.selected_orf().copied();

    let buf = frame.buffer_mut();
    for (lane, label) in labels.iter().enumerate() {
        let y = lanes.y + lane as u16;
        let color = if lane < 3 { p.strand_a } else { p.strand_b };
        buf.set_string(inner.x, y, format!("{label:<3}"), Style::default().fg(fx::lerp(p.bg, color, 0.8)));
        for x in 0..w {
            let glyph = if x == cursor_x { "┊" } else { "·" };
            let fg = if x == cursor_x { p.accent } else { p.surface_hi };
            buf.set_string(lanes.x + x as u16, y, glyph, Style::default().fg(fg));
        }
    }

    let mut orfs: Vec<&Orf> = app.orf.orfs.iter().collect();
    orfs.sort_by_key(|o| Some(**o) == selected);
    for orf in orfs {
        let lane = match orf.strand {
            Strand::Forward => orf.frame,
            Strand::Reverse => 3 + orf.frame,
        };
        let y = lanes.y + lane as u16;
        let (x0, x1) = (to_x(orf.start), to_x(orf.end.saturating_sub(1)));
        let is_sel = Some(*orf) == selected;
        let base = strand_color(orf, &p);
        let color = if is_sel { p.accent } else { fx::lerp(p.bg, base, 0.75) };
        for x in x0..=x1 {
            let glyph = match (orf.strand, x) {
                (Strand::Forward, x) if x == x1 && x1 > x0 => "▶",
                (Strand::Reverse, x) if x == x0 && x1 > x0 => "◀",
                _ if is_sel => "█",
                _ => "━",
            };
            buf.set_string(lanes.x + x as u16, y, glyph, Style::default().fg(color));
        }
    }
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();

    let Some(orf) = app.orf.selected_orf() else {
        frame.render_widget(panel("⬡", "Encoded protein", &p), area);
        return;
    };

    let protein = orf.protein(&app.record().bases);
    let residues: Vec<char> = protein.chars().collect();
    let color = strand_color(orf, &p);
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::raw(" "),
        chip(format!("{} frame {}", orf.strand.symbol(), orf.frame + 1), color),
        Span::raw(" "),
        chip(format!("{} aa", orf.aa_len), p.surface_hi),
        Span::raw(" "),
        if orf.has_stop {
            chip("stop ✓", p.stop_bg)
        } else {
            chip("runs off end", p.muted)
        },
    ]));
    lines.push(Line::from(""));

    let kv = |k: &str, v: String| {
        Line::from(vec![
            Span::styled(format!(" {k:<11}"), Style::default().fg(p.muted)),
            Span::styled(v, Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        ])
    };
    lines.push(kv("span", format!("{}..{}", thousands(orf.start + 1), thousands(orf.end))));
    lines.push(kv("length", format!("{} nt", thousands(orf.nt_len()))));
    lines.push(kv("mass", format!("~{:.1} kDa", orf.aa_len as f64 * aa::AVG_RESIDUE_DA / 1000.0)));
    let gravy = if orf.aa_len > 0 {
        residues.iter().filter(|&&a| a != '*').map(|&a| aa::hydropathy(a)).sum::<f64>() / orf.aa_len as f64
    } else {
        0.0
    };
    lines.push(kv("GRAVY", format!("{gravy:+.2}")));

    // Where the ORF sits in the record.
    let track_w = area.width.saturating_sub(16) as usize;
    let len = app.record_len().max(1);
    let (x0, x1) = (orf.start * track_w / len, (orf.end * track_w).div_ceil(len));
    let track: Vec<Span> = (0..track_w)
        .map(|x| {
            if x >= x0 && x < x1.max(x0 + 1) {
                Span::styled("█", Style::default().fg(color))
            } else {
                Span::styled("─", Style::default().fg(p.border))
            }
        })
        .collect();
    let mut where_line = vec![Span::styled(format!(" {:<11}", "position"), Style::default().fg(p.muted))];
    where_line.extend(track);
    lines.push(Line::from(where_line));

    // Class composition as a stacked bar.
    let mut comp = vec![Span::styled(format!(" {:<11}", "classes"), Style::default().fg(p.muted))];
    let total = residues.iter().filter(|&&a| a != '*').count().max(1);
    let mut used = 0;
    for class in AaClass::LEGEND {
        let n = residues.iter().filter(|&&a| AaClass::of(a) == class).count();
        let cells = (n * track_w).div_ceil(total).min(track_w - used.min(track_w));
        used += cells;
        comp.push(Span::styled("█".repeat(cells), Style::default().fg(p.aa_color(class))));
    }
    lines.push(Line::from(comp));
    lines.push(Line::from(""));

    // The protein, grouped in tens.
    let per_row = ((area.width.saturating_sub(4) as usize + 1) / (GROUP + 1)).max(1) * GROUP;
    for chunk in residues.chunks(per_row) {
        let mut spans = vec![Span::raw(" ")];
        for (i, &res) in chunk.iter().enumerate() {
            if i > 0 && i % GROUP == 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(res.to_string(), residue_style(res, &p)));
        }
        lines.push(Line::from(spans));
    }

    let detail = Paragraph::new(lines)
        .block(panel("⬡", format!("ORF {}", app.orf.selected + 1), &p))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}
