//! Central application state and the logic that mutates it.

pub mod clipboard;
pub mod commands;
pub mod input;
pub mod mouse;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use ratatui::layout::Rect;

use crate::fasta::enzymes::{self, Digest};
use crate::fasta::orf::{self, Orf};
use crate::fasta::{iupac, oligo, stats, Record};
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
    /// Six-frame open reading frame table.
    Orf,
    /// Restriction enzyme map of the current record.
    Enzymes,
}

impl View {
    pub const ALL: [View; 5] = [
        View::Reader,
        View::Stats,
        View::Protein,
        View::Orf,
        View::Enzymes,
    ];

    pub fn title(self) -> &'static str {
        match self {
            View::Reader => "Reader",
            View::Stats => "Stats",
            View::Protein => "Protein",
            View::Orf => "ORFs",
            View::Enzymes => "Enzymes",
        }
    }

    /// A one-glyph icon shown beside the title in the tab bar.
    pub fn icon(self) -> &'static str {
        match self {
            View::Reader => "≋",
            View::Stats => "▤",
            View::Protein => "⬡",
            View::Orf => "⇶",
            View::Enzymes => "✂",
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

/// One motif occurrence, in forward-strand coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MotifMatch {
    pub start: usize,
    pub len: usize,
    /// True when the hit is the motif's reverse complement.
    pub revcomp: bool,
}

impl MotifMatch {
    pub fn end(&self) -> usize {
        self.start + self.len
    }
}

/// Incremental motif search over the current record. The query may use any
/// IUPAC ambiguity code, and optionally matches the minus strand too.
#[derive(Debug, Default, Clone)]
pub struct SearchState {
    /// The motif being searched for (uppercased).
    pub query: String,
    /// Every match in the current record, ordered by position.
    pub matches: Vec<MotifMatch>,
    /// Index into `matches` of the currently focused hit.
    pub active: usize,
    /// True while the user is typing into the search box.
    pub input_mode: bool,
    /// Also report hits for the motif's reverse complement.
    pub both_strands: bool,
    /// Set when the query came from the enzyme table, for the status line.
    pub label: Option<String>,
}

impl SearchState {
    /// The focused hit, if there is one.
    pub fn active_match(&self) -> Option<MotifMatch> {
        self.matches.get(self.active).copied()
    }
}

/// The six-frame ORF table for the current record.
#[derive(Debug, Clone)]
pub struct OrfState {
    pub orfs: Vec<Orf>,
    /// Index into `orfs` of the highlighted row.
    pub selected: usize,
    /// Minimum ORF length in amino acids.
    pub min_aa: usize,
}

impl Default for OrfState {
    fn default() -> Self {
        OrfState {
            orfs: Vec::new(),
            selected: 0,
            min_aa: DEFAULT_MIN_ORF_AA,
        }
    }
}

impl OrfState {
    pub fn selected_orf(&self) -> Option<&Orf> {
        self.orfs.get(self.selected)
    }
}

/// The restriction map for the current record.
#[derive(Debug, Clone, Default)]
pub struct EnzymeState {
    pub digests: Vec<Digest>,
    /// Index into the *visible* rows (see [`EnzymeState::visible`]).
    pub selected: usize,
    /// Hide enzymes that do not cut this record.
    pub hide_zero: bool,
    /// Enzymes picked for a combined digest, by name.
    pub marked: Vec<&'static str>,
    /// Tick the shown digest last changed on; restarts the gel run.
    pub changed_at: u64,
}

impl EnzymeState {
    /// The rows actually listed, honouring the `hide_zero` filter.
    pub fn visible(&self) -> Vec<&Digest> {
        self.digests
            .iter()
            .filter(|d| !self.hide_zero || d.count() > 0)
            .collect()
    }

    pub fn selected_digest(&self) -> Option<&Digest> {
        self.visible().get(self.selected).copied()
    }

    /// Cut positions of every marked enzyme together, ascending.
    pub fn combined_cuts(&self) -> Vec<usize> {
        let mut cuts: Vec<usize> = self
            .digests
            .iter()
            .filter(|d| self.marked.contains(&d.enzyme.name))
            .flat_map(|d| d.cuts())
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        cuts
    }
}

/// A base range picked out in the reader, anchored where the user pressed `v`
/// and extended by every cursor move afterwards.
#[derive(Debug, Clone, Copy, Default)]
pub struct Selection {
    pub active: bool,
    pub anchor: usize,
}

impl Selection {
    /// The selected half-open range given the live cursor position.
    pub fn range(&self, cursor: usize) -> Option<(usize, usize)> {
        if !self.active {
            return None;
        }
        let (lo, hi) = if self.anchor <= cursor {
            (self.anchor, cursor)
        } else {
            (cursor, self.anchor)
        };
        Some((lo, hi + 1))
    }
}

/// Reader-pane geometry, measured during rendering and read back by the
/// navigation keys so row / page movement matches what is on screen.
#[derive(Debug, Clone, Copy)]
pub struct ReaderMetrics {
    /// Bases drawn on a single sequence row.
    pub bases_per_row: usize,
    /// Sequence rows visible in the viewport at once.
    pub visible_rows: usize,
    /// Residues drawn per row in the Protein view.
    pub residues_per_row: usize,
}

impl Default for ReaderMetrics {
    fn default() -> Self {
        // Sensible values for the first frame, before render measures the area.
        ReaderMetrics {
            bases_per_row: 60,
            visible_rows: 20,
            residues_per_row: 60,
        }
    }
}

/// Where the reader's sequence rows landed on screen, so a click can be
/// mapped back to a base.
#[derive(Debug, Clone, Copy)]
pub struct ReaderHit {
    /// The rows area (below the ruler), inside the border.
    pub rows: Rect,
    pub lines_per_row: usize,
    pub bases_per_row: usize,
    pub scroll_top: usize,
    /// Columns taken by the position gutter before the first base.
    pub gutter: u16,
}

/// A rendered list: its body rectangle and the index of its first row.
#[derive(Debug, Clone, Copy)]
pub struct ListHit {
    pub body: Rect,
    pub offset: usize,
}

/// Clickable regions recorded by the last frame. Rebuilt on every draw.
#[derive(Debug, Clone, Default)]
pub struct HitMap {
    pub tabs: Vec<(Rect, View)>,
    pub reader: Option<ReaderHit>,
    pub overview: Option<Rect>,
    pub orf_table: Option<ListHit>,
    pub six_frame: Option<Rect>,
    pub enzyme_table: Option<ListHit>,
    pub palette_list: Option<ListHit>,
}

/// How a toast should be colored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warn,
}

/// A transient notification stacked in the bottom-right corner.
#[derive(Debug, Clone)]
pub struct Toast {
    pub msg: String,
    pub kind: ToastKind,
    /// Tick the toast appeared on; it slides in, holds, then fades.
    pub born: u64,
}

/// Per-record lookups computed once instead of on every frame.
#[derive(Debug, Clone, Default)]
pub struct RecordCache {
    /// GC prefix sums for O(1) window queries (overview track, gauges).
    pub gc_prefix: Vec<u32>,
    pub gc: f64,
    pub counts: stats::BaseCounts,
    pub dinucleotides: [[f64; 4]; 4],
}

/// The translation of one strand and frame, with what the Protein view
/// derives from it.
#[derive(Debug, Clone)]
pub struct Translation {
    pub residues: Vec<char>,
    /// Raw hydropathy profile (one point per full window).
    pub hydropathy: Vec<(f64, f64)>,
    pub stops: usize,
    /// Hydrophobic plus aromatic residues.
    pub hydrophobic: usize,
}

/// A chart series of `(x, y)` points.
pub type Series = Vec<(f64, f64)>;

/// Derived data that depends on view parameters (strand, frame, chart
/// width). Filled lazily during rendering and keyed by those parameters, so a
/// multi-megabase record is not re-scanned on every frame.
#[derive(Debug, Default)]
pub struct Memo {
    pub codons: Option<((bool, usize), [usize; 64])>,
    pub curves: Option<(usize, Series, Series)>,
    pub translation: Option<((bool, usize), Rc<Translation>)>,
}

/// The fuzzy command palette.
#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
}

/// The complete UI state.
pub struct App {
    pub records: Vec<Record>,
    pub current: usize,
    pub view: View,
    /// Scroll cursor measured in bases within the current record.
    pub cursor: usize,
    /// Top row of the reader viewport (in sequence rows). Interior-mutable so
    /// rendering can re-clamp it when the geometry changes under the cursor.
    pub scroll_top: Cell<usize>,
    /// Reader geometry from the last render; updated through interior
    /// mutability because rendering only holds `&App`.
    pub metrics: Cell<ReaderMetrics>,
    pub theme: Theme,
    pub helix: HelixState,
    pub search: SearchState,
    pub orf: OrfState,
    pub enzymes: EnzymeState,
    pub selection: Selection,
    pub cache: RecordCache,
    pub memo: RefCell<Memo>,
    pub palette: PaletteState,
    /// Clickable regions from the last frame, for the mouse handler.
    pub hits: RefCell<HitMap>,
    pub toasts: Vec<Toast>,
    /// Tick the current view was entered on, driving its fade-in.
    pub view_since: u64,
    /// Show the complementary strand under each reader row.
    pub show_complement: bool,
    /// Show the amino-acid translation under each reader row.
    pub show_translation: bool,
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
        let mut app = App {
            records,
            current: 0,
            view: View::Reader,
            cursor: 0,
            scroll_top: Cell::new(0),
            metrics: Cell::new(ReaderMetrics::default()),
            theme: Theme::Slate,
            helix: HelixState::default(),
            search: SearchState::default(),
            orf: OrfState::default(),
            enzymes: EnzymeState::default(),
            selection: Selection::default(),
            cache: RecordCache::default(),
            memo: RefCell::new(Memo::default()),
            palette: PaletteState::default(),
            hits: RefCell::new(HitMap::default()),
            toasts: Vec::new(),
            view_since: 0,
            show_complement: true,
            show_translation: true,
            frame: 0,
            reverse: false,
            show_help: false,
            tick: 0,
            should_quit: false,
        };
        app.recompute_analyses();
        app
    }

    /// Recompute the per-record analyses (ORFs, restriction map). Called on
    /// startup and whenever the focused record or a parameter changes.
    fn recompute_analyses(&mut self) {
        let bases = &self.records[self.current].bases;
        self.orf.orfs = orf::find_orfs(bases, self.orf.min_aa);
        self.orf.selected = 0;
        self.enzymes.digests = enzymes::digest_all(bases);
        self.enzymes.selected = 0;
        self.cache = RecordCache {
            gc_prefix: stats::gc_prefix(bases),
            gc: stats::gc_content(bases),
            counts: stats::base_counts(bases),
            dinucleotides: stats::dinucleotide_oe(bases),
        };
        *self.memo.borrow_mut() = Memo::default();
    }

    /// Codon usage for the current strand and frame.
    pub fn codon_usage(&self) -> [usize; 64] {
        let key = (self.reverse, self.frame);
        let mut memo = self.memo.borrow_mut();
        if let Some((k, usage)) = &memo.codons {
            if *k == key {
                return *usage;
            }
        }
        let usage = stats::codon_usage(&self.record().oriented(self.reverse), self.frame);
        memo.codons = Some((key, usage));
        usage
    }

    /// GC% and GC-skew curves sampled `samples` times along the record.
    pub fn gc_curves(&self, window: usize, samples: usize) -> (Series, Series) {
        let mut memo = self.memo.borrow_mut();
        if let Some((k, gc, skew)) = &memo.curves {
            if *k == samples {
                return (gc.clone(), skew.clone());
            }
        }
        let bases = &self.record().bases;
        let gc = stats::gc_sliding_window(bases, window, samples);
        let skew = stats::gc_skew_window(bases, window, samples);
        memo.curves = Some((samples, gc.clone(), skew.clone()));
        (gc, skew)
    }

    /// The translation of the current strand and frame.
    pub fn translation(&self, hydropathy_window: usize) -> Rc<Translation> {
        use crate::fasta::aa::{self, AaClass};
        let key = (self.reverse, self.frame);
        let mut memo = self.memo.borrow_mut();
        if let Some((k, t)) = &memo.translation {
            if *k == key {
                return Rc::clone(t);
            }
        }
        let residues = crate::fasta::translate::translate(&self.record().oriented(self.reverse), self.frame);
        let t = Rc::new(Translation {
            hydropathy: aa::hydropathy_profile(&residues, hydropathy_window),
            stops: residues.iter().filter(|&&a| a == '*').count(),
            hydrophobic: residues
                .iter()
                .filter(|&&a| matches!(AaClass::of(a), AaClass::Hydrophobic | AaClass::Aromatic))
                .count(),
            residues,
        });
        memo.translation = Some((key, Rc::clone(&t)));
        t
    }

    /// Pop up a short-lived notification.
    pub fn flash(&mut self, msg: impl Into<String>) {
        self.toast(msg, ToastKind::Info);
    }

    /// A notification for something that went right (a copy, a jump).
    pub fn flash_ok(&mut self, msg: impl Into<String>) {
        self.toast(msg, ToastKind::Success);
    }

    /// A notification for something that did not work.
    pub fn flash_warn(&mut self, msg: impl Into<String>) {
        self.toast(msg, ToastKind::Warn);
    }

    fn toast(&mut self, msg: impl Into<String>, kind: ToastKind) {
        self.toasts.push(Toast {
            msg: msg.into(),
            kind,
            born: self.tick,
        });
        // A burst of key presses should not bury the screen in toasts.
        if self.toasts.len() > MAX_TOASTS {
            self.toasts.remove(0);
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
            self.helix.phase += self.helix.speed * HELIX_TICK_SCALE;
        }
        let tick = self.tick;
        self.toasts.retain(|t| tick < t.born + TOAST_TICKS);
    }

    /// Progress `0..=1` of the current view's entrance animation.
    pub fn view_progress(&self) -> f64 {
        (self.tick.saturating_sub(self.view_since)) as f64 / TRANSITION_TICKS as f64
    }

    // --- Navigation ------------------------------------------------------

    /// The base currently under the cursor (uppercase ASCII), if any.
    pub fn cursor_base(&self) -> Option<char> {
        self.record().bases.get(self.cursor).map(|&b| b as char)
    }

    /// Move the cursor by `delta` bases, clamped to the record, rotating the
    /// helix in proportion and keeping the viewport in sync.
    pub fn move_cursor(&mut self, delta: isize) {
        let max = self.record_len().saturating_sub(1) as isize;
        if max < 0 {
            return;
        }
        let target = (self.cursor as isize + delta).clamp(0, max);
        let moved = target - self.cursor as isize;
        if moved == 0 {
            return;
        }
        self.cursor = target as usize;
        self.helix.phase += SCROLL_PHASE * moved as f64;
        self.ensure_visible();
    }

    /// Horizontal step: a single base left / right.
    pub fn step_left(&mut self) {
        self.move_cursor(-1);
    }

    pub fn step_right(&mut self) {
        self.move_cursor(1);
    }

    /// Vertical step: one rendered row up / down.
    pub fn row_up(&mut self) {
        self.move_cursor(-(self.metrics.get().bases_per_row as isize));
    }

    pub fn row_down(&mut self) {
        self.move_cursor(self.metrics.get().bases_per_row as isize);
    }

    /// One viewport page up / down.
    pub fn page_up(&mut self) {
        let m = self.metrics.get();
        self.move_cursor(-((m.bases_per_row * m.visible_rows.max(1)) as isize));
    }

    pub fn page_down(&mut self) {
        let m = self.metrics.get();
        self.move_cursor((m.bases_per_row * m.visible_rows.max(1)) as isize);
    }

    pub fn goto_start(&mut self) {
        self.move_cursor(isize::MIN / 2);
    }

    pub fn goto_end(&mut self) {
        self.move_cursor(isize::MAX / 2);
    }

    /// Scroll the viewport so the cursor's row stays visible.
    fn ensure_visible(&mut self) {
        let m = self.metrics.get();
        if m.bases_per_row == 0 {
            return;
        }
        self.scroll_top.set(clamp_scroll(
            self.scroll_top.get(),
            self.cursor / m.bases_per_row,
            m.visible_rows,
        ));
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
        self.scroll_top.set(0);
        self.selection.active = false;
        self.recompute_matches();
        self.recompute_analyses();
    }

    // --- Views & toggles -------------------------------------------------

    pub fn set_view(&mut self, view: View) {
        if self.view != view {
            self.view = view;
            self.view_since = self.tick;
            self.enzymes.changed_at = self.tick;
        }
    }

    pub fn toggle_complement(&mut self) {
        self.show_complement = !self.show_complement;
        self.flash(if self.show_complement {
            "complement strand shown"
        } else {
            "complement strand hidden"
        });
    }

    pub fn toggle_translation(&mut self) {
        self.show_translation = !self.show_translation;
        self.flash(if self.show_translation {
            "translation track shown"
        } else {
            "translation track hidden"
        });
    }

    /// The codon covering forward position `pos` in the current strand and
    /// reading frame, as `(forward start, amino acid)`. On the reverse strand
    /// the codon is read on the minus strand but reported in forward
    /// coordinates, so the reader can draw it under the right bases.
    pub fn codon_at(&self, pos: usize) -> Option<(usize, char)> {
        let seq = &self.record().bases;
        let n = seq.len();
        if pos >= n {
            return None;
        }
        if !self.reverse {
            if pos < self.frame {
                return None;
            }
            let start = pos - (pos - self.frame) % 3;
            (start + 3 <= n).then(|| (start, crate::fasta::translate::codon_to_aa(&seq[start..start + 3])))
        } else {
            let j = n - 1 - pos; // index on the minus strand
            if j < self.frame {
                return None;
            }
            let rc_start = j - (j - self.frame) % 3;
            if rc_start + 3 > n {
                return None;
            }
            let fwd_start = n - rc_start - 3;
            let codon: Vec<u8> = seq[fwd_start..fwd_start + 3]
                .iter()
                .rev()
                .map(|&b| Record::complement(b))
                .collect();
            Some((fwd_start, crate::fasta::translate::codon_to_aa(&codon)))
        }
    }

    /// Index of the residue under the cursor in the current translation.
    pub fn cursor_residue(&self) -> Option<usize> {
        let n = self.record_len();
        let along = if self.reverse {
            n.checked_sub(self.cursor + 1)?
        } else {
            self.cursor
        };
        along.checked_sub(self.frame).map(|d| d / 3)
    }

    /// Move the cursor `delta` residues along the strand in view.
    pub fn move_residues(&mut self, delta: isize) {
        let sign = if self.reverse { -1 } else { 1 };
        self.move_cursor(delta * 3 * sign);
    }

    pub fn cycle_theme(&mut self) {
        self.theme = self.theme.next();
        self.flash(format!("theme: {}", self.theme.name()));
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
        self.search.label = None;
    }

    /// Accept any IUPAC nucleotide code while typing a motif.
    pub fn push_search_char(&mut self, c: char) {
        let c = c.to_ascii_uppercase();
        if iupac::is_code(c as u8) {
            self.search.query.push(c);
        }
    }

    /// Search for a named site (used by the enzyme table) and show the reader.
    pub fn search_site(&mut self, label: &str, site: &str) {
        self.search.query = site.to_ascii_uppercase();
        self.search.label = Some(label.to_string());
        self.search.input_mode = false;
        self.recompute_matches();
        let hits = self.search.matches.len();
        if let Some(m) = self.search.matches.first().copied() {
            self.search.active = 0;
            self.jump_to(m.start);
        }
        self.set_view(View::Reader);
        self.flash_ok(format!("{label} ({site}) — {hits} site(s)"));
    }

    /// Toggle whether the motif's reverse complement counts as a hit.
    pub fn toggle_both_strands(&mut self) {
        self.search.both_strands = !self.search.both_strands;
        self.recompute_matches();
        self.flash(if self.search.both_strands {
            "Search: both strands"
        } else {
            "Search: forward strand only"
        });
    }

    pub fn pop_search_char(&mut self) {
        self.search.query.pop();
    }

    pub fn cancel_search(&mut self) {
        self.search.input_mode = false;
        self.search.query.clear();
        self.search.matches.clear();
        self.search.label = None;
    }

    /// Confirm the typed motif and jump to the first match.
    pub fn confirm_search(&mut self) {
        self.search.input_mode = false;
        self.recompute_matches();
        if let Some(m) = self.search.matches.first().copied() {
            self.search.active = 0;
            self.jump_to(m.start);
        } else if !self.search.query.is_empty() {
            self.flash_warn(format!("no match for {}", self.search.query));
        }
    }

    pub fn next_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.search.active = (self.search.active + 1) % self.search.matches.len();
        self.jump_to(self.search.matches[self.search.active].start);
    }

    pub fn prev_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        let n = self.search.matches.len();
        self.search.active = (self.search.active + n - 1) % n;
        self.jump_to(self.search.matches[self.search.active].start);
    }

    /// Move the cursor to an absolute base index and reveal it.
    pub fn jump_to(&mut self, pos: usize) {
        self.cursor = pos.min(self.record_len().saturating_sub(1));
        self.ensure_visible();
    }

    /// Recompute every motif occurrence in the current record, honouring IUPAC
    /// ambiguity codes and — when enabled — the minus strand.
    fn recompute_matches(&mut self) {
        self.search.matches.clear();
        self.search.active = 0;
        let query = self.search.query.clone();
        let q = query.as_bytes();
        if q.is_empty() {
            return;
        }
        let hay = &self.records[self.current].bases;

        let mut hits: Vec<MotifMatch> = iupac::find_all(hay, q)
            .into_iter()
            .map(|start| MotifMatch {
                start,
                len: q.len(),
                revcomp: false,
            })
            .collect();

        // The reverse-complement pass would only re-find the same positions for
        // a palindromic motif, so skip it there.
        if self.search.both_strands && !iupac::is_palindrome(q) {
            let rc = iupac::revcomp(q);
            hits.extend(iupac::find_all(hay, &rc).into_iter().map(|start| {
                MotifMatch {
                    start,
                    len: q.len(),
                    revcomp: true,
                }
            }));
            hits.sort_by_key(|m| (m.start, m.revcomp));
        }

        self.search.matches = hits;
    }

    // --- ORFs ------------------------------------------------------------

    pub fn orf_next(&mut self) {
        if self.orf.orfs.is_empty() {
            return;
        }
        self.orf.selected = (self.orf.selected + 1) % self.orf.orfs.len();
    }

    pub fn orf_prev(&mut self) {
        if self.orf.orfs.is_empty() {
            return;
        }
        let n = self.orf.orfs.len();
        self.orf.selected = (self.orf.selected + n - 1) % n;
    }

    /// Raise / lower the minimum ORF length and rescan.
    pub fn orf_min_step(&mut self, delta: isize) {
        let next = (self.orf.min_aa as isize + delta).clamp(1, 1000) as usize;
        if next == self.orf.min_aa {
            return;
        }
        self.orf.min_aa = next;
        let bases = &self.records[self.current].bases;
        self.orf.orfs = orf::find_orfs(bases, self.orf.min_aa);
        self.orf.selected = 0;
        self.flash(format!(
            "min ORF {} aa — {} found",
            self.orf.min_aa,
            self.orf.orfs.len()
        ));
    }

    /// Show the selected ORF in the reader, selected end to end.
    pub fn orf_goto(&mut self) {
        let Some(orf) = self.orf.selected_orf().copied() else {
            return;
        };
        // Reading the ORF's own strand and frame makes the Protein view agree
        // with the ORF table.
        self.reverse = orf.strand == crate::fasta::Strand::Reverse;
        self.frame = if self.reverse {
            (self.record_len() - orf.end) % 3
        } else {
            orf.start % 3
        };
        self.selection = Selection {
            active: true,
            anchor: orf.start,
        };
        self.cursor = orf.end.saturating_sub(1).min(self.record_len().saturating_sub(1));
        self.ensure_visible();
        self.set_view(View::Reader);
        self.flash(format!(
            "ORF {} — {} aa @ {}..{}",
            orf.frame_label(),
            orf.aa_len,
            orf.start + 1,
            orf.end
        ));
    }

    // --- Enzymes ---------------------------------------------------------

    pub fn enzyme_next(&mut self) {
        let n = self.enzymes.visible().len();
        if n == 0 {
            return;
        }
        self.enzymes.selected = (self.enzymes.selected + 1) % n;
        self.enzymes.changed_at = self.tick;
    }

    pub fn enzyme_prev(&mut self) {
        let n = self.enzymes.visible().len();
        if n == 0 {
            return;
        }
        self.enzymes.selected = (self.enzymes.selected + n - 1) % n;
        self.enzymes.changed_at = self.tick;
    }

    /// Add / remove the selected enzyme from the combined digest.
    pub fn toggle_enzyme_mark(&mut self) {
        let Some(name) = self.enzymes.selected_digest().map(|d| d.enzyme.name) else {
            return;
        };
        if let Some(i) = self.enzymes.marked.iter().position(|&m| m == name) {
            self.enzymes.marked.remove(i);
            self.flash(format!("{name} removed from digest"));
        } else {
            self.enzymes.marked.push(name);
            self.flash(format!("{name} added to digest ({} in total)", self.enzymes.marked.len()));
        }
        self.enzymes.changed_at = self.tick;
    }

    pub fn toggle_hide_noncutters(&mut self) {
        self.enzymes.hide_zero = !self.enzymes.hide_zero;
        self.enzymes.selected = 0;
        self.flash(if self.enzymes.hide_zero {
            "Enzymes: cutters only"
        } else {
            "Enzymes: all"
        });
    }

    /// Load the selected enzyme's site into the motif search.
    pub fn enzyme_search(&mut self) {
        let Some((name, site)) = self
            .enzymes
            .selected_digest()
            .map(|d| (d.enzyme.name, d.enzyme.site))
        else {
            return;
        };
        self.search_site(name, site);
    }

    // --- Selection -------------------------------------------------------

    /// Start a selection at the cursor, or drop the current one.
    pub fn toggle_selection(&mut self) {
        if self.selection.active {
            self.selection.active = false;
            self.flash("selection cleared");
        } else {
            self.selection = Selection {
                active: true,
                anchor: self.cursor,
            };
            self.flash("selection started — move to extend, y to copy");
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection.active = false;
    }

    /// The selected bases, read along the strand currently in view.
    pub fn selected_bases(&self) -> Option<Vec<u8>> {
        let (lo, hi) = self.selection.range(self.cursor)?;
        let seq = &self.record().bases;
        let hi = hi.min(seq.len());
        if lo >= hi {
            return None;
        }
        let slice = &seq[lo..hi];
        Some(if self.reverse {
            slice.iter().rev().map(|&b| Record::complement(b)).collect()
        } else {
            slice.to_vec()
        })
    }

    /// Melting temperature and friends for the current selection.
    pub fn selection_stats(&self) -> Option<oligo::OligoStats> {
        self.selected_bases().map(|b| oligo::analyze(&b))
    }
}

/// The smallest change to `top` that keeps `row` inside a viewport of
/// `visible` rows.
pub fn clamp_scroll(top: usize, row: usize, visible: usize) -> usize {
    if row < top {
        row
    } else if visible > 0 && row >= top + visible {
        row + 1 - visible
    } else {
        top
    }
}

/// Phase rotation contributed per base when the cursor moves.
pub const SCROLL_PHASE: f64 = 0.08;

/// Default minimum ORF length, in amino acids.
pub const DEFAULT_MIN_ORF_AA: usize = 30;

/// Length of one animation tick.
pub const TICK: std::time::Duration = std::time::Duration::from_millis(40);

/// Helix speeds were tuned for an 80 ms tick; scale them to the real one.
const HELIX_TICK_SCALE: f64 = 0.5;

/// How long a toast lives, slide-in and fade-out included (~2.8 s).
pub const TOAST_TICKS: u64 = 70;

/// The most toasts stacked at once.
const MAX_TOASTS: usize = 4;

/// Length of the fade-in when switching views.
pub const TRANSITION_TICKS: u64 = 8;
