//! PageState, from sokkel-design `parts/page-state`.
//!
//! The states a read lands in, kept distinct all the way to the screen: loading, failed, empty,
//! and the content (principle 11). A failure is headed by what the server answered; its
//! diagnostic and the problems it named stand under the heading, with Try again.
//!
//! In cells (the part's `cells` note) the frame is a box whose border stands on the page and whose
//! inside wears the card, padded two columns across and a row down at regular, none down at
//! compact; unframed, it is padded as a panel and not down. The lines are no rows apart; Try again
//! comes after one blank row at either density. The mark is `circle-alert`, or `triangle-alert`
//! in the warning tone, one column and a gap before the heading.
//!
//! The warning tone draws its own frame and mark (`failed+warning`: `warning/8` in a `warning/35`
//! edge, the triangle in its ink); its other pieces are the failed state's.

use crate::button::{Button, ButtonSize, ButtonVariant, IconPlace};
use crate::cells::by_density;
use crate::glyphs::Charset;
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Widget};

/// What the server answered: no answer, or an HTTP status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Unreachable,
    Status(u16),
}

/// The page's own read, or secondary data the page stands without.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateTone {
    #[default]
    Critical,
    Warning,
}

/// The words a page state says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageStateWords {
    pub loading: String,
    pub refused: String,
    pub missing: String,
    pub unreachable: String,
    pub part_refused: String,
    pub retry: String,
}

impl Default for PageStateWords {
    fn default() -> Self {
        PageStateWords {
            loading: "Loading".into(),
            refused: "The server refused this read".into(),
            missing: "Nothing is there".into(),
            unreachable: "The server is not answering".into(),
            part_refused: "Part of this page could not be read".into(),
            retry: "Try again".into(),
        }
    }
}

/// The heading a failure takes, by what the server answered.
pub fn heading(outcome: Option<Outcome>, tone: StateTone, words: &PageStateWords) -> &str {
    match (outcome, tone) {
        (Some(Outcome::Unreachable), _) => &words.unreachable,
        (Some(Outcome::Status(404)), _) => &words.missing,
        (_, StateTone::Warning) => &words.part_refused,
        _ => &words.refused,
    }
}

#[derive(Debug, Clone)]
enum Read {
    Loading,
    Failed {
        outcome: Option<Outcome>,
        diagnostic: Option<String>,
        problems: Vec<String>,
    },
    Empty(String),
}

/// A read that is loading, failed or empty. The content, when it comes, is drawn in its place.
#[derive(Debug, Clone)]
pub struct PageState<'a> {
    look: &'a Look,
    read: Read,
    tone: StateTone,
    framed: bool,
    retry: bool,
    retry_focused: bool,
    busy_ms: u64,
    words: PageStateWords,
    on: &'a str,
}

impl<'a> PageState<'a> {
    fn of(look: &'a Look, read: Read) -> PageState<'a> {
        PageState {
            look,
            read,
            tone: StateTone::Critical,
            framed: true,
            retry: false,
            retry_focused: false,
            busy_ms: 0,
            words: PageStateWords::default(),
            on: "background",
        }
    }

    /// Loading, for `elapsed_ms` so far: the spinner turns.
    pub fn loading(look: &'a Look, elapsed_ms: u64) -> PageState<'a> {
        let mut s = PageState::of(look, Read::Loading);
        s.busy_ms = elapsed_ms;
        s
    }

    /// Failed: headed by what the server answered (`None`: a refusal with no status to go by).
    pub fn failed(look: &'a Look, outcome: Option<Outcome>) -> PageState<'a> {
        PageState::of(
            look,
            Read::Failed {
                outcome,
                diagnostic: None,
                problems: Vec::new(),
            },
        )
    }

    /// Empty, with the one line the app gives.
    pub fn empty(look: &'a Look, message: impl Into<String>) -> PageState<'a> {
        PageState::of(look, Read::Empty(message.into()))
    }

    /// The server's words, or the app's sentence in their place. Backticked parts are code.
    pub fn diagnostic(mut self, words: impl Into<String>) -> Self {
        if let Read::Failed { diagnostic, .. } = &mut self.read {
            *diagnostic = Some(words.into());
        }
        self
    }

    /// Each failure a refusal named.
    pub fn problems(mut self, list: impl IntoIterator<Item = impl Into<String>>) -> Self {
        if let Read::Failed { problems, .. } = &mut self.read {
            problems.extend(list.into_iter().map(Into::into));
        }
        self
    }

    /// Offer Try again; `focused` while it has keyboard focus.
    pub fn retry(mut self, focused: bool) -> Self {
        self.retry = true;
        self.retry_focused = focused;
        self
    }

    pub fn tone(mut self, tone: StateTone) -> Self {
        self.tone = tone;
        self
    }

    /// Drawn in a box of its own (default), or unframed inside a panel.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    pub fn words(mut self, words: PageStateWords) -> Self {
        self.words = words;
        self
    }

    /// The surface an unframed state stands on, a color token (`background`, `card`).
    pub fn on(mut self, surface: &'a str) -> Self {
        self.on = surface;
        self
    }

    /// The heading a failure says; `None` while loading or empty.
    pub fn heading(&self) -> Option<&str> {
        match &self.read {
            Read::Failed { outcome, .. } => Some(heading(*outcome, self.tone, &self.words)),
            _ => None,
        }
    }

    /// Try again, as it is drawn: ask it whether a key pressed it.
    pub fn retry_button(&self) -> Option<Button<'a>> {
        if !self.retry || !matches!(self.read, Read::Failed { .. }) {
            return None;
        }
        let mut b = Button::new(self.look, self.words.retry.clone())
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Sm)
            .focused(self.retry_focused);
        // An icon beside its own word may be left out in ASCII (glyphs.json, rules).
        if self.look.charset() == Charset::Unicode {
            b = b.with_icon(self.look.glyphs().retry, IconPlace::Start);
        }
        Some(b)
    }

    /// Where Try again lands in `area`, to name it as a focus stop.
    pub fn retry_area(&self, area: Rect) -> Option<Rect> {
        let b = self.retry_button()?;
        let inner = self.inner(area);
        let y = inner.y + self.body_rows(inner.width) - 1;
        // Its ground starts where the heading's words do.
        Some(Rect::new(inner.x + 2, y, b.width(), 1).intersection(area))
    }

    /// The rows it takes at `width` columns.
    pub fn height(&self, width: u16) -> u16 {
        let (_, pad_y, edge) = self.insets();
        let inner_w = width.saturating_sub(2 * (self.insets().0 + edge));
        self.body_rows(inner_w) + 2 * (pad_y + edge)
    }

    /// Padding across and down, and the border's width (`cells.sizes`).
    fn insets(&self) -> (u16, u16, u16) {
        if self.framed {
            (2, by_density(self.look, 1, 0), 1)
        } else {
            (by_density(self.look, 2, 1), 0, 0)
        }
    }

    fn inner(&self, area: Rect) -> Rect {
        let (px, py, e) = self.insets();
        Rect {
            x: area.x + px + e,
            y: area.y + py + e,
            width: area.width.saturating_sub(2 * (px + e)),
            height: area.height.saturating_sub(2 * (py + e)),
        }
    }

    fn state_key(&self) -> &'static str {
        match self.read {
            Read::Loading => "loading",
            Read::Failed { .. } => "failed",
            Read::Empty(_) => "empty",
        }
    }

    fn body_rows(&self, width: u16) -> u16 {
        let room = width.saturating_sub(2) as usize;
        match &self.read {
            Read::Loading => 1,
            Read::Empty(m) => text::wrap(m, width as usize).len() as u16,
            Read::Failed {
                outcome,
                diagnostic,
                problems,
            } => {
                let mut n = text::wrap(heading(*outcome, self.tone, &self.words), room).len();
                if let Some(d) = diagnostic {
                    n += text::wrap_code(d, room).len();
                }
                for p in problems {
                    n += text::wrap_code(p, room.saturating_sub(2)).len();
                }
                if self.retry {
                    n += 2;
                }
                n as u16
            }
        }
    }
}

impl Widget for PageState<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &PageState<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let state = self.state_key();
        let page = look.color(self.on);
        // The warning tone's frame and mark are a state of their own; the rest is failed's.
        let toned = if state == "failed" && self.tone == StateTone::Warning {
            "failed+warning"
        } else {
            state
        };
        let frame_key = format!("page-state.base.{toned}.frame");
        let ground = if self.framed {
            look.paint(&frame_key, "ground").unwrap_or(page)
        } else {
            page
        };
        if self.framed {
            let edge = look.paint(&frame_key, "edge").unwrap_or(ground);
            let block = Block::new()
                .borders(Borders::ALL)
                .border_set(look.border_set("radius-surface"))
                .border_style(Style::new().fg(t(edge)).bg(t(page)));
            let inside = block.inner(area);
            block.render(area, buf);
            buf.set_style(inside, Style::new().bg(t(ground)));
        }
        let inner = self.inner(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        let piece = |name: &str, field: &str| look.paint(&format!("page-state.base.{state}.{name}"), field);
        let quiet = piece("frame", "ink").unwrap_or_else(|| look.color("muted-foreground"));
        let mut y = inner.y;
        let put = |buf: &mut Buffer, x: u16, y: u16, s: &str, st: Style| {
            if y < inner.bottom() {
                buf.set_stringn(x, y, s, inner.right().saturating_sub(x) as usize, st);
            }
        };

        match &self.read {
            Read::Loading => {
                // The spinner and the word, centered.
                let w = 2 + text::width(&self.words.loading) as u16;
                let x = inner.x + inner.width.saturating_sub(w) / 2;
                let spin = piece("spinner", "ink").unwrap_or(quiet);
                put(buf, x, y, look.spinner_frame(self.busy_ms), Style::new().fg(t(spin)));
                put(
                    buf,
                    x + 2,
                    y,
                    &self.words.loading,
                    TextRole::Body.style(look).fg(t(quiet)),
                );
            }
            Read::Empty(message) => {
                for line in text::wrap(message, inner.width as usize) {
                    put(buf, inner.x, y, &line, TextRole::Body.style(look).fg(t(quiet)));
                    y += 1;
                }
            }
            Read::Failed {
                outcome,
                diagnostic,
                problems,
            } => {
                let room = inner.width.saturating_sub(2) as usize;
                let (mark, mark_ink) = match self.tone {
                    StateTone::Critical => (
                        glyphs.issue_critical,
                        piece("mark", "ink").unwrap_or_else(|| look.color("destructive")),
                    ),
                    StateTone::Warning => (
                        glyphs.issue_warning,
                        look.paint("page-state.base.failed+warning.mark", "ink")
                            .unwrap_or_else(|| look.color("warning-ink")),
                    ),
                };
                put(buf, inner.x, y, mark, Style::new().fg(t(mark_ink)));
                let head_ink = piece("heading", "ink").unwrap_or_else(|| look.color("foreground"));
                for line in text::wrap(heading(*outcome, self.tone, &self.words), room) {
                    put(buf, inner.x + 2, y, &line, TextRole::Action.style(look).fg(t(head_ink)));
                    y += 1;
                }
                let help_ink = piece("diagnostic", "ink").unwrap_or(quiet);
                let code_ground = piece("code", "ground").unwrap_or(ground);
                let help = TextRole::Help.style(look).fg(t(help_ink));
                let code = help.bg(t(code_ground));
                let line_out = |buf: &mut Buffer, x: u16, y: u16, pieces: &[(String, bool)]| {
                    let mut x = x;
                    for (s, is_code) in pieces {
                        put(buf, x, y, s, if *is_code { code } else { help });
                        x += text::width(s) as u16;
                    }
                };
                if let Some(d) = diagnostic {
                    for line in text::wrap_code(d, room) {
                        line_out(buf, inner.x + 2, y, &line);
                        y += 1;
                    }
                }
                for p in problems {
                    for (i, line) in text::wrap_code(p, room.saturating_sub(2)).into_iter().enumerate() {
                        if i == 0 {
                            put(buf, inner.x + 2, y, glyphs.bullet, help);
                        }
                        line_out(buf, inner.x + 4, y, &line);
                        y += 1;
                    }
                }
                if let Some(b) = self.retry_button() {
                    y += 1;
                    if y < inner.bottom() {
                        // Its ground starts where the heading's words do.
                        let x = inner.x + 2;
                        (&b).render(Rect::new(x, y, b.width().min(inner.right() - x + 1), 1), buf);
                    }
                }
            }
        }
    }
}
