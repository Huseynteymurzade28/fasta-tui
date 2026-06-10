//! The Protein view: amino-acid translation of the selected reading frame.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};

use crate::app::App;
use crate::fasta::translate;

/// Amino-acid residues shown per row.
const RESIDUES_PER_ROW: usize = 60;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let palette = app.theme.palette();
    let record = app.record();

    // Translate the chosen strand orientation and reading frame.
    let strand = record.oriented(app.reverse);
    let residues = translate::translate(&strand, app.frame);

    let title = format!(
        " Protein — {} strand, frame +{}  (M=start, *=stop)  [r:strand  f:frame] ",
        if app.reverse { "reverse" } else { "forward" },
        app.frame
    );

    let lines = if residues.is_empty() {
        vec![Line::from("<sequence too short to translate>")]
    } else {
        residues
            .chunks(RESIDUES_PER_ROW)
            .map(|chunk| {
                let spans = chunk
                    .iter()
                    .map(|&aa| Span::styled(aa.to_string(), residue_style(aa, &palette)))
                    .collect::<Vec<_>>();
                Line::from(spans)
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines)
        .block(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title(title),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn residue_style(aa: char, palette: &crate::theme::Palette) -> Style {
    match aa {
        'M' => Style::default().bg(palette.start_bg).fg(Color::Black).add_modifier(Modifier::BOLD),
        '*' => Style::default().bg(palette.stop_bg).fg(Color::White).add_modifier(Modifier::BOLD),
        'X' => Style::default().fg(Color::DarkGray),
        _ => Style::default().fg(palette.text),
    }
}
