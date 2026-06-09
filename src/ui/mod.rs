//! Rendering layer: top-level layout, tab bar, status line and popups.

mod helix;
mod help;
mod protein;
mod reader;
mod stats;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Tabs},
    Frame,
};

use crate::app::{App, View};

/// Entry point used by the event loop each frame.
pub fn draw(frame: &mut Frame, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status line
        ])
        .split(frame.area());

    render_tabs(frame, rows[0], app);

    match app.view {
        View::Reader => reader::render(frame, rows[1], app),
        View::Stats => stats::render(frame, rows[1], app),
        View::Protein => protein::render(frame, rows[1], app),
    }

    render_status(frame, rows[2], app);

    if app.show_help {
        help::render(frame, app);
    }
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(30)])
        .split(area);

    let titles: Vec<Line> = View::ALL
        .iter()
        .enumerate()
        .map(|(i, v)| Line::from(format!(" {}:{} ", i + 1, v.title())))
        .collect();

    let tabs = Tabs::new(titles)
        .select(app.view.index())
        .style(Style::default().fg(palette.text))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD),
        )
        .divider("");
    frame.render_widget(tabs, cols[0]);

    // Record indicator on the right.
    let record = app.record();
    let label = format!(
        " {} [{}/{}] ",
        record.id,
        app.current + 1,
        app.records.len()
    );
    let indicator = Line::from(Span::styled(
        label,
        Style::default().fg(palette.accent).add_modifier(Modifier::BOLD),
    ))
    .right_aligned();
    frame.render_widget(indicator, cols[1]);
}

fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();

    // While searching, the status line becomes the search prompt.
    if app.search.input_mode {
        let prompt = format!("  /{}", app.search.query);
        let line = Line::from(vec![
            Span::styled("SEARCH", Style::default().fg(Color::Black).bg(palette.accent)),
            Span::styled(prompt, Style::default().fg(palette.accent)),
            Span::styled(
                "   (Enter: confirm  Esc: cancel  A/T/G/C only)",
                Style::default().fg(palette.text),
            ),
        ]);
        frame.render_widget(line, area);
        return;
    }

    let matches = if app.search.matches.is_empty() {
        String::new()
    } else {
        format!(
            "  match {}/{}",
            app.search.active + 1,
            app.search.matches.len()
        )
    };
    let helix = if app.helix.paused { "paused" } else { "spin" };
    let hint = format!(
        " {}  pos {}/{}  frame +{}  {}  helix:{}{}   ?:help  q:quit",
        if app.reverse { "rev-comp" } else { "forward" },
        app.cursor,
        app.record_len(),
        app.frame,
        app.theme.name(),
        helix,
        matches,
    );
    let block = Block::default().style(Style::default().fg(palette.text));
    let line = Line::from(hint);
    frame.render_widget(block, area);
    frame.render_widget(line, area);
}

/// A centered rectangle `pct_x` × `pct_y` of `area`, used for popups.
pub fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(vertical[1])[1]
}
