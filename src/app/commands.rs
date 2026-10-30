//! The command palette: every action by name, fuzzy-filtered as you type.
//! Typing a number offers "go to position", typing a motif offers a search.

use super::{App, View};
use crate::fasta::iupac;

/// Something the palette can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    View(View),
    NextRecord,
    PrevRecord,
    Search,
    ToggleBothStrands,
    ToggleReverse,
    CycleFrame,
    CycleTheme,
    ToggleComplement,
    ToggleTranslation,
    TogglePause,
    ToggleSelection,
    CopySelection,
    GotoStart,
    GotoEnd,
    OrfMinUp,
    OrfMinDown,
    CuttersOnly,
    Help,
    Quit,
    /// Jump to a 1-based base position.
    Goto(usize),
}

/// A named palette entry with the key that does the same thing directly.
#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    pub cmd: Command,
    pub name: &'static str,
    pub key: &'static str,
}

pub const COMMANDS: &[CommandSpec] = &[
    CommandSpec { cmd: Command::View(View::Reader), name: "Open Reader", key: "1" },
    CommandSpec { cmd: Command::View(View::Stats), name: "Open Stats", key: "2" },
    CommandSpec { cmd: Command::View(View::Protein), name: "Open Protein", key: "3" },
    CommandSpec { cmd: Command::View(View::Orf), name: "Open ORF finder", key: "4" },
    CommandSpec { cmd: Command::View(View::Enzymes), name: "Open restriction map", key: "5" },
    CommandSpec { cmd: Command::Search, name: "Search motif", key: "/" },
    CommandSpec { cmd: Command::ToggleBothStrands, name: "Toggle reverse-complement matching", key: "b" },
    CommandSpec { cmd: Command::ToggleReverse, name: "Toggle reverse strand", key: "r" },
    CommandSpec { cmd: Command::CycleFrame, name: "Cycle reading frame", key: "f" },
    CommandSpec { cmd: Command::ToggleComplement, name: "Toggle complement strand track", key: "c" },
    CommandSpec { cmd: Command::ToggleTranslation, name: "Toggle translation track", key: "a" },
    CommandSpec { cmd: Command::ToggleSelection, name: "Start / drop selection", key: "v" },
    CommandSpec { cmd: Command::CopySelection, name: "Copy selection as FASTA", key: "y" },
    CommandSpec { cmd: Command::GotoStart, name: "Go to sequence start", key: "g" },
    CommandSpec { cmd: Command::GotoEnd, name: "Go to sequence end", key: "G" },
    CommandSpec { cmd: Command::NextRecord, name: "Next record", key: "Tab" },
    CommandSpec { cmd: Command::PrevRecord, name: "Previous record", key: "S-Tab" },
    CommandSpec { cmd: Command::OrfMinUp, name: "Raise minimum ORF length", key: "M" },
    CommandSpec { cmd: Command::OrfMinDown, name: "Lower minimum ORF length", key: "m" },
    CommandSpec { cmd: Command::CuttersOnly, name: "Toggle cutters-only enzyme list", key: "u" },
    CommandSpec { cmd: Command::CycleTheme, name: "Cycle color theme", key: "t" },
    CommandSpec { cmd: Command::TogglePause, name: "Pause / resume helix", key: "Space" },
    CommandSpec { cmd: Command::Help, name: "Show key bindings", key: "?" },
    CommandSpec { cmd: Command::Quit, name: "Quit", key: "q" },
];

/// One row of the filtered palette.
#[derive(Debug, Clone)]
pub struct Entry {
    pub cmd: Option<Command>,
    pub name: String,
    pub key: &'static str,
    /// Character positions in `name` that matched the query, for highlighting.
    pub hits: Vec<usize>,
}

/// Subsequence fuzzy match. Returns a score (higher is better) and the
/// matched positions, or `None` when `query` is not a subsequence of `text`.
/// Consecutive runs and word starts score extra, like most editors' pickers.
pub fn fuzzy(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    if query.is_empty() {
        return Some((0, Vec::new()));
    }
    let text: Vec<char> = text.chars().collect();
    let mut hits = Vec::new();
    let mut score = 0;
    let mut ti = 0;
    let mut prev: Option<usize> = None;
    for qc in query.chars().filter(|c| !c.is_whitespace()) {
        let qc = qc.to_ascii_lowercase();
        let found = (ti..text.len()).find(|&i| text[i].to_ascii_lowercase() == qc)?;
        score += 1;
        if prev == Some(found.wrapping_sub(1)) {
            score += 5;
        }
        if found == 0 || text[found - 1] == ' ' {
            score += 8;
        }
        score -= (found - ti) as i32 / 4;
        hits.push(found);
        prev = Some(found);
        ti = found + 1;
    }
    Some((score, hits))
}

/// Parse `"12345"`, `"12,345"` or `"12.5k"` as a position.
fn parse_position(q: &str) -> Option<usize> {
    let q = q.trim().replace([',', '_'], "");
    if let Some(k) = q.strip_suffix(['k', 'K']) {
        return k.parse::<f64>().ok().map(|v| (v * 1000.0) as usize);
    }
    q.parse().ok()
}

impl App {
    /// The palette rows for the current query, best match first.
    pub fn palette_entries(&self) -> Vec<Entry> {
        let q = self.palette.query.trim();
        let mut out = Vec::new();

        if let Some(pos) = parse_position(q) {
            out.push(Entry {
                cmd: Some(Command::Goto(pos)),
                name: format!("Go to position {pos}"),
                key: "",
                hits: Vec::new(),
            });
        }

        let mut scored: Vec<(i32, &CommandSpec, Vec<usize>)> = COMMANDS
            .iter()
            .filter_map(|spec| fuzzy(q, spec.name).map(|(s, h)| (s, spec, h)))
            .collect();
        scored.sort_by_key(|e| std::cmp::Reverse(e.0));
        out.extend(scored.into_iter().map(|(_, spec, hits)| Entry {
            cmd: Some(spec.cmd),
            name: spec.name.to_string(),
            key: spec.key,
            hits,
        }));

        // A query that reads as DNA is offered as a motif search too.
        let motif = q.to_ascii_uppercase();
        if motif.len() >= 3 && motif.bytes().all(iupac::is_code) {
            out.push(Entry {
                cmd: None,
                name: format!("Search motif {motif}"),
                key: "/",
                hits: Vec::new(),
            });
        }
        out
    }

    pub fn open_palette(&mut self) {
        self.palette.open = true;
        self.palette.query.clear();
        self.palette.selected = 0;
    }

    pub fn close_palette(&mut self) {
        self.palette.open = false;
    }

    pub fn palette_move(&mut self, delta: isize) {
        let n = self.palette_entries().len();
        if n == 0 {
            return;
        }
        self.palette.selected = (self.palette.selected as isize + delta).rem_euclid(n as isize) as usize;
    }

    /// Run the highlighted palette row.
    pub fn palette_run(&mut self) {
        let entries = self.palette_entries();
        let Some(entry) = entries.get(self.palette.selected).cloned() else {
            return;
        };
        self.close_palette();
        match entry.cmd {
            Some(cmd) => self.run_command(cmd),
            None => {
                let motif = self.palette.query.trim().to_ascii_uppercase();
                self.search.query = motif;
                self.search.label = None;
                self.confirm_search();
                self.set_view(View::Reader);
            }
        }
    }

    pub fn run_command(&mut self, cmd: Command) {
        match cmd {
            Command::View(v) => self.set_view(v),
            Command::NextRecord => self.next_record(),
            Command::PrevRecord => self.prev_record(),
            Command::Search => {
                self.set_view(View::Reader);
                self.begin_search();
            }
            Command::ToggleBothStrands => self.toggle_both_strands(),
            Command::ToggleReverse => self.toggle_reverse(),
            Command::CycleFrame => self.cycle_frame(),
            Command::CycleTheme => self.cycle_theme(),
            Command::ToggleComplement => self.toggle_complement(),
            Command::ToggleTranslation => self.toggle_translation(),
            Command::TogglePause => self.toggle_pause(),
            Command::ToggleSelection => self.toggle_selection(),
            Command::CopySelection => self.copy_selection(),
            Command::GotoStart => self.goto_start(),
            Command::GotoEnd => self.goto_end(),
            Command::OrfMinUp => self.orf_min_step(10),
            Command::OrfMinDown => self.orf_min_step(-10),
            Command::CuttersOnly => self.toggle_hide_noncutters(),
            Command::Help => self.toggle_help(),
            Command::Quit => self.should_quit = true,
            Command::Goto(pos) => {
                let len = self.record_len();
                if len == 0 {
                    return;
                }
                let target = pos.clamp(1, len);
                self.jump_to(target - 1);
                self.set_view(View::Reader);
                self.flash_ok(format!("jumped to {target}"));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_prefers_word_starts() {
        let (a, _) = fuzzy("rs", "Toggle reverse strand").unwrap();
        let b = fuzzy("rs", "Cycle color theme").map_or(i32::MIN, |(s, _)| s);
        assert!(a > b);
        assert!(fuzzy("xyz", "Quit").is_none());
        assert_eq!(fuzzy("qt", "Quit").unwrap().1, vec![0, 3]);
    }

    #[test]
    fn positions_parse() {
        assert_eq!(parse_position("1,234"), Some(1234));
        assert_eq!(parse_position("2.5k"), Some(2500));
        assert_eq!(parse_position("abc"), None);
    }
}
