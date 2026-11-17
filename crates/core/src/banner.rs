//! Banner: a state that holds until it changes (the server not answering, a read-only instance),
//! in a row across the top of the content, from sokkel-design's `patterns/shell` (its `banner`
//! slot, between the top bar and the content, not scrolling) and sokkel-ui's `Banner`.
//!
//! The contract has no banner part yet, so its paints are sokkel-tui's reading (proposal c5): the
//! web's tone at 12 percent is a mix a renderer may not make (principle 20), so a critical banner
//! stands on page-state's failed frame and a warning one on its failed+warning frame, both resolved
//! and opaque; info and good stand on `card`. The mark and the words take the tone's ink. One row:
//! the words cut short, [`Banner::cut`] saying so, so the whole of them can go to the status line.

use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

/// A banner's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BannerTone {
    Critical,
    Warning,
    #[default]
    Info,
    Good,
}

/// A row across the content for a state that holds.
#[derive(Debug, Clone)]
pub struct Banner<'a> {
    look: &'a Look,
    tone: BannerTone,
    words: String,
}

impl<'a> Banner<'a> {
    pub fn new(look: &'a Look, tone: BannerTone, words: impl Into<String>) -> Banner<'a> {
        Banner {
            look,
            tone,
            words: words.into(),
        }
    }

    pub fn words(&self) -> &str {
        &self.words
    }

    /// The pad at each end, the mark and the space after it.
    fn lead(&self) -> u16 {
        1 + 2
    }

    /// Whether its words are cut short at `width` columns.
    pub fn cut(&self, width: u16) -> bool {
        text::width(&self.words) as u16 + self.lead() + 1 > width
    }

    fn colors(&self) -> (Rgba, Rgba) {
        let look = self.look;
        let frame = |state: &str| look.paint(&format!("page-state.base.{state}.frame"), "ground");
        let card = look.color("card");
        match self.tone {
            BannerTone::Critical => (frame("failed").unwrap_or(card), look.color("critical-ink")),
            BannerTone::Warning => (frame("failed+warning").unwrap_or(card), look.color("warning-ink")),
            BannerTone::Info => (card, look.color("info-ink")),
            BannerTone::Good => (card, look.color("good-ink")),
        }
    }
}

impl Widget for &Banner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 6 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let (ground, ink) = self.colors();
        let row = Rect { height: 1, ..area };
        buf.set_style(row, Style::new().bg(t(ground)));
        let mark = match self.tone {
            BannerTone::Critical => glyphs.issue_critical,
            BannerTone::Warning => glyphs.issue_warning,
            BannerTone::Info => glyphs.info,
            BannerTone::Good => glyphs.check,
        };
        let style = Style::new().fg(t(ink)).bg(t(ground));
        buf.set_string(row.x + 1, row.y, mark, style.add_modifier(Modifier::BOLD));
        let room = row.width.saturating_sub(self.lead() + 1) as usize;
        let words = text::truncate(&self.words, room, glyphs.ellipsis);
        buf.set_string(
            row.x + self.lead(),
            row.y,
            words,
            TextRole::Label.style(look).fg(t(ink)).bg(t(ground)),
        );
    }
}
