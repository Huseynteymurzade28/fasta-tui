//! The Enzymes view: a restriction table with an inline cut map per enzyme,
//! the selected digest's details, and a virtual gel of the result.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Cell, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use super::gel::{self, Lane};
use super::{chip, fx, focus, hints, panel, scroll_window, thousands};
use crate::app::{App, ListHit};
use crate::fasta::enzymes;

/// Gel migration time, in ticks.
const RUN_TICKS: u64 = 22;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
    let [detail, gel_area] =
        Layout::vertical([Constraint::Length(10), Constraint::Min(0)]).areas(right);

    render_table(frame, left, app);
    render_detail(frame, detail, app);
    render_gel(frame, gel_area, app);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let rows_data = app.enzymes.visible();
    let cutters = app.enzymes.digests.iter().filter(|d| d.count() > 0).count();
    let unique = enzymes::unique_cutters(&app.enzymes.digests).len();
    let len = app.record_len().max(1);

    let block = panel("✂", format!("Restriction map · {cutters} cut · {unique} unique"), &p)
        .title_bottom(hints(
            &[
                ("⏎", "find sites"),
                ("x", "add to digest"),
                ("u", if app.enzymes.hide_zero { "show all" } else { "cutters only" }),
            ],
            &p,
        ));
    let inner = block.inner(area);

    let fixed = 2 + 2 + 9 + 11 + 5 + 7 + 6;
    let map_w = (inner.width as usize).saturating_sub(fixed).max(4);

    let body_h = inner.height.saturating_sub(1) as usize;
    let (first, last) = scroll_window(app.enzymes.selected, rows_data.len(), body_h);
    let rows: Vec<Row> = rows_data
        .iter()
        .enumerate()
        .take(last)
        .skip(first)
        .map(|(i, d)| {
            let marked = app.enzymes.marked.contains(&d.enzyme.name);
            let n = d.count();
            let name_style = if n == 1 {
                // Unique cutters are the ones worth spotting when cloning.
                Style::default().fg(p.base_a).add_modifier(Modifier::BOLD)
            } else if n == 0 {
                Style::default().fg(p.muted)
            } else {
                Style::default().fg(p.text)
            };

            // Inline map: a tick wherever this enzyme cuts.
            let mut cells = vec![false; map_w];
            for c in d.cuts() {
                cells[(c * map_w / len).min(map_w - 1)] = true;
            }
            let tick_color = if n == 1 { p.base_a } else { p.accent };
            let map: Vec<Span> = cells
                .iter()
                .map(|&hit| {
                    if hit {
                        Span::styled("┃", Style::default().fg(tick_color))
                    } else {
                        Span::styled("─", Style::default().fg(p.surface_hi))
                    }
                })
                .collect();

            let zebra = if i % 2 == 0 { p.bg } else { fx::lerp(p.bg, p.surface, 0.6) };
            Row::new(vec![
                Cell::from(if marked {
                    Span::styled("●", Style::default().fg(p.match_bg))
                } else {
                    Span::raw(" ")
                }),
                Cell::from(Span::styled(d.enzyme.name, name_style)),
                Cell::from(Span::styled(d.enzyme.site_with_cut(), Style::default().fg(p.muted))),
                Cell::from(Line::from(Span::styled(n.to_string(), name_style)).right_aligned()),
                Cell::from(Span::styled(
                    if d.enzyme.is_blunt() { "blunt" } else { "sticky" },
                    Style::default().fg(if d.enzyme.is_blunt() { p.base_c } else { p.base_g }),
                )),
                Cell::from(Line::from(map)),
            ])
            .style(Style::default().bg(zebra).fg(p.text))
        })
        .collect();

    let header = Row::new(["", "enzyme", "site", "cuts", "ends", "map"])
        .style(Style::default().fg(p.accent).add_modifier(Modifier::BOLD));
    let table = Table::new(
        rows,
        [
            Constraint::Length(1),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(4),
            Constraint::Length(6),
            Constraint::Fill(1),
        ],
    )
    .header(header)
    .block(block)
    .highlight_symbol(Span::styled("▶ ", Style::default().fg(p.accent)))
    .row_highlight_style(Style::default().bg(p.surface_hi).add_modifier(Modifier::BOLD));

    let selected = app.enzymes.selected.checked_sub(first).filter(|_| !rows_data.is_empty());
    let mut state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, area, &mut state);
    focus(frame, area, app);

    app.hits.borrow_mut().enzyme_table = Some(ListHit {
        body: Rect { y: inner.y + 1, height: inner.height.saturating_sub(1), ..inner },
        offset: first,
    });
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let Some(d) = app.enzymes.selected_digest() else {
        frame.render_widget(panel("✂", "Cut sites", &p), area);
        return;
    };

    let len = app.record_len();
    let cuts = d.cuts();
    let mut lines = vec![
        Line::from(vec![
            Span::raw(" "),
            chip(d.enzyme.site_with_cut(), p.surface_hi),
            Span::raw(" "),
            if d.enzyme.is_blunt() { chip("blunt", p.base_c) } else { chip("sticky", p.base_g) },
            Span::raw(" "),
            match d.count() {
                0 => chip("no sites", p.muted),
                1 => chip("unique cutter", p.base_a),
                n => chip(format!("{n} sites"), p.accent),
            },
        ]),
        Line::from(""),
    ];

    if cuts.is_empty() {
        lines.push(Line::from(Span::styled(" does not cut this record", Style::default().fg(p.muted))));
    } else {
        let label = |s: &str| Span::styled(format!(" {s:<10}"), Style::default().fg(p.muted));
        lines.push(Line::from(vec![
            label("cuts at"),
            Span::styled(
                cuts.iter().map(|c| thousands(c + 1)).collect::<Vec<_>>().join(", "),
                Style::default().fg(p.text),
            ),
        ]));
        // Longest fragment first is how a gel is read.
        let mut frags = d.fragments(len);
        frags.sort_unstable_by(|a, b| b.cmp(a));
        lines.push(Line::from(vec![
            label("fragments"),
            Span::styled(
                frags.iter().map(|f| format!("{} bp", thousands(*f))).collect::<Vec<_>>().join("  "),
                Style::default().fg(p.text),
            ),
        ]));
    }
    if !app.enzymes.marked.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" digest    ", Style::default().fg(p.muted)),
            Span::styled(app.enzymes.marked.join(" + "), Style::default().fg(p.match_bg).add_modifier(Modifier::BOLD)),
        ]));
    }

    let detail = Paragraph::new(lines)
        .block(panel("✂", d.enzyme.name, &p))
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}

fn render_gel(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let len = app.record_len();
    let block = panel("▥", "Virtual gel · 1% agarose", &p).title(
        Line::from(Span::styled(" log scale · linear molecule ", Style::default().fg(p.muted))).right_aligned(),
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lanes = vec![
        Lane { label: "ladder".into(), bands: gel::ladder_for(len), ladder: true },
        Lane { label: "uncut".into(), bands: vec![len], ladder: false },
    ];
    if let Some(d) = app.enzymes.selected_digest() {
        lanes.push(Lane { label: d.enzyme.name.into(), bands: d.fragments(len), ladder: false });
    }
    if !app.enzymes.marked.is_empty() {
        let cuts = app.enzymes.combined_cuts();
        let mut frags = Vec::with_capacity(cuts.len() + 1);
        let mut prev = 0;
        for c in cuts {
            let c = c.min(len);
            frags.push(c - prev);
            prev = c;
        }
        frags.push(len - prev);
        frags.retain(|&f| f > 0);
        lanes.push(Lane { label: "digest".into(), bands: frags, ladder: false });
    }

    let run = fx::ease_out(app.tick.saturating_sub(app.enzymes.changed_at) as f64 / RUN_TICKS as f64);
    gel::render(frame.buffer_mut(), inner, &lanes, run, &p);
}
