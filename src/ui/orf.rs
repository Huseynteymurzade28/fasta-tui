//! The ORF view: a six-frame table of open reading frames beside the protein
//! encoded by the highlighted one.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};

use super::scroll_window;
use crate::app::App;
use crate::fasta::Strand;
use crate::theme::Palette;

/// Residues per row in the protein pane.
const RESIDUES_PER_ROW: usize = 60;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_table(frame, cols[0], app);
    render_detail(frame, cols[1], app);
}

fn render_table(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let orfs = &app.orf.orfs;

    let title = format!(
        " ORFs — {} found, ≥{} aa  [↑/↓ select  Enter:go  m/M:min  y:copy] ",
        orfs.len(),
        app.orf.min_aa
    );

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!(
            " {:<4} {:<6} {:>10} {:>10} {:>7} {:>6}",
            "#", "frame", "start", "end", "nt", "aa"
        ),
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
    )));

    if orfs.is_empty() {
        lines.push(Line::from(Span::styled(
            "  no ORF meets the minimum length — press m to lower it",
            Style::default().fg(palette.text),
        )));
    }

    // Only build the rows that fit, keeping the selection inside the window.
    let rows = (area.height as usize).saturating_sub(3); // borders + header
    let (first, last) = scroll_window(app.orf.selected, orfs.len(), rows);
    for (i, orf) in orfs.iter().enumerate().take(last).skip(first) {
        let selected = i == app.orf.selected;
        let strand_color = match orf.strand {
            Strand::Forward => palette.strand_a,
            Strand::Reverse => palette.strand_b,
        };
        let text = format!(
            " {:<4} {:<6} {:>10} {:>10} {:>7} {:>6}{}",
            i + 1,
            orf.frame_label(),
            orf.start + 1,
            orf.end,
            orf.nt_len(),
            orf.aa_len,
            if orf.has_stop { "" } else { "  (no stop)" }
        );
        let style = if selected {
            Style::default()
                .bg(palette.accent)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(strand_color)
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

    let Some(orf) = app.orf.selected_orf() else {
        let empty = Paragraph::new("").block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Protein ")
                .border_style(Style::default().fg(palette.rung)),
        );
        frame.render_widget(empty, area);
        return;
    };

    let protein = orf.protein(&app.record().bases);
    let mut lines: Vec<Line> = Vec::new();

    let facts = [
        ("strand", match orf.strand {
            Strand::Forward => "forward (+)".to_string(),
            Strand::Reverse => "reverse (−)".to_string(),
        }),
        ("frame", orf.frame_label()),
        ("span", format!("{}..{} ({} nt)", orf.start + 1, orf.end, orf.nt_len())),
        ("length", format!("{} aa", orf.aa_len)),
        (
            "stop",
            if orf.has_stop {
                "present".to_string()
            } else {
                "runs off the end".to_string()
            },
        ),
    ];
    for (label, value) in facts {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {label:<8}"),
                Style::default().fg(palette.accent).add_modifier(Modifier::DIM),
            ),
            Span::styled(value, Style::default().fg(palette.text)),
        ]));
    }
    lines.push(Line::from(""));

    for chunk in protein.as_bytes().chunks(RESIDUES_PER_ROW) {
        let spans = chunk
            .iter()
            .map(|&aa| Span::styled((aa as char).to_string(), residue_style(aa as char, &palette)))
            .collect::<Vec<_>>();
        lines.push(Line::from(spans));
    }

    let detail = Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(" Encoded protein ")
                .border_style(Style::default().fg(palette.rung)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(detail, area);
}

fn residue_style(aa: char, palette: &Palette) -> Style {
    match aa {
        'M' => Style::default()
            .bg(palette.start_bg)
            .fg(Color::Black)
            .add_modifier(Modifier::BOLD),
        '*' => Style::default()
            .bg(palette.stop_bg)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
        'X' => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(palette.text),
    }
}
