//! The genome overview: the whole record squeezed into three tracks — a GC
//! heatmap at double horizontal resolution, ORFs on both strands, and the
//! viewport with motif hits. Click or drag on it to jump.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Frame,
};

use super::{fx, panel, thousands};
use crate::app::App;
use crate::fasta::{stats, Strand};

/// Width of the row labels on the left.
const LABEL: u16 = 4;
/// Smallest window, in bases, behind one GC heatmap sample.
const MIN_GC_WINDOW: usize = 40;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let p = app.theme.palette();
    let record = app.record();
    let len = record.len();

    let title = if record.description.is_empty() {
        record.id.clone()
    } else {
        format!("{} — {}", record.id, record.description)
    };
    let cutters = app.enzymes.digests.iter().filter(|d| d.count() > 0).count();
    let summary = Line::from(vec![
        Span::styled(" GC ", Style::default().fg(p.muted)),
        Span::styled(
            format!("{:.1}%", app.cache.gc * 100.0),
            Style::default().fg(p.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ORFs ", Style::default().fg(p.muted)),
        Span::styled(app.orf.orfs.len().to_string(), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
        Span::styled("  cutters ", Style::default().fg(p.muted)),
        Span::styled(format!("{cutters} "), Style::default().fg(p.text).add_modifier(Modifier::BOLD)),
    ])
    .right_aligned();
    let block = panel("◈", title, &p).title(summary);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 || inner.width <= LABEL + 2 || len == 0 {
        return;
    }
    let labels = ["GC", "ORF", "pos"];
    for (i, l) in labels.iter().enumerate() {
        frame.render_widget(
            Span::styled(format!("{l:<4}"), Style::default().fg(p.muted)),
            Rect { x: inner.x, y: inner.y + i as u16, width: LABEL, height: 1 },
        );
    }
    let track = Rect { x: inner.x + LABEL, width: inner.width - LABEL, ..inner };
    app.hits.borrow_mut().overview = Some(Rect { height: 3, ..track });

    let buf = frame.buffer_mut();
    draw_gc(buf, track, app);
    draw_orfs(buf, Rect { y: track.y + 1, ..track }, app);
    draw_position(buf, Rect { y: track.y + 2, ..track }, app);

    // Scale labels for the ends of the position track.
    let end = format!(" {} ", thousands(len));
    let w = end.len() as u16;
    if track.width > w + 8 {
        let at = Rect { x: track.right() - w, y: area.bottom() - 1, width: w, height: 1 };
        frame.render_widget(Span::styled(end, Style::default().fg(p.muted)), at);
    }
}

/// The base range `[lo, hi)` covered by bin `i` of `bins`.
fn bin(i: usize, bins: usize, len: usize) -> (usize, usize) {
    let lo = i * len / bins;
    let hi = ((i + 1) * len / bins).max(lo + 1).min(len);
    (lo.min(len.saturating_sub(1)), hi)
}

/// GC heatmap. Each cell is `▌` with the left half-bin as foreground and the
/// right half-bin as background, doubling the horizontal resolution.
fn draw_gc(buf: &mut Buffer, area: Rect, app: &App) {
    let len = app.record_len();
    let w = area.width as usize;
    let bins = w * 2;
    // Short records have bins only a base or two wide; widen each sample to a
    // minimum window so the map shows a trend rather than a barcode.
    let half = (len / bins / 2).max(MIN_GC_WINDOW / 2);
    let color = |i: usize| {
        let (lo, hi) = bin(i, bins, len);
        let mid = (lo + hi) / 2;
        let (lo, hi) = (mid.saturating_sub(half), (mid + half).min(len));
        let gc = stats::gc_between(&app.cache.gc_prefix, lo, hi);
        // 20 %…80 % spans the ramp; that covers nearly all real sequence.
        fx::heat((gc - 0.2) / 0.6)
    };
    for x in 0..w {
        if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y)) {
            cell.set_symbol("▌").set_fg(color(2 * x)).set_bg(color(2 * x + 1));
        }
    }
}

/// ORFs: forward-strand ORFs fill the top half of the cell, reverse-strand
/// ones the bottom half; the selected ORF is drawn in the accent color.
fn draw_orfs(buf: &mut Buffer, area: Rect, app: &App) {
    let p = app.theme.palette();
    let len = app.record_len();
    let w = area.width as usize;
    let mut top: Vec<Option<Color>> = vec![None; w];
    let mut bottom: Vec<Option<Color>> = vec![None; w];
    let selected = app.orf.selected_orf().copied();

    // Selected last so it wins any shared cell.
    let mut orfs: Vec<_> = app.orf.orfs.iter().collect();
    orfs.sort_by_key(|o| Some(**o) == selected);
    for orf in orfs {
        let x0 = orf.start * w / len;
        let x1 = ((orf.end * w).div_ceil(len)).clamp(x0 + 1, w);
        let is_sel = Some(*orf) == selected;
        let (lane, color) = match orf.strand {
            Strand::Forward => (&mut top, if is_sel { p.accent } else { p.strand_a }),
            Strand::Reverse => (&mut bottom, if is_sel { p.accent } else { p.strand_b }),
        };
        for c in lane.iter_mut().take(x1).skip(x0) {
            *c = Some(color);
        }
    }

    for x in 0..w {
        let Some(cell) = buf.cell_mut((area.x + x as u16, area.y)) else {
            continue;
        };
        match (top[x], bottom[x]) {
            (Some(t), Some(b)) => cell.set_symbol("▀").set_fg(t).set_bg(b),
            (Some(t), None) => cell.set_symbol("▀").set_fg(t),
            (None, Some(b)) => cell.set_symbol("▄").set_fg(b),
            (None, None) => cell.set_symbol("╌").set_fg(p.surface_hi),
        };
    }
}

/// The viewport bracket, the selection, motif hits and the cursor.
fn draw_position(buf: &mut Buffer, area: Rect, app: &App) {
    let p = app.theme.palette();
    let len = app.record_len();
    let w = area.width as usize;
    let to_x = |pos: usize| (pos * w / len).min(w - 1);

    let m = app.metrics.get();
    let view_lo = app.scroll_top.get() * m.bases_per_row;
    let view_hi = (view_lo + m.visible_rows * m.bases_per_row).min(len);
    let (vx0, vx1) = (to_x(view_lo), to_x(view_hi.saturating_sub(1)));

    for x in 0..w {
        if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y)) {
            if x >= vx0 && x <= vx1 {
                let t = if vx1 > vx0 { (x - vx0) as f64 / (vx1 - vx0) as f64 } else { 0.0 };
                cell.set_symbol("━").set_fg(fx::gradient(&p.grad, t)).set_bg(p.surface);
            } else {
                cell.set_symbol("─").set_fg(p.border);
            }
        }
    }
    if let Some((lo, hi)) = app.selection.range(app.cursor) {
        for x in to_x(lo)..=to_x(hi.min(len).saturating_sub(1)) {
            if let Some(cell) = buf.cell_mut((area.x + x as u16, area.y)) {
                cell.set_bg(fx::lerp(p.bg, p.match_bg, 0.35));
            }
        }
    }
    for hit in &app.search.matches {
        if let Some(cell) = buf.cell_mut((area.x + to_x(hit.start) as u16, area.y)) {
            cell.set_symbol("┃").set_fg(p.match_bg);
        }
    }
    if let Some(cell) = buf.cell_mut((area.x + to_x(app.cursor) as u16, area.y)) {
        cell.set_symbol("◆").set_fg(p.text).set_style(Style::default().add_modifier(Modifier::BOLD));
    }
}

#[cfg(test)]
mod tests {
    use super::bin;

    #[test]
    fn bins_cover_record() {
        assert_eq!(bin(0, 4, 100), (0, 25));
        assert_eq!(bin(3, 4, 100), (75, 100));
        // More bins than bases still yields non-empty ranges.
        let (lo, hi) = bin(9, 10, 3);
        assert!(lo < hi && hi <= 3);
    }
}
