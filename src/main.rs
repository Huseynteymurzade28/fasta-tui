//! fasta-tui — a terminal viewer for FASTA DNA sequences.
//!
//! Modules:
//! - [`cli`]   command-line argument parsing
//! - [`fasta`] domain layer (parsing, records, stats, translation)
//! - [`theme`] color palettes
//! - [`app`]   application state and input handling
//! - [`ui`]    rendering (reader, helix, stats, protein, help)

mod app;
mod cli;
mod fasta;
mod theme;
mod ui;

use std::io::{self, Write};
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event};

use app::App;
use cli::Cli;
use fasta::Record;

fn main() -> io::Result<()> {
    let cli = Cli::parse_args();
    let records = fasta::parse_file(&cli.file)?;

    // Headless mode: translate to protein and exit without touching the screen.
    if let Some(out) = &cli.export_protein {
        return export_protein(&records, cli.frame as usize, cli.reverse, out);
    }

    let mut app = App::new(records);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

/// Translate each record (forward or reverse strand, in the chosen frame) and
/// write the results as a protein FASTA file — or to stdout when `out` is `-`.
fn export_protein(records: &[Record], frame: usize, reverse: bool, out: &Path) -> io::Result<()> {
    let mut buf = String::new();
    for rec in records {
        let bases = rec.oriented(reverse);
        let protein: String = fasta::translate::translate(&bases, frame).into_iter().collect();

        buf.push('>');
        buf.push_str(&rec.id);
        if !rec.description.is_empty() {
            buf.push(' ');
            buf.push_str(&rec.description);
        }
        let strand = if reverse { ", reverse strand" } else { "" };
        buf.push_str(&format!(" [translated frame {frame}{strand}]\n"));
        // Wrap residues at 60 columns like a conventional FASTA file.
        for chunk in protein.as_bytes().chunks(60) {
            buf.push_str(std::str::from_utf8(chunk).expect("ASCII residues"));
            buf.push('\n');
        }
    }

    if out == Path::new("-") {
        io::stdout().write_all(buf.as_bytes())
    } else {
        std::fs::write(out, buf)?;
        eprintln!(
            "Wrote {} protein record(s) to {}",
            records.len(),
            out.display()
        );
        Ok(())
    }
}

/// The main event loop. Polls with a timeout so the helix keeps spinning even
/// when the user is idle.
fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(Duration::from_millis(80))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key);
            }
        }
        app.on_tick();
    }
    Ok(())
}
