//! Banner: a state that holds until it changes (the server not answering, a read-only instance),
//! in a row across the top of the content, from sokkel-design's `patterns/shell` (its `banner`
//! slot, between the top bar and the content, not scrolling) and sokkel-ui's `Banner`.
//!
//! Its paints are the contract's `banner` (0.31, from proposal c5): each tone's container ground in
//! its opaque form, the mark and the words in its ink. One row:
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
        // The contract's banner container, its tone's ground in its opaque form, and its ink.
        let tone = match self.tone {
            BannerTone::Critical => "critical",
            BannerTone::Warning => "warning",
            BannerTone::Info => "info",
            BannerTone::Good => "good",
        };
        let key = format!("banner.{tone}.rest.container");
        let card = look.color("card");
        (
            look.paint(&key, "ground").unwrap_or(card),
            look.paint(&key, "ink")
                .unwrap_or_else(|| look.color(&format!("{tone}-ink"))),
        )
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
