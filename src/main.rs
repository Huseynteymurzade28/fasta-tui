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

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event};

use app::App;
use cli::Cli;

fn main() -> io::Result<()> {
    let cli = Cli::parse_args();
    let records = fasta::parse_file(&cli.file)?;
    let mut app = App::new(records);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
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
