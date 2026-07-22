//! The help overlay listing every key binding.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use super::centered_rect;
use crate::app::App;

/// `(keys, description)` pairs; an empty key string starts a new section whose
/// description is the heading.
const BINDINGS: &[(&str, &str)] = &[
    ("", "Navigation"),
    ("←/h, →/l", "Move cursor one base"),
    ("↑/k, ↓/j", "Move cursor one row"),
    ("PgUp / PgDn", "Move one page"),
    ("Home/g, End/G", "Jump to start / end"),
    ("Tab / ]", "Next FASTA record"),
    ("BackTab / [", "Previous FASTA record"),
    ("", "Views"),
    ("1 … 5", "Reader / Stats / Protein / ORFs / Enzymes"),
    ("r", "Toggle reverse-complement strand"),
    ("f", "Cycle reading frame 0/1/2"),
    ("t", "Cycle color theme"),
    ("", "Search"),
    ("/", "Search a motif (any IUPAC code)"),
    ("n / N", "Jump to next / previous match"),
    ("b", "Toggle reverse-complement matching"),
    ("", "Selection"),
    ("v", "Start / drop a selection at the cursor"),
    ("y", "Copy selection (or ORF) to the clipboard"),
    ("", "ORF table (view 4)"),
    ("↑ / ↓", "Select an ORF"),
    ("Enter", "Show it in the reader, selected"),
    ("m / M", "Lower / raise the minimum ORF length"),
    ("", "Enzyme table (view 5)"),
    ("↑ / ↓", "Select an enzyme"),
    ("Enter", "Find its sites in the reader"),
    ("u", "Show only enzymes that cut"),
    ("", "Helix"),
    ("Space", "Pause / resume helix spin"),
    ("+ / -", "Helix spin faster / slower"),
    ("> / <", "More / less helix twist"),
    ("", "Other"),
    ("?", "Toggle this help"),
    ("Esc", "Close help / drop selection / clear search"),
    ("q", "Quit"),
];

pub fn render(frame: &mut Frame, app: &App) {
    let palette = app.theme.palette();
    // The list is long enough that the overlay wants most of the screen.
    let area = centered_rect(64, 94, frame.area());

    let mut lines: Vec<Line> = Vec::with_capacity(BINDINGS.len() + 2);
    for (keys, desc) in BINDINGS {
        if keys.is_empty() {
            lines.push(Line::from(Span::styled(
                format!(" {desc}"),
                Style::default()
                    .fg(palette.match_bg)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }
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
