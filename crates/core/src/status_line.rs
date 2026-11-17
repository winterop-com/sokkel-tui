//! StatusLine, from sokkel-design `parts/status-line`.
//!
//! The shell's foot: one row with a note at its start (a live dot and words, in a tone) and facts at
//! its end. While a control has keyboard focus, its tooltip's words, or a disabled control's reason,
//! take the note's place (principle 23): a terminal has no floating box for them.
//!
//! In cells (the part's `cells` note) the strip is one row, a column of padding at each end, its
//! items two columns apart; the facts take at most 70 percent of the width from the end and the note
//! the rest, cut short. The rule over the strip is a row of the rule glyph at regular; at compact the
//! strip's ground alone sets it apart.

use crate::button::Tip;
use crate::cells::by_density;
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// The note's tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoteTone {
    #[default]
    Plain,
    Live,
    Warn,
    Error,
}

/// One fact: an optional label and its value, the value in the code face's ink when it is a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub label: Option<String>,
    pub value: String,
    pub code: bool,
}

impl Fact {
    pub fn new(value: impl Into<String>) -> Fact {
        Fact {
            label: None,
            value: value.into(),
            code: false,
        }
    }

    pub fn labeled(label: impl Into<String>, value: impl Into<String>) -> Fact {
        Fact {
            label: Some(label.into()),
            ..Fact::new(value)
        }
    }

    /// The value is a code (a host, a key): drawn in the code item's ink.
    pub fn code(mut self) -> Fact {
        self.code = true;
        self
    }

    fn width(&self) -> usize {
        self.label.as_ref().map_or(0, |l| text::width(l) + 1) + text::width(&self.value)
    }
}

/// The shell's foot.
#[derive(Debug, Clone)]
pub struct StatusLine<'a> {
    look: &'a Look,
    note: Option<String>,
    tone: NoteTone,
    live: bool,
    facts: Vec<Fact>,
    tip: Option<Tip>,
}

impl<'a> StatusLine<'a> {
    pub fn new(look: &'a Look) -> StatusLine<'a> {
        StatusLine {
            look,
            note: None,
            tone: NoteTone::Plain,
            live: false,
            facts: Vec::new(),
            tip: None,
        }
    }

    /// The note at the strip's start, in a tone.
    pub fn note(mut self, words: impl Into<String>, tone: NoteTone) -> Self {
        self.note = Some(words.into());
        self.tone = tone;
        self
    }

    /// The live dot before the note.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    pub fn facts(mut self, facts: impl IntoIterator<Item = Fact>) -> Self {
        self.facts = facts.into_iter().collect();
        self
    }

    /// A focused control's tip: its words, and its keys after two columns, in the note's place.
    pub fn tip(mut self, tip: Option<Tip>) -> Self {
        self.tip = tip;
        self
    }

    /// The rows it takes: the strip, and its rule at regular.
    pub fn height(look: &Look) -> u16 {
        1 + by_density(look, 1, 0)
    }
}

impl Widget for StatusLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &StatusLine<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 4 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let strip = "status-line.base.rest.strip";
        let ground = look
            .paint(strip, "ground")
            .unwrap_or_else(|| look.color("page-foot-ground"));
        buf.set_style(area, Style::new().bg(t(ground)));
        let mut y = area.y;
        if area.height >= 2 && by_density(look, 1, 0) == 1 {
            let rule = look
                .paint(strip, "rule")
                .unwrap_or_else(|| look.color("page-foot-line"));
            for x in area.x..area.right() {
                buf[(x, y)].set_symbol(glyphs.rule).set_fg(t(rule));
            }
            y += 1;
        }
        let ink = |key: &str| {
            look.paint(&format!("status-line.base.{key}"), "ink")
                .unwrap_or_else(|| look.color("page-foot-ink"))
        };
        let help = TextRole::Help.style(look).bg(t(ground));

        // The facts from the end, at most 70 percent of the width, two columns apart.
        let inner_w = area.width.saturating_sub(2);
        let room = (inner_w as usize) * 7 / 10;
        let mut shown = Vec::new();
        let mut used = 0;
        for f in self.facts.iter().rev() {
            let w = f.width() + if shown.is_empty() { 0 } else { 2 };
            if used + w > room {
                break;
            }
            used += w;
            shown.push(f);
        }
        shown.reverse();
        let mut x = area.right() - 1 - used as u16;
        for (i, f) in shown.iter().enumerate() {
            if i > 0 {
                x += 2;
            }
            if let Some(l) = &f.label {
                buf.set_string(x, y, l, help.fg(t(ink("rest.itemLabel"))));
                x += text::width(l) as u16 + 1;
            }
            let value_ink = if f.code { ink("code.item") } else { ink("rest.item") };
            buf.set_string(x, y, &f.value, help.fg(t(value_ink)));
            x += text::width(&f.value) as u16;
        }

        // The note in the rest, or a focused control's tip in its place.
        let note_room = (inner_w as usize).saturating_sub(used + if used > 0 { 2 } else { 0 });
        let mut nx = area.x + 1;
        if let Some(tip) = &self.tip {
            let words = text::truncate(&tip.words, note_room, glyphs.ellipsis);
            buf.set_string(nx, y, &words, help.fg(t(ink("rest.note"))));
            nx += text::width(&words) as u16;
            if let Some(keys) = &tip.shortcut
                && text::width(&words) + 2 + text::width(keys) <= note_room
            {
                buf.set_string(nx + 2, y, keys, help.fg(t(ink("code.item"))));
            }
            return;
        }
        let tone = match self.tone {
            NoteTone::Plain => "rest",
            NoteTone::Live => "toneLive",
            NoteTone::Warn => "toneWarn",
            NoteTone::Error => "toneError",
        };
        let note_ink = ink(&format!("{tone}.note"));
        let mut room = note_room;
        if self.live {
            let dot = look
                .paint("status-line.base.live.liveDot", "ground")
                .unwrap_or(note_ink);
            let dot = if self.tone == NoteTone::Plain { dot } else { note_ink };
            buf.set_string(nx, y, glyphs.status_dot, help.fg(t(dot)));
            nx += 2;
            room = room.saturating_sub(2);
        }
        if let Some(n) = &self.note {
            let words = text::truncate(n, room, glyphs.ellipsis);
            buf.set_string(nx, y, words, help.fg(t(note_ink)));
        }
    }
}
