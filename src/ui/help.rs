//! The help overlay listing every key binding, laid out in two columns.

use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use super::{centered, focus};
use crate::app::App;

/// `(keys, description)` pairs; an empty key string starts a new section whose
/// description is the heading.
const LEFT: &[(&str, &str)] = &[
    ("", "Navigation"),
    ("←/h  →/l", "Move one base / residue"),
    ("↑/k  ↓/j", "Move one row"),
    ("PgUp PgDn", "Move one page"),
    ("g  G", "Jump to start / end"),
    ("Tab  S-Tab", "Next / previous record"),
    ("mouse", "Click, drag and scroll anywhere"),
    ("", "Views"),
    ("1 … 5", "Reader Stats Protein ORFs Enzymes"),
    ("r", "Toggle reverse strand"),
    ("f", "Cycle reading frame"),
    ("c  a", "Complement / translation tracks"),
    ("t", "Cycle color theme"),
    ("", "Search"),
    ("/", "Search a motif (any IUPAC code)"),
    ("n  N", "Next / previous match"),
    ("b", "Toggle minus-strand matching"),
];

const RIGHT: &[(&str, &str)] = &[
    ("", "Selection"),
    ("v", "Start / drop a selection"),
    ("drag", "Select with the mouse"),
    ("y", "Copy selection (or ORF) as FASTA"),
    ("", "ORF finder"),
    ("↑ ↓  ⏎", "Select, open in reader"),
    ("m  M", "Lower / raise minimum length"),
    ("", "Restriction map"),
    ("↑ ↓  ⏎", "Select, find sites in reader"),
    ("x", "Add enzyme to combined digest"),
    ("u", "Show only enzymes that cut"),
    ("", "Helix"),
    ("Space", "Pause / resume spin"),
    ("+ -  > <", "Speed / twist"),
    ("", "Other"),
    (":  Ctrl-P", "Command palette"),
    ("?  Esc  q", "Help / back out / quit"),
];

pub fn render(frame: &mut Frame, app: &App) {
    let p = app.theme.palette();
    let area = centered(104, LEFT.len().max(RIGHT.len()) as u16 + 4, frame.area());

    let column = |items: &[(&str, &str)]| -> Vec<Line<'static>> {
        items
            .iter()
            .map(|(keys, desc)| {
                if keys.is_empty() {
                    Line::from(vec![
                        Span::styled("▍", Style::default().fg(p.accent)),
                        Span::styled(
                            desc.to_string(),
                            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            format!("  {keys:<11}"),
                            Style::default().fg(p.accent).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(desc.to_string(), Style::default().fg(p.muted)),
                    ])
                }
            })
            .collect()
    };

    let title = Line::from(vec![
        Span::styled(" ? ", Style::default().fg(p.accent)),
        Span::styled("Key bindings ", Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
    ]);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(title)
        .title_bottom(Line::from(Span::styled(" any key or click to close ", Style::default().fg(p.muted))).right_aligned())
        .style(Style::default().bg(p.surface));
    let inner = block.inner(area);

    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    focus(frame, area, app);

    let [l, r] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .horizontal_margin(1)
        .areas(inner);
    frame.render_widget(Paragraph::new(column(LEFT)), l);
    frame.render_widget(Paragraph::new(column(RIGHT)), r);
}
