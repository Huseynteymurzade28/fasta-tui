//! The Stats view: base-composition bar chart + GC sliding-window line chart.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::Span,
    widgets::{Axis, BarChart, Block, BorderType, Chart, Dataset, GraphType},
    Frame,
};

use crate::app::App;
use crate::fasta::stats;

/// Window size (in bases) for the GC sliding-window plot.
const GC_WINDOW: usize = 50;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_composition(frame, rows[0], app);
    render_gc_curve(frame, rows[1], app);
}

fn render_composition(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let counts = stats::base_counts(&app.record().bases);
    let data = counts.as_pairs();

    let bars = BarChart::default()
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(format!(" Base Composition (total {} bp) ", counts.total())),
        )
        .data(&data)
        .bar_width(7)
        .bar_gap(3)
        .bar_style(Style::default().fg(palette.accent))
        .value_style(
            Style::default()
                .fg(Color::Black)
                .bg(palette.accent)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(bars, area);
}

fn render_gc_curve(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let len = app.record_len();

    // One sample roughly per horizontal cell keeps the curve crisp.
    let samples = (area.width as usize).saturating_sub(4).max(2);
    let points = stats::gc_sliding_window(&app.record().bases, GC_WINDOW, samples);

    let max_x = len.max(1) as f64;
    let datasets = vec![Dataset::default()
        .name("GC%")
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(palette.accent))
        .data(&points)];

    let chart = Chart::new(datasets)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(format!(" GC Content — sliding window ({GC_WINDOW} bp) ")),
        )
        .x_axis(
            Axis::default()
                .title("position")
                .style(Style::default().fg(palette.text))
                .bounds([0.0, max_x])
                .labels(vec![
                    Span::raw("0"),
                    Span::raw(format!("{}", len / 2)),
                    Span::raw(format!("{len}")),
                ]),
        )
        .y_axis(
            Axis::default()
                .title("GC %")
                .style(Style::default().fg(palette.text))
                .bounds([0.0, 100.0])
                .labels(vec![Span::raw("0"), Span::raw("50"), Span::raw("100")]),
        );
    frame.render_widget(chart, area);
}
