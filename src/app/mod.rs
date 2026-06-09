//! Central application state and the logic that mutates it.

pub mod input;

use crate::fasta::Record;
use crate::theme::Theme;

/// Which main view is currently shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Sequence reader plus the 3D helix.
    Reader,
    /// Composition bar chart plus GC sliding-window chart.
    Stats,
    /// Amino-acid translation of the current reading frame.
    Protein,
}

impl View {
    pub const ALL: [View; 3] = [View::Reader, View::Stats, View::Protein];

    pub fn title(self) -> &'static str {
        match self {
            View::Reader => "Reader",
            View::Stats => "Stats",
            View::Protein => "Protein",
        }
    }

    pub fn index(self) -> usize {
        match self {
            View::Reader => 0,
            View::Stats => 1,
            View::Protein => 2,
        }
    }
}

/// Animation parameters for the double helix.
#[derive(Debug, Clone, Copy)]
pub struct HelixState {
    /// Accumulated rotation angle.
    pub phase: f64,
    /// Idle spin per tick.
    pub speed: f64,
    /// Radians of twist contributed by each base pair.
    pub twist: f64,
    /// When true, the idle spin is frozen (scrolling still rotates it).
    pub paused: bool,
}

impl Default for HelixState {
    fn default() -> Self {
        HelixState {
            phase: 0.0,
            speed: 0.03,
            twist: 0.55,
            paused: false,
        }
    }
}

/// Incremental motif search over the current record.
#[derive(Debug, Default, Clone)]
pub struct SearchState {
    /// The motif being searched for (uppercased).
    pub query: String,
    /// Start indices of every match in the current record.
    pub matches: Vec<usize>,
    /// Index into `matches` of the currently focused hit.
    pub active: usize,
    /// True while the user is typing into the search box.
    pub input_mode: bool,
}

impl SearchState {
    /// Length of the motif, used to compute highlight spans.
    pub fn query_len(&self) -> usize {
        self.query.len()
    }
}

/// The complete UI state.
pub struct App {
    pub records: Vec<Record>,
    pub current: usize,
    pub view: View,
    /// Scroll cursor measured in bases within the current record.
    pub cursor: usize,
    pub theme: Theme,
    pub helix: HelixState,
    pub search: SearchState,
    /// Reading frame (0/1/2) for translation.
    pub frame: usize,
    /// Whether to view the reverse-complement strand.
    pub reverse: bool,
    pub show_help: bool,
    pub tick: u64,
    pub should_quit: bool,
}

impl App {
    pub fn new(records: Vec<Record>) -> Self {
        App {
            records,
            current: 0,
            view: View::Reader,
            cursor: 0,
            theme: Theme::Neon,
            helix: HelixState::default(),
            search: SearchState::default(),
            frame: 0,
            reverse: false,
            show_help: false,
            tick: 0,
            should_quit: false,
        }
    }

    /// The record currently in focus.
    pub fn record(&self) -> &Record {
        &self.records[self.current]
    }

    pub fn record_len(&self) -> usize {
        self.record().len()
    }

    // --- Animation -------------------------------------------------------

    /// Advance idle animation; called once per event-loop tick.
    pub fn on_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        if !self.helix.paused {
            self.helix.phase += self.helix.speed;
        }
    }

    // --- Scrolling -------------------------------------------------------

    pub fn scroll_down(&mut self) {
        let max = self.record_len().saturating_sub(1);
        self.cursor = (self.cursor + SCROLL_STEP).min(max);
        self.helix.phase += SCROLL_PHASE * SCROLL_STEP as f64;
    }

    pub fn scroll_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(SCROLL_STEP);
        self.helix.phase -= SCROLL_PHASE * SCROLL_STEP as f64;
    }

    // --- Records ---------------------------------------------------------

    pub fn next_record(&mut self) {
        if self.records.len() < 2 {
            return;
        }
        self.current = (self.current + 1) % self.records.len();
        self.reset_for_record();
    }

    pub fn prev_record(&mut self) {
        if self.records.len() < 2 {
            return;
        }
        self.current = (self.current + self.records.len() - 1) % self.records.len();
        self.reset_for_record();
    }

    fn reset_for_record(&mut self) {
        self.cursor = 0;
        self.recompute_matches();
    }

    // --- Views & toggles -------------------------------------------------

    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.next();
    }

    pub fn toggle_reverse(&mut self) {
        self.reverse = !self.reverse;
    }

    pub fn cycle_frame(&mut self) {
        self.frame = (self.frame + 1) % 3;
    }

    pub fn toggle_pause(&mut self) {
        self.helix.paused = !self.helix.paused;
    }

    pub fn faster(&mut self) {
        self.helix.speed = (self.helix.speed + 0.01).min(0.4);
    }

    pub fn slower(&mut self) {
        self.helix.speed = (self.helix.speed - 0.01).max(0.0);
    }

    pub fn more_twist(&mut self) {
        self.helix.twist = (self.helix.twist + 0.05).min(2.0);
    }

    pub fn less_twist(&mut self) {
        self.helix.twist = (self.helix.twist - 0.05).max(0.1);
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    // --- Search ----------------------------------------------------------

    pub fn begin_search(&mut self) {
        self.search.input_mode = true;
        self.search.query.clear();
    }

    pub fn push_search_char(&mut self, c: char) {
        if matches!(c.to_ascii_uppercase(), 'A' | 'T' | 'G' | 'C') {
            self.search.query.push(c.to_ascii_uppercase());
        }
    }

    pub fn pop_search_char(&mut self) {
        self.search.query.pop();
    }

    pub fn cancel_search(&mut self) {
        self.search.input_mode = false;
        self.search.query.clear();
        self.search.matches.clear();
    }

    /// Confirm the typed motif and jump to the first match.
    pub fn confirm_search(&mut self) {
        self.search.input_mode = false;
        self.recompute_matches();
        if let Some(&pos) = self.search.matches.first() {
            self.search.active = 0;
            self.cursor = pos;
        }
    }

    pub fn next_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.search.active = (self.search.active + 1) % self.search.matches.len();
        self.cursor = self.search.matches[self.search.active];
    }

    pub fn prev_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        let n = self.search.matches.len();
        self.search.active = (self.search.active + n - 1) % n;
        self.cursor = self.search.matches[self.search.active];
    }

    /// Recompute every motif occurrence in the current record.
    fn recompute_matches(&mut self) {
        self.search.matches.clear();
        self.search.active = 0;
        let q = self.search.query.as_bytes();
        if q.is_empty() {
            return;
        }
        let hay = &self.records[self.current].bases;
        if q.len() > hay.len() {
            return;
        }
        for start in 0..=hay.len() - q.len() {
            if &hay[start..start + q.len()] == q {
                self.search.matches.push(start);
            }
        }
    }
}

/// How many bases we advance per scroll key press (one codon).
pub const SCROLL_STEP: usize = 3;
/// Phase rotation contributed per base when scrolling.
pub const SCROLL_PHASE: f64 = 0.08;
