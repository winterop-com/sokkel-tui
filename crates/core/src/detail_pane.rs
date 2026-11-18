//! Detail pane (parts/detail-pane): one thing's detail as a side panel holds it, such as the run
//! chosen in the page beside it. Titled sections one under the other, each with an optional
//! sentence, its facts as terms beside their values, and room for the app's own content after
//! them; the thing's verbs on a line at the end. It stands on the side panel's ground.
//!
//! In cells: a section pads two columns across (16 px) and a row above and below at regular (12
//! px; none at compact), its pieces with no row between (8 px is no row). Its heading is
//! `text.badge` upper case in `faint`; the summary `text.body`; terms `text.help` in
//! `muted-foreground` in one column (the longest term, at most 40 percent of the section), the
//! values beside them two columns on (12 px), a missing value a dash in `faint`. A rule in
//! `border` under every section but the last. The verbs follow, padded as a section is, a column
//! apart, wrapping to as many lines as they need.

use crate::button::Button;
use crate::cells::by_density;
use crate::look::Look;
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// One section of the pane: a title, an optional sentence, its facts, and rows the app draws.
#[derive(Debug, Clone, Default)]
pub struct PaneSection {
    pub title: String,
    pub summary: Option<String>,
    /// Terms beside their values; `None` is the quiet dash.
    pub facts: Vec<(String, Option<String>)>,
    /// Rows left after the facts for the app's own content (a preview, a run's steps): see
    /// [`DetailLayout::content`].
    pub content_rows: u16,
}

impl PaneSection {
    pub fn new(title: impl Into<String>) -> PaneSection {
        PaneSection {
            title: title.into(),
            ..PaneSection::default()
        }
    }

    pub fn summary(mut self, words: impl Into<String>) -> Self {
        self.summary = Some(words.into());
        self
    }

    pub fn facts<T: Into<String>, V: Into<String>>(mut self, facts: impl IntoIterator<Item = (T, Option<V>)>) -> Self {
        self.facts = facts.into_iter().map(|(t, v)| (t.into(), v.map(Into::into))).collect();
        self
    }

    /// Rows after the facts for the app to draw its own content in.
    pub fn content(mut self, rows: u16) -> Self {
        self.content_rows = rows;
        self
    }
}

/// Where the pane's pieces landed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetailLayout {
    /// Each section's room for the app's own content, by section, where it has any.
    pub content: Vec<(usize, Rect)>,
    /// Each verb, by its index.
    pub actions: Vec<(usize, Rect)>,
}

/// A detail pane.
#[derive(Debug, Clone)]
pub struct DetailPane<'a> {
    look: &'a Look,
    sections: Vec<PaneSection>,
    actions: Vec<Button<'a>>,
}

impl<'a> DetailPane<'a> {
    pub fn new(look: &'a Look) -> DetailPane<'a> {
        DetailPane {
            look,
            sections: Vec::new(),
            actions: Vec::new(),
        }
    }

    pub fn sections(mut self, sections: impl IntoIterator<Item = PaneSection>) -> Self {
        self.sections = sections.into_iter().collect();
        self
    }

    /// The thing's verbs, on a line at the end.
    pub fn actions(mut self, actions: impl IntoIterator<Item = Button<'a>>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    fn pad_y(&self) -> u16 {
        by_density(self.look, 1, 0)
    }

    /// The summary's lines at `width`.
    fn summary_lines(&self, s: &PaneSection, width: u16) -> Vec<String> {
        s.summary
            .as_deref()
            .map_or_else(Vec::new, |w| text::wrap(w, width.saturating_sub(4) as usize))
    }

    /// The verbs laid out from `x0` in `width`, a column apart, wrapping: each one's place as an
    /// offset (column, line).
    fn verb_places(&self, width: u16) -> Vec<(u16, u16)> {
        let room = width.saturating_sub(4);
        let (mut x, mut line) = (0u16, 0u16);
        let mut out = Vec::new();
        for b in &self.actions {
            let w = b.width();
            if x > 0 && x + w > room {
                x = 0;
                line += 1;
            }
            out.push((x, line));
            x += w + 1;
        }
        out
    }

    /// The rows it takes at `width` columns.
    pub fn height(&self, width: u16) -> u16 {
        let pad = self.pad_y();
        let mut h = 0;
        for (i, s) in self.sections.iter().enumerate() {
            h += pad + 1 + self.summary_lines(s, width).len() as u16 + s.facts.len() as u16 + s.content_rows + pad;
            if i + 1 < self.sections.len() {
                h += 1;
            }
        }
        if !self.actions.is_empty() {
            let lines = self.verb_places(width).last().map_or(0, |p| p.1 + 1);
            h += 1 + pad + lines + pad;
        }
        h
    }

    /// Where its pieces land when drawn at `area`.
    pub fn layout(&self, area: Rect) -> DetailLayout {
        let mut out = DetailLayout::default();
        let pad = self.pad_y();
        let (x, w) = (area.x + 2, area.width.saturating_sub(4));
        let mut y = area.y;
        for (i, s) in self.sections.iter().enumerate() {
            y += pad + 1 + self.summary_lines(s, area.width).len() as u16 + s.facts.len() as u16;
            if s.content_rows > 0 {
                out.content
                    .push((i, Rect::new(x, y, w, s.content_rows).intersection(area)));
            }
            y += s.content_rows + pad + u16::from(i + 1 < self.sections.len());
        }
        if !self.actions.is_empty() {
            y += 1 + pad;
            for (k, ((dx, line), b)) in self.verb_places(area.width).into_iter().zip(&self.actions).enumerate() {
                let r = Rect::new(x + dx, y + line, b.width(), 1);
                if r.bottom() <= area.bottom() {
                    out.actions.push((k, r));
                }
            }
        }
        out
    }

    /// The verb a press at `at` lands on, by its index.
    pub fn press(&self, area: Rect, at: Position) -> Option<usize> {
        self.layout(area)
            .actions
            .into_iter()
            .find(|(_, r)| r.contains(at))
            .map(|(k, _)| k)
    }
}

impl Widget for &DetailPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c| look.terminal(c);
        let ground = look
            .paint("side-panel.base.rest.panel", "ground")
            .unwrap_or_else(|| look.color("sidebar"));
        let pad = self.pad_y();
        let (x, w) = (area.x + 2, area.width.saturating_sub(4));
        let put = |buf: &mut Buffer, y: u16, x: u16, words: &str, room: u16, style: Style| {
            if y < area.bottom() {
                buf.set_stringn(x, y, words, room as usize, style.bg(t(ground)));
            }
        };
        let mut y = area.y;
        for (i, s) in self.sections.iter().enumerate() {
            y += pad;
            let heading = TextRole::Badge.style(look).fg(t(look.color("faint")));
            put(
                buf,
                y,
                x,
                &text::truncate(&s.title.to_uppercase(), w as usize, glyphs.ellipsis),
                w,
                heading,
            );
            y += 1;
            let body = TextRole::Body.style(look).fg(t(look.color("foreground")));
            for line in self.summary_lines(s, area.width) {
                put(buf, y, x, &line, w, body);
                y += 1;
            }
            // The terms in one column: the longest, at most 40 percent of the section.
            let term_w = (s.facts.iter().map(|(t, _)| text::width(t)).max().unwrap_or(0) as u16).min(w * 2 / 5);
            let term = TextRole::Help.style(look).fg(t(look.color("muted-foreground")));
            for (k, v) in &s.facts {
                put(
                    buf,
                    y,
                    x,
                    &text::truncate(k, term_w as usize, glyphs.ellipsis),
                    term_w,
                    term,
                );
                let vx = x + term_w + 2;
                let room = (x + w).saturating_sub(vx);
                match v {
                    Some(v) => put(
                        buf,
                        y,
                        vx,
                        &text::truncate(v, room as usize, glyphs.ellipsis),
                        room,
                        body,
                    ),
                    None => put(
                        buf,
                        y,
                        vx,
                        glyphs.missing,
                        room,
                        TextRole::Body.style(look).fg(t(look.color("faint"))),
                    ),
                }
                y += 1;
            }
            y += s.content_rows + pad;
            // A rule under every section but the last.
            if i + 1 < self.sections.len() {
                if y < area.bottom() {
                    let rule = look.color("border");
                    for rx in area.x..area.right() {
                        buf[(rx, y)].set_symbol(glyphs.rule).set_fg(t(rule)).set_bg(t(ground));
                    }
                }
                y += 1;
            }
        }
        if !self.actions.is_empty() {
            if y < area.bottom() {
                let rule = look.color("border");
                for rx in area.x..area.right() {
                    buf[(rx, y)].set_symbol(glyphs.rule).set_fg(t(rule)).set_bg(t(ground));
                }
            }
            for (k, r) in self.layout(area).actions {
                self.actions[k].clone().render(r, buf);
            }
        }
    }
}
