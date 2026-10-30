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
use std::time::Instant;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;

use app::App;
use cli::Cli;
use fasta::Record;

fn main() -> io::Result<()> {
    let cli = Cli::parse_args();
    let records = fasta::parse_file(&cli.file)?;

    // Headless modes: produce a file or a report and exit without touching the
    // screen, so the tool composes with scripts and pipelines.
    if let Some(out) = &cli.export_protein {
        return export_protein(&records, cli.frame as usize, cli.reverse, out);
    }
    if let Some(out) = &cli.export_orfs {
        return export_orfs(&records, cli.min_orf, out);
    }
    if cli.restriction_map {
        return print_restriction_map(&records);
    }

    let mut app = App::new(records);
    let mut terminal = ratatui::init();
    execute!(io::stdout(), EnableMouseCapture)?;
    let result = run(&mut terminal, &mut app);
    let _ = execute!(io::stdout(), DisableMouseCapture);
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

/// Find every ORF of at least `min_aa` residues in each record and write them
/// as a protein FASTA file — or to stdout when `out` is `-`.
fn export_orfs(records: &[Record], min_aa: usize, out: &Path) -> io::Result<()> {
    let mut buf = String::new();
    let mut total = 0usize;
    for rec in records {
        for (i, orf) in fasta::orf::find_orfs(&rec.bases, min_aa).iter().enumerate() {
            total += 1;
            buf.push_str(&format!(
                ">{}_orf{} strand={} frame={} start={} end={} len={}aa{}\n",
                rec.id,
                i + 1,
                orf.strand.symbol(),
                orf.frame + 1,
                orf.start + 1,
                orf.end,
                orf.aa_len,
                if orf.has_stop { "" } else { " partial" },
            ));
            let protein = orf.protein(&rec.bases);
            for chunk in protein.as_bytes().chunks(60) {
                buf.push_str(&String::from_utf8_lossy(chunk));
                buf.push('\n');
            }
        }
    }

    if out == Path::new("-") {
        io::stdout().write_all(buf.as_bytes())
    } else {
        std::fs::write(out, buf)?;
        eprintln!("Wrote {total} ORF(s) ≥{min_aa} aa to {}", out.display());
        Ok(())
    }
}

/// Print a TSV restriction map: one row per record and enzyme that cuts.
fn print_restriction_map(records: &[Record]) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "record\tenzyme\tsite\tsites\tpositions\tfragments")?;
    for rec in records {
        for d in fasta::enzymes::digest_all(&rec.bases) {
            if d.count() == 0 {
                continue;
            }
            let positions = d
                .cuts()
                .iter()
                .map(|c| (c + 1).to_string())
                .collect::<Vec<_>>()
                .join(",");
            let frags = d
                .fragments(rec.len())
                .iter()
                .map(|f| f.to_string())
                .collect::<Vec<_>>()
                .join(",");
            writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}\t{}",
                rec.id,
                d.enzyme.name,
                d.enzyme.site_with_cut(),
                d.count(),
                positions,
                frags
            )?;
        }
    }
    Ok(())
}

/// The main event loop. Animation advances on a fixed clock independent of
/// input, so holding a key does not speed the helix up; events that arrive
/// between ticks are handled and redrawn immediately.
fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut next_tick = Instant::now() + app::TICK;
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        let timeout = next_tick.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => app.handle_key(key),
                Event::Mouse(mouse) => app.handle_mouse(mouse),
                _ => {}
            }
        }
        if Instant::now() >= next_tick {
            app.on_tick();
            next_tick += app::TICK;
            // After a stall (suspended terminal), resync instead of racing.
            if next_tick < Instant::now() {
                next_tick = Instant::now() + app::TICK;
            }
        }
    }
    Ok(())
}
