//! Keyboard handling: maps key events onto [`App`] mutations.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::{clipboard, App, View};

impl App {
    /// Handle a single key event. Returns immediately for key *release*
    /// events so a press is not counted twice on terminals that report both.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        // While typing a motif, the keyboard is captured by the search box.
        if self.search.input_mode {
            self.handle_search_key(key.code);
            return;
        }

        // The list views bind the arrow keys to their own row selection, so
        // give them first refusal on the key.
        let consumed = match self.view {
            View::Orf => self.handle_orf_key(key.code),
            View::Enzymes => self.handle_enzyme_key(key.code),
            _ => false,
        };
        if consumed {
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => self.escape(),

            // Navigation: rows (↑/↓), single base (←/→), pages and ends.
            KeyCode::Down | KeyCode::Char('j') => self.row_down(),
            KeyCode::Up | KeyCode::Char('k') => self.row_up(),
            KeyCode::Left | KeyCode::Char('h') => self.step_left(),
            KeyCode::Right | KeyCode::Char('l') => self.step_right(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::PageUp => self.page_up(),
            KeyCode::Home | KeyCode::Char('g') => self.goto_start(),
            KeyCode::End | KeyCode::Char('G') => self.goto_end(),

            // View switching.
            KeyCode::Char('1') => self.set_view(View::Reader),
            KeyCode::Char('2') => self.set_view(View::Stats),
            KeyCode::Char('3') => self.set_view(View::Protein),
            KeyCode::Char('4') => self.set_view(View::Orf),
            KeyCode::Char('5') => self.set_view(View::Enzymes),

            // Record navigation.
            KeyCode::Tab | KeyCode::Char(']') => self.next_record(),
            KeyCode::BackTab | KeyCode::Char('[') => self.prev_record(),

            // Search.
            KeyCode::Char('/') => self.begin_search(),
            KeyCode::Char('n') => self.next_match(),
            KeyCode::Char('N') => self.prev_match(),
            KeyCode::Char('b') => self.toggle_both_strands(),

            // Selection.
            KeyCode::Char('v') => self.toggle_selection(),
            KeyCode::Char('y') => self.copy_selection(),

            // Helix controls.
            KeyCode::Char(' ') => self.toggle_pause(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.faster(),
            KeyCode::Char('-') | KeyCode::Char('_') => self.slower(),
            KeyCode::Char('>') | KeyCode::Char('.') => self.more_twist(),
            KeyCode::Char('<') | KeyCode::Char(',') => self.less_twist(),

            // Strand / frame / theme toggles.
            KeyCode::Char('r') => self.toggle_reverse(),
            KeyCode::Char('f') => self.cycle_frame(),
            KeyCode::Char('t') => self.cycle_theme(),

            // Help overlay.
            KeyCode::Char('?') => self.toggle_help(),

            _ => {}
        }
    }

    /// Esc peels off one layer at a time and only quits when nothing is open.
    fn escape(&mut self) {
        if self.show_help {
            self.show_help = false;
        } else if self.selection.active {
            self.clear_selection();
        } else if !self.search.matches.is_empty() {
            self.cancel_search();
        } else {
            self.should_quit = true;
        }
    }

    /// ORF table keys. Returns true when the key was consumed.
    fn handle_orf_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.orf_next(),
            KeyCode::Up | KeyCode::Char('k') => self.orf_prev(),
            KeyCode::Enter => self.orf_goto(),
            KeyCode::Char('m') => self.orf_min_step(-10),
            KeyCode::Char('M') => self.orf_min_step(10),
            KeyCode::Char('y') => self.copy_orf(),
            _ => return false,
        }
        true
    }

    /// Restriction table keys. Returns true when the key was consumed.
    fn handle_enzyme_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Down | KeyCode::Char('j') => self.enzyme_next(),
            KeyCode::Up | KeyCode::Char('k') => self.enzyme_prev(),
            KeyCode::Enter => self.enzyme_search(),
            KeyCode::Char('u') => self.toggle_hide_noncutters(),
            _ => return false,
        }
        true
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.cancel_search(),
            KeyCode::Enter => self.confirm_search(),
            KeyCode::Backspace => self.pop_search_char(),
            KeyCode::Char(c) => self.push_search_char(c),
            _ => {}
        }
    }

    /// Copy the selected bases to the clipboard as a FASTA record.
    fn copy_selection(&mut self) {
        let Some(bases) = self.selected_bases() else {
            self.flash("nothing selected — press v first");
            return;
        };
        let (lo, hi) = self.selection.range(self.cursor).expect("selection active");
        let strand = if self.reverse { " (reverse)" } else { "" };
        let header = format!(
            ">{}:{}-{}{}",
            self.record().id,
            lo + 1,
            hi.min(self.record_len()),
            strand
        );
        let n = bases.len();
        self.copy_fasta(&header, &bases, format!("copied {n} bases"));
    }

    /// Copy the selected ORF's protein to the clipboard as a FASTA record.
    fn copy_orf(&mut self) {
        let Some(orf) = self.orf.selected_orf().copied() else {
            self.flash("no ORF selected");
            return;
        };
        let protein = orf.protein(&self.record().bases);
        let header = format!(
            ">{}_orf_{}_{}..{} frame={} len={}aa",
            self.record().id,
            orf.frame_label().replace(['+', '-'], ""),
            orf.start + 1,
            orf.end,
            orf.frame_label(),
            orf.aa_len
        );
        let msg = format!("copied ORF protein ({} aa)", orf.aa_len);
        self.copy_fasta(&header, protein.as_bytes(), msg);
    }

    /// Shared clipboard path: wrap at 60 columns, then hand it to the terminal.
    fn copy_fasta(&mut self, header: &str, body: &[u8], ok_msg: String) {
        let mut text = String::with_capacity(body.len() + header.len() + 16);
        text.push_str(header);
        text.push('\n');
        for chunk in body.chunks(60) {
            text.push_str(&String::from_utf8_lossy(chunk));
            text.push('\n');
        }
        match clipboard::copy(&text) {
            Ok(()) => self.flash(ok_msg),
            Err(e) => self.flash(format!("clipboard failed: {e}")),
        }
    }
}
