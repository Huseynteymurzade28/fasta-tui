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

    // Styled segments ("chips") read more clearly than one flat string.
    let chip = |label: String, fg: Color, bg: Color| {
        Span::styled(
            format!(" {label} "),
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
        )
    };
    let field = |label: String| Span::styled(label, Style::default().fg(palette.text));

    let base = app
        .cursor_base()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "·".to_string());

    let mut spans = vec![
        chip(
            if app.reverse { "REV".into() } else { "FWD".into() },
            Color::Black,
            palette.accent,
        ),
        field(format!(
            "  base {} @ {}/{}",
            base,
            app.cursor + 1,
            app.record_len()
        )),
        field("   ".into()),
        chip(format!("F+{}", app.frame), Color::Black, palette.base_g),
        field("  ".into()),
        chip(app.theme.name().into(), Color::Black, palette.base_c),
        field("  ".into()),
        if app.helix.paused {
            chip("PAUSED".into(), Color::White, palette.stop_bg)
        } else {
            chip("SPIN".into(), Color::Black, palette.base_a)
        },
    ];

    if !app.search.matches.is_empty() {
        spans.push(field("  ".into()));
        spans.push(chip(
            format!(
                "/{}  {}/{}",
                app.search.query,
                app.search.active + 1,
                app.search.matches.len()
            ),
            Color::Black,
            palette.match_bg,
        ));
    }

    frame.render_widget(
        Block::default().style(Style::default().fg(palette.text)),
        area,
    );
    frame.render_widget(Line::from(spans), area);

    // Right-aligned key reminder.
    let hint = Line::from(Span::styled(
        "←/→ base  ↑/↓ row  ?:help  q:quit ",
        Style::default().fg(palette.text).add_modifier(Modifier::DIM),
    ))
    .right_aligned();
    frame.render_widget(hint, area);
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
