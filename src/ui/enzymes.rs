//! The Enzymes view: a restriction map of the current record.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};

use super::scroll_window;
use crate::app::App;
use crate::fasta::enzymes;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_table(frame, cols[0], app);
    render_detail(frame, cols[1], app);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let rows_data = app.enzymes.visible();
    let cutters = app.enzymes.digests.iter().filter(|d| d.count() > 0).count();
    let unique = enzymes::unique_cutters(&app.enzymes.digests).len();

    let title = format!(
        " Restriction map — {cutters} cut, {unique} unique  [↑/↓  Enter:find  u:{}] ",
        if app.enzymes.hide_zero { "show all" } else { "cutters only" }
    );

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(" {:<9} {:<11} {:>6} {:>8}", "enzyme", "site", "sites", "ends"),
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
    )));

    let rows = (area.height as usize).saturating_sub(3);
    let (first, last) = scroll_window(app.enzymes.selected, rows_data.len(), rows);
    for (i, d) in rows_data.iter().enumerate().take(last).skip(first) {
        let selected = i == app.enzymes.selected;
        let text = format!(
            " {:<9} {:<11} {:>6} {:>8}",
            d.enzyme.name,
            d.enzyme.site_with_cut(),
            d.count(),
            if d.enzyme.is_blunt() { "blunt" } else { "sticky" }
        );
        let style = if selected {
            Style::default()
                .bg(palette.accent)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else if d.count() == 1 {
            // Unique cutters are the ones worth spotting when cloning.
            Style::default().fg(palette.base_a).add_modifier(Modifier::BOLD)
        } else if d.count() == 0 {
            Style::default().fg(palette.text).add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(palette.text)
        };
        lines.push(Line::from(Span::styled(text, style)));
    }

    let table = Paragraph::new(lines).block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(title)
            .border_style(Style::default().fg(palette.rung)),
    );
    frame.render_widget(table, area);
}

fn render_detail(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let key = |s: String| {
        Span::styled(
            s,
            Style::default().fg(palette.accent).add_modifier(Modifier::DIM),
        )
    };
    let val = |s: String| Span::styled(s, Style::default().fg(palette.text));

    let Some(d) = app.enzymes.selected_digest() else {
        frame.render_widget(
            Paragraph::new("").block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title(" Cut sites "),
            ),
            area,
        );
        return;
    };

    let len = app.record_len();
    let cuts = d.cuts();
    let frags = d.fragments(len);
    let mut lines = vec![
        Line::from(vec![key(" site    ".into()), val(d.enzyme.site_with_cut())]),
        Line::from(vec![
            key(" ends    ".into()),
            val(if d.enzyme.is_blunt() {
                "blunt".into()
            } else {
                "sticky (overhang)".into()
            }),
        ]),
        Line::from(vec![key(" sites   ".into()), val(d.count().to_string())]),
        Line::from(""),
    ];

    if cuts.is_empty() {
        lines.push(val(" does not cut this record".into()).into());
    } else {
        lines.push(Line::from(vec![
            key(" cuts at ".into()),
            val(
                cuts.iter()
                    .map(|c| (c + 1).to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            key(" fragments (linear) ".into()),
            val(format!("{} pieces", frags.len())),
        ]));
        // Longest fragment first is how a gel would be read.
        let mut sorted = frags.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        lines.push(Line::from(val(format!(
            "   {}",
            sorted
                .iter()
                .map(|f| format!("{f} bp"))
                .collect::<Vec<_>>()
                .join("  ")
        ))));
    }

    let detail = Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(format!(" {} ", d.enzyme.name))
                .border_style(Style::default().fg(palette.rung)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}
