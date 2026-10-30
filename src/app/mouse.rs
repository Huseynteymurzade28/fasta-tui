//! Mouse handling: clicks and the wheel, resolved against the regions the
//! last frame recorded in [`HitMap`](super::HitMap).

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

use super::{App, View};

/// Rows scrolled per wheel notch in the reader.
const WHEEL_ROWS: isize = 2;

fn contains(r: Rect, x: u16, y: u16) -> bool {
    r.contains(Position { x, y })
}

impl App {
    pub fn handle_mouse(&mut self, ev: MouseEvent) {
        let (x, y) = (ev.column, ev.row);
        match ev.kind {
            MouseEventKind::Down(MouseButton::Left) => self.click(x, y),
            MouseEventKind::Drag(MouseButton::Left) => self.drag(x, y),
            MouseEventKind::ScrollDown => self.wheel(x, y, 1),
            MouseEventKind::ScrollUp => self.wheel(x, y, -1),
            _ => {}
        }
    }

    fn click(&mut self, x: u16, y: u16) {
        let hits = self.hits.borrow().clone();

        if self.palette.open {
            if let Some(list) = hits.palette_list.filter(|l| contains(l.body, x, y)) {
                self.palette.selected = list.offset + (y - list.body.y) as usize;
                self.palette_run();
            } else {
                self.close_palette();
            }
            return;
        }
        if self.show_help {
            self.show_help = false;
            return;
        }

        if let Some(&(_, view)) = hits.tabs.iter().find(|(r, _)| contains(*r, x, y)) {
            self.set_view(view);
            return;
        }
        if let Some(r) = hits.overview.filter(|r| contains(*r, x, y)) {
            self.overview_jump(r, x);
            return;
        }
        if let Some(pos) = self.reader_pos(x, y) {
            self.jump_to(pos);
            return;
        }
        if let Some(list) = hits.orf_table.filter(|l| contains(l.body, x, y)) {
            let i = list.offset + (y - list.body.y) as usize;
            if i < self.orf.orfs.len() {
                // A second click on the selected row opens it, like Enter.
                if i == self.orf.selected {
                    self.orf_goto();
                } else {
                    self.orf.selected = i;
                }
            }
            return;
        }
        if let Some(r) = hits.six_frame.filter(|r| contains(*r, x, y)) {
            self.six_frame_pick(r, x, y);
            return;
        }
        if let Some(list) = hits.enzyme_table.filter(|l| contains(l.body, x, y)) {
            let i = list.offset + (y - list.body.y) as usize;
            if i < self.enzymes.visible().len() {
                if i == self.enzymes.selected {
                    self.enzyme_search();
                } else {
                    self.enzymes.selected = i;
                    self.enzymes.changed_at = self.tick;
                }
            }
        }
    }

    /// Dragging across the reader extends a selection from where it began;
    /// dragging along the overview scrubs through the record.
    fn drag(&mut self, x: u16, y: u16) {
        let overview = self.hits.borrow().overview;
        if let Some(r) = overview.filter(|r| contains(*r, x, y)) {
            self.overview_jump(r, x);
            return;
        }
        if let Some(pos) = self.reader_pos(x, y) {
            if !self.selection.active {
                self.selection.active = true;
                self.selection.anchor = self.cursor;
            }
            self.jump_to(pos);
        }
    }

    fn wheel(&mut self, x: u16, y: u16, dir: isize) {
        let hits = self.hits.borrow().clone();
        let over = |r: Option<Rect>| r.is_some_and(|r| contains(r, x, y));
        if self.palette.open {
            self.palette_move(dir);
            return;
        }
        match self.view {
            View::Orf if over(hits.orf_table.map(|l| l.body)) || over(hits.six_frame) => {
                if dir > 0 { self.orf_next() } else { self.orf_prev() }
            }
            View::Enzymes => {
                if dir > 0 { self.enzyme_next() } else { self.enzyme_prev() }
            }
            View::Protein => {
                let row = self.metrics.get().residues_per_row.max(1) as isize;
                self.move_residues(dir * row);
            }
            _ => {
                let bpr = self.metrics.get().bases_per_row as isize;
                self.move_cursor(dir * WHEEL_ROWS * bpr);
            }
        }
    }

    fn overview_jump(&mut self, r: Rect, x: u16) {
        let len = self.record_len();
        if len == 0 || r.width == 0 {
            return;
        }
        let frac = (x - r.x) as f64 / r.width as f64;
        self.jump_to(((frac * len as f64) as usize).min(len - 1));
    }

    /// The base under a screen cell in the reader, if any.
    fn reader_pos(&self, x: u16, y: u16) -> Option<usize> {
        let hit = self.hits.borrow().reader?;
        if !contains(hit.rows, x, y) || x < hit.rows.x + hit.gutter {
            return None;
        }
        let line = (y - hit.rows.y) as usize;
        let row = hit.scroll_top + line / hit.lines_per_row.max(1);
        let col = (x - hit.rows.x - hit.gutter) as usize;
        // Each 10-base group is followed by one blank column.
        let (group, off) = (col / 11, col % 11);
        let within = group * 10 + off.min(9);
        if within >= hit.bases_per_row {
            return None;
        }
        let pos = row * hit.bases_per_row + within;
        (pos < self.record_len()).then_some(pos)
    }

    /// Select the ORF drawn under a click on the six-frame map.
    fn six_frame_pick(&mut self, r: Rect, x: u16, y: u16) {
        let len = self.record_len();
        let lane = (y - r.y) as usize;
        if len == 0 || lane >= 6 || r.width == 0 {
            return;
        }
        let pos = ((x - r.x) as f64 / r.width as f64 * len as f64) as usize;
        let hit = self.orf.orfs.iter().position(|o| {
            let l = match o.strand {
                crate::fasta::Strand::Forward => o.frame,
                crate::fasta::Strand::Reverse => 3 + o.frame,
            };
            l == lane && o.start <= pos && pos < o.end
        });
        if let Some(i) = hit {
            self.orf.selected = i;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ReaderHit;
    use crate::fasta::Record;

    fn app_with(len: usize) -> App {
        let mut rec = Record::new("t");
        rec.bases = vec![b'A'; len];
        App::new(vec![rec])
    }

    #[test]
    fn reader_clicks_map_to_bases() {
        let app = app_with(500);
        app.hits.borrow_mut().reader = Some(ReaderHit {
            rows: Rect { x: 10, y: 5, width: 60, height: 20 },
            lines_per_row: 4,
            bases_per_row: 40,
            scroll_top: 2,
            gutter: 7,
        });
        // First base of the first visible row.
        assert_eq!(app.reader_pos(17, 5), Some(80));
        // Second group starts after 10 bases and a blank column.
        assert_eq!(app.reader_pos(17 + 11, 5), Some(90));
        // The blank between groups snaps to the group's last base.
        assert_eq!(app.reader_pos(17 + 10, 5), Some(89));
        // Complement / translation lines belong to the same row.
        assert_eq!(app.reader_pos(17, 7), Some(80));
        assert_eq!(app.reader_pos(17, 9), Some(120));
        // The gutter is not a base.
        assert_eq!(app.reader_pos(12, 5), None);
    }

    #[test]
    fn overview_click_scrubs() {
        let mut app = app_with(1000);
        app.overview_jump(Rect { x: 0, y: 0, width: 100, height: 3 }, 50);
        assert_eq!(app.cursor, 500);
    }
}
