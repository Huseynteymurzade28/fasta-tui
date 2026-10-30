//! The command palette popup.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, Paragraph},
    Frame,
};

use super::{fx, glow, scroll_window};
use crate::app::{App, ListHit};

/// Rows of results shown at once.
const MAX_ROWS: usize = 12;

pub fn render(frame: &mut Frame, app: &App) {
    let p = app.theme.palette();
    let entries = app.palette_entries();
    let screen = frame.area();
    let width = 64.min(screen.width.saturating_sub(4));
    let rows = entries.len().clamp(1, MAX_ROWS) as u16;
    let height = rows + 4;
    // Sits in the upper third, where pickers usually live.
    let area = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: screen.y + (screen.height / 5).min(screen.height.saturating_sub(height)),
        width,
        height: height.min(screen.height),
    };

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .title(fx::gradient_text(" ❯ Command palette ", &p.grad, app.tick as f64 * 0.01, Modifier::BOLD))
        .title_bottom(
            Line::from(Span::styled(" ↑↓ choose · ⏎ run · Esc close · try a position or a motif ", Style::default().fg(p.muted)))
                .right_aligned(),
        )
        .style(Style::default().bg(p.surface));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    glow(frame, area, app);

    // Input line with a blinking caret.
    let caret = if app.tick / 12 % 2 == 0 { "▏" } else { " " };
    let input = Line::from(vec![
        Span::styled(" ❯ ", Style::default().fg(p.accent).add_modifier(Modifier::BOLD)),
        Span::styled(app.palette.query.clone(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        Span::styled(caret, Style::default().fg(p.accent)),
        if app.palette.query.is_empty() {
            Span::styled("type to filter…", Style::default().fg(p.muted))
        } else {
            Span::raw("")
        },
    ]);
    frame.render_widget(input, Rect { height: 1, ..inner });
    let sep = "─".repeat(inner.width as usize);
    frame.render_widget(
        Span::styled(sep, Style::default().fg(p.border)),
        Rect { y: inner.y + 1, height: 1, ..inner },
    );

    let list = Rect { y: inner.y + 2, height: inner.height.saturating_sub(2), ..inner };
    if entries.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled("  no matching command", Style::default().fg(p.muted))),
            list,
        );
        return;
    }
    let (first, last) = scroll_window(app.palette.selected, entries.len(), list.height as usize);
    let mut lines = Vec::new();
    for (i, e) in entries.iter().enumerate().take(last).skip(first) {
        let selected = i == app.palette.selected;
        let bg = if selected { p.surface_hi } else { p.surface };
        let mut spans = vec![Span::styled(
            if selected { " ▶ " } else { "   " },
            Style::default().fg(p.accent).bg(bg),
        )];
        // Matched characters light up in the accent color.
        for (ci, ch) in e.name.chars().enumerate() {
            let hit = e.hits.contains(&ci);
            let mut style = Style::default().bg(bg).fg(if hit { p.accent } else { p.text });
            if hit || selected {
                style = style.add_modifier(Modifier::BOLD);
            }
            spans.push(Span::styled(ch.to_string(), style));
        }
        let used = 3 + e.name.chars().count() + e.key.chars().count() + 2;
        let pad = (list.width as usize).saturating_sub(used);
        spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
        spans.push(Span::styled(
            format!("{} ", e.key),
            Style::default().fg(p.muted).bg(bg),
        ));
        spans.push(Span::styled(" ", Style::default().bg(bg)));
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), list);
    app.hits.borrow_mut().palette_list = Some(ListHit { body: list, offset: first });
}
