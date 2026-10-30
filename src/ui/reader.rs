//! The Reader view: genome overview, a double-stranded sequence browser with
//! a translation track, and the helix alongside.

use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use super::{fx, glow, helix, hints, overview, panel, thousands};
use crate::app::{clamp_scroll, App, ReaderHit, ReaderMetrics};
use crate::fasta::aa::AaClass;
use crate::fasta::Record;
use crate::theme::Palette;

/// Bases grouped together with a blank between groups, like a genome browser.
const GROUP: usize = 10;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    // Narrow terminals give the whole width to the sequence.
    let helix_w = if area.width >= 110 { area.width * 36 / 100 } else { 0 };
    let [left, right] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(helix_w)]).areas(area);

    let sel_height = if app.selection.active { 3 } else { 0 };
    let [over, seq, sel] = Layout::vertical([
        Constraint::Length(5),
        Constraint::Min(0),
        Constraint::Length(sel_height),
    ])
    .areas(left);

    overview::render(frame, over, app);
    render_sequence(frame, seq, app);
    if app.selection.active {
        render_selection_panel(frame, sel, app);
    }
    if helix_w > 0 {
        helix::render(frame, right, app);
    }
}

/// Which extra lines accompany each sequence row.
struct Tracks {
    complement: bool,
    translation: bool,
}

impl Tracks {
    fn lines_per_row(&self) -> usize {
        let extra = self.complement as usize + self.translation as usize;
        // A blank spacer keeps multi-line rows from running together.
        if extra > 0 { 2 + extra } else { 1 }
    }
}

fn render_sequence(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let seq = &app.record().bases;
    let len = seq.len();
    let tracks = Tracks {
        complement: app.show_complement,
        translation: app.show_translation,
    };

    let frame_label = format!(
        "{} strand · frame {}",
        if app.reverse { "−" } else { "+" },
        app.frame + 1
    );
    let block = panel("≋", "Sequence", &p)
        .title(
            Line::from(Span::styled(format!(" {frame_label} "), Style::default().fg(p.muted)))
                .right_aligned(),
        )
        .title_bottom(hints(
            &[("c", "complement"), ("a", "translation"), ("f", "frame"), ("r", "strand")],
            &p,
        ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    glow(frame, area, app);

    if len == 0 || inner.height < 2 {
        frame.render_widget(Paragraph::new("<empty sequence>"), inner);
        return;
    }

    // Size each row to the width: a gutter, then as many 10-base groups as
    // fit (each group plus its trailing blank is 11 columns). One column is
    // left free on the right for breathing room next to the scrollbar.
    let digits = len.to_string().len().max(4);
    let gutter = digits + 3;
    let usable = (inner.width as usize).saturating_sub(gutter + 1);
    let groups = ((usable + 1) / (GROUP + 1)).max(1);
    let bases_per_row = groups * GROUP;

    let lpr = tracks.lines_per_row();
    let body = Rect { y: inner.y + 1, height: inner.height - 1, ..inner };
    // The last row may lose its blank spacer line, nothing more.
    let h = body.height as usize;
    let visible_rows = if lpr > 1 { (h + 1) / lpr } else { h }.max(1);

    let prev = app.metrics.get();
    app.metrics.set(ReaderMetrics { bases_per_row, visible_rows, ..prev });

    let total_rows = len.div_ceil(bases_per_row);
    let cursor_row = app.cursor / bases_per_row;
    let top = clamp_scroll(app.scroll_top.get(), cursor_row, visible_rows)
        .min(total_rows.saturating_sub(1));
    app.scroll_top.set(top);

    app.hits.borrow_mut().reader = Some(ReaderHit {
        rows: body,
        lines_per_row: lpr,
        bases_per_row,
        scroll_top: top,
        gutter: gutter as u16,
    });

    let mut lines = vec![ruler(app, &p, digits, bases_per_row)];
    let styles = BaseStyler::new(app, &p);
    for row in top..(top + visible_rows).min(total_rows) {
        let start = row * bases_per_row;
        let end = (start + bases_per_row).min(len);
        let is_cursor_row = row == cursor_row;
        let row_bg = if is_cursor_row { Some(p.surface) } else { None };

        // Forward strand.
        let mut spans = vec![
            Span::styled(
                format!("{:>digits$} ", start + 1),
                Style::default().fg(if is_cursor_row { p.accent } else { p.muted }),
            ),
            Span::styled("│ ", Style::default().fg(p.border)),
        ];
        for k in start..end {
            if k > start && (k - start) % GROUP == 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled((seq[k] as char).to_string(), styles.forward(k)));
        }
        lines.push(with_bg(Line::from(spans), row_bg));

        if tracks.complement {
            let mut spans = vec![
                Span::styled(format!("{:>digits$} ", ""), Style::default()),
                Span::styled("┊ ", Style::default().fg(p.border)),
            ];
            for k in start..end {
                if k > start && (k - start) % GROUP == 0 {
                    spans.push(Span::raw(" "));
                }
                let comp = Record::complement(seq[k]) as char;
                spans.push(Span::styled(comp.to_string(), styles.complement(k, comp)));
            }
            lines.push(with_bg(Line::from(spans), row_bg));
        }

        if tracks.translation {
            let mut spans = vec![
                Span::styled(format!("{:>digits$} ", ""), Style::default()),
                Span::styled("┊ ", Style::default().fg(p.border)),
            ];
            for k in start..end {
                if k > start && (k - start) % GROUP == 0 {
                    spans.push(Span::raw(" "));
                }
                spans.push(styles.translation(k));
            }
            lines.push(with_bg(Line::from(spans), row_bg));
        }

        if lpr > 1 {
            lines.push(Line::from(""));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);

    // A slim scrollbar riding on the right border.
    if total_rows > visible_rows {
        let mut state = ScrollbarState::new(total_rows.saturating_sub(visible_rows))
            .viewport_content_length(visible_rows)
            .position(top);
        let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some("│"))
            .thumb_symbol("┃")
            .track_style(Style::default().fg(p.border))
            .thumb_style(Style::default().fg(p.accent));
        frame.render_stateful_widget(bar, area.inner(Margin { vertical: 1, horizontal: 0 }), &mut state);
    }
}

/// Paint a whole line's background (for the cursor-row band).
fn with_bg(line: Line<'static>, bg: Option<Color>) -> Line<'static> {
    match bg {
        Some(bg) => {
            let spans = line
                .spans
                .into_iter()
                .map(|s| {
                    // Leave already-colored cells (matches, codons) alone.
                    if s.style.bg.is_none() {
                        let style = s.style.bg(bg);
                        Span::styled(s.content, style)
                    } else {
                        s
                    }
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        }
        None => line,
    }
}

/// Column ruler above the rows: a tick and offset at each group start, and
/// the cursor's column marked.
fn ruler(app: &App, p: &Palette, digits: usize, bases_per_row: usize) -> Line<'static> {
    let col = |k: usize| k + k / GROUP; // display column of the k-th base
    let width = col(bases_per_row - 1) + 1;
    let muted = Style::default().fg(p.muted);
    let mut cells: Vec<(char, Style)> = vec![(' ', Style::default()); width];

    for g in (0..bases_per_row).step_by(GROUP) {
        cells[col(g)] = ('╹', muted);
        if g + 4 < bases_per_row {
            cells[col(g + 4)] = ('·', Style::default().fg(p.border));
        }
        for (i, ch) in (g + 1).to_string().chars().enumerate() {
            let k = g + 1 + i;
            if k < bases_per_row && k < g + GROUP {
                cells[col(k)] = (ch, muted);
            }
        }
    }
    let cursor = app.cursor % bases_per_row;
    cells[col(cursor)] = ('▼', Style::default().fg(p.accent).add_modifier(Modifier::BOLD));

    let mut spans = vec![Span::raw(format!("{:>digits$}   ", ""))];
    spans.extend(cells.into_iter().map(|(ch, st)| Span::styled(ch.to_string(), st)));
    Line::from(spans)
}

/// Resolves the style of each base from the layers that can apply to it:
/// nucleotide color, in-frame start/stop codons, motif hits, the selection
/// and the cursor.
struct BaseStyler<'a> {
    app: &'a App,
    p: &'a Palette,
    selection: Option<(usize, usize)>,
    cursor_codon: Option<usize>,
}

impl<'a> BaseStyler<'a> {
    fn new(app: &'a App, p: &'a Palette) -> Self {
        BaseStyler {
            app,
            p,
            selection: app.selection.range(app.cursor),
            cursor_codon: app.codon_at(app.cursor).map(|(s, _)| s),
        }
    }

    fn in_selection(&self, k: usize) -> bool {
        self.selection.is_some_and(|(lo, hi)| k >= lo && k < hi)
    }

    fn motif(&self, k: usize) -> Option<(bool, bool)> {
        // Matches are sorted by start; binary search the candidates.
        let m = &self.app.search.matches;
        if m.is_empty() {
            return None;
        }
        let idx = m.partition_point(|x| x.start <= k);
        let active = self.app.search.active_match();
        m[..idx]
            .iter()
            .rev()
            .take(16)
            .find(|x| k < x.end())
            .map(|x| (x.revcomp, Some(*x) == active))
    }

    fn forward(&self, k: usize) -> Style {
        let p = self.p;
        let base = self.app.record().bases[k] as char;
        let mut style = Style::default().fg(p.base_color(base));

        // In-frame start and stop codons get a soft tinted background.
        if let Some((_, aa)) = self.app.codon_at(k) {
            match aa {
                'M' => style = style.bg(fx::lerp(p.bg, p.start_bg, 0.55)).add_modifier(Modifier::BOLD),
                '*' => style = style.bg(fx::lerp(p.bg, p.stop_bg, 0.6)).add_modifier(Modifier::BOLD),
                _ => {}
            }
        }
        if self.in_selection(k) {
            style = style.bg(fx::lerp(p.bg, p.match_bg, 0.3));
        }
        if let Some((revcomp, active)) = self.motif(k) {
            let bg = if active { p.accent } else { p.match_bg };
            style = Style::default().fg(fx::contrast_text(bg)).bg(bg).add_modifier(Modifier::BOLD);
            if revcomp {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
        }
        if k == self.app.cursor {
            // A gently breathing cursor is easy to find without being loud.
            let bg = fx::lerp(p.accent, p.text, 0.35 * fx::pulse(self.app.tick, 30));
            style = Style::default().fg(fx::contrast_text(bg)).bg(bg).add_modifier(Modifier::BOLD);
        }
        style
    }

    fn complement(&self, k: usize, comp: char) -> Style {
        let p = self.p;
        let mut style = Style::default().fg(fx::lerp(p.base_color(comp), p.bg, 0.45));
        if self.in_selection(k) {
            style = style.bg(fx::lerp(p.bg, p.match_bg, 0.18));
        }
        if k == self.app.cursor {
            style = style.fg(p.accent).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        }
        style
    }

    /// The translation track: each residue sits under its codon's middle
    /// base, and alternate codons are shaded so frame boundaries show.
    fn translation(&self, k: usize) -> Span<'static> {
        let p = self.p;
        let Some((start, aa)) = self.app.codon_at(k) else {
            return Span::raw(" ");
        };
        let shade = if (start / 3) % 2 == 0 { Some(p.surface) } else { None };
        let mut style = Style::default();
        if let Some(bg) = shade {
            style = style.bg(bg);
        }
        if Some(start) == self.cursor_codon {
            style = style.bg(p.surface_hi);
        }
        if k != start + 1 {
            return Span::styled(" ", style);
        }
        let class = AaClass::of(aa);
        style = match aa {
            'M' => style.fg(fx::contrast_text(p.start_bg)).bg(p.start_bg).add_modifier(Modifier::BOLD),
            '*' => style.fg(fx::contrast_text(p.stop_bg)).bg(p.stop_bg).add_modifier(Modifier::BOLD),
            _ => style.fg(p.aa_color(class)),
        };
        Span::styled(aa.to_string(), style)
    }
}

/// The oligo panel under the reader: length, composition and melting
/// temperature for whatever the user has selected.
fn render_selection_panel(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let Some((lo, hi)) = app.selection.range(app.cursor) else {
        return;
    };
    let hi = hi.min(app.record_len());

    let block = panel(
        "▭",
        format!(
            "Selection {}..{} · {} strand",
            thousands(lo + 1),
            thousands(hi),
            if app.reverse { "reverse" } else { "forward" }
        ),
        &p,
    )
    .border_style(Style::default().fg(p.match_bg))
    .title(hints(&[("y", "copy FASTA"), ("Esc", "clear")], &p).right_aligned());

    let line = match app.selection_stats() {
        Some(s) => {
            let key = |t: &str| Span::styled(format!(" {t} "), Style::default().fg(p.muted));
            let val = |t: String| {
                Span::styled(t, Style::default().fg(p.text).add_modifier(Modifier::BOLD))
            };
            // The nearest-neighbour Tm is the one to trust; Wallace is shown as
            // a sanity check and is all we have for ambiguous sequence.
            let tm = match s.tm_nn {
                Some(tm) => format!("{tm:.1} °C (NN)"),
                None => format!("{:.1} °C (Wallace)", s.tm_wallace),
            };
            let mut spans = vec![key("len"), val(format!("{} nt", s.len)), key("  GC")];
            spans.extend(fx::hbar(6, s.gc, p.base_a, p.base_g, Some(p.surface)));
            spans.push(val(format!(" {:.1}%", s.gc * 100.0)));
            spans.extend([
                key("  Tm"),
                val(tm),
                key("  MW"),
                val(format!("{:.1} kDa", s.mw / 1000.0)),
            ]);
            if s.ambiguous > 0 {
                spans.push(key("  ambiguous"));
                spans.push(val(format!("{}", s.ambiguous)));
            }
            Line::from(spans)
        }
        None => Line::from(Span::styled(" empty selection", Style::default().fg(p.muted))),
    };

    frame.render_widget(Paragraph::new(line).block(block), area);
}
