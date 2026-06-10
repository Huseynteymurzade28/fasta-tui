//! The help overlay listing every key binding.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
use crate::app::App;

const BINDINGS: &[(&str, &str)] = &[
    ("←/h, →/l", "Move cursor one base"),
    ("↑/k, ↓/j", "Move cursor one row"),
    ("PgUp / PgDn", "Move one page"),
    ("Home/g, End/G", "Jump to start / end"),
    ("1 / 2 / 3", "Switch view: Reader / Stats / Protein"),
    ("Tab / ]", "Next FASTA record"),
    ("BackTab / [", "Previous FASTA record"),
    ("/", "Search a motif (A/T/G/C)"),
    ("n / N", "Jump to next / previous match"),
    ("Space", "Pause / resume helix spin"),
    ("+ / -", "Helix spin faster / slower"),
    ("> / <", "More / less helix twist"),
    ("r", "Toggle reverse-complement (Protein)"),
    ("f", "Cycle reading frame 0/1/2"),
    ("t", "Cycle color theme"),
    ("?", "Toggle this help"),
    ("q / Esc", "Quit"),
];

pub fn render(frame: &mut Frame, app: &App) {
    let palette = app.theme.palette();
    let area = centered_rect(60, 70, frame.area());

    let mut lines: Vec<Line> = Vec::with_capacity(BINDINGS.len() + 2);
    lines.push(Line::from(""));
    for (keys, desc) in BINDINGS {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {keys:<14}"),
                Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc.to_string(), Style::default().fg(palette.text)),
        ]));
    }

    let popup = Paragraph::new(lines).block(
        Block::bordered()
            .border_type(BorderType::Rounded)
            .title(" Key Bindings — press ? or Esc to close ")
            .border_style(Style::default().fg(palette.accent)),
    );

    frame.render_widget(Clear, area);
    frame.render_widget(popup, area);
}
