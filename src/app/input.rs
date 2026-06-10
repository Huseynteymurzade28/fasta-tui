//! Keyboard handling: maps key events onto [`App`] mutations.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::{App, View};

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

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc => {
                if self.show_help {
                    self.show_help = false;
                } else {
                    self.should_quit = true;
                }
            }

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

            // Record navigation.
            KeyCode::Tab | KeyCode::Char(']') => self.next_record(),
            KeyCode::BackTab | KeyCode::Char('[') => self.prev_record(),

            // Search.
            KeyCode::Char('/') => self.begin_search(),
            KeyCode::Char('n') => self.next_match(),
            KeyCode::Char('N') => self.prev_match(),

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

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc => self.cancel_search(),
            KeyCode::Enter => self.confirm_search(),
            KeyCode::Backspace => self.pop_search_char(),
            KeyCode::Char(c) => self.push_search_char(c),
            _ => {}
        }
    }
}
