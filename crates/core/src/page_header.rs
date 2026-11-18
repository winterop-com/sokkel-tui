//! PageHeader, from sokkel-design `parts/page-header`.
//!
//! A page's head: its title (the name, else the code) with a count beside it, then the code, a
//! description and a line of facts under it. A back button can stand before it. The actions sit at
//! the row's end, level with the block's foot, the primary last; they wrap under the title when
//! there is no room, and items fold into More from the last so the row stays one line. The header
//! lays out by its own box. In the frame it draws no trail of its own (principle 8).
//!
//! In cells (the part's `cells` note) the lines 4 and 6 apart are no rows; actions wrapped under
//! the block come after a blank row at regular, none at compact. Buttons stand a column apart, the
//! back button is three columns and one before the block, the meta's facts two apart. The 512 px
//! line is 61 columns.

use crate::button::{Button, ButtonVariant};
use crate::cells::{PX_PER_COL, cols, gap_rows};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

/// The words a page header says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageHeaderWords {
    pub back: String,
    pub trail: String,
    pub more: String,
    pub more_ways: String,
}

impl Default for PageHeaderWords {
    fn default() -> Self {
        PageHeaderWords {
            back: "Back".into(),
            trail: "Where you are".into(),
            more: "More".into(),
            more_ways: "More ways".into(),
        }
    }
}

/// Where the header put its pieces, for focus stops and presses.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeaderLayout {
    pub back: Option<Rect>,
    /// The lead, at the actions' start.
    pub lead: Option<Rect>,
    /// The aside's tile.
    pub aside: Option<Rect>,
    /// The items shown, by their index among the items.
    pub items: Vec<(usize, Rect)>,
    /// More, while any item is folded into it.
    pub more: Option<Rect>,
    pub primary: Option<Rect>,
    /// The actions, by their index.
    pub actions: Vec<(usize, Rect)>,
    /// How many items, from the last, are folded into More.
    pub folded: usize,
    /// Whether the actions wrapped under the title block.
    pub wrapped: bool,
}

/// A page's head.
#[derive(Debug, Clone)]
pub struct PageHeader<'a> {
    look: &'a Look,
    title: Option<String>,
    code: Option<String>,
    count: Option<String>,
    description: Option<String>,
    meta: Vec<String>,
    back: Option<bool>,
    trail: Vec<String>,
    in_shell: bool,
    items: Vec<Button<'a>>,
    primary: Option<Button<'a>>,
    actions: Vec<Button<'a>>,
    more_focused: bool,
    words: PageHeaderWords,
    lead: Option<crate::status::Status>,
    aside: Option<(String, String)>,
    words_only: bool,
    single_line: bool,
}

impl<'a> PageHeader<'a> {
    /// A header titled by a name.
    pub fn new(look: &'a Look, title: impl Into<String>) -> PageHeader<'a> {
        PageHeader {
            look,
            title: Some(title.into()),
            code: None,
            count: None,
            description: None,
            meta: Vec::new(),
            back: None,
            trail: Vec::new(),
            in_shell: true,
            items: Vec::new(),
            primary: None,
            actions: Vec::new(),
            more_focused: false,
            words: PageHeaderWords::default(),
            lead: None,
            aside: None,
            words_only: false,
            single_line: false,
        }
    }

    /// A header with no name: its code stands as the title, on one line.
    pub fn coded(look: &'a Look, code: impl Into<String>) -> PageHeader<'a> {
        let mut h = PageHeader::new(look, "");
        h.title = None;
        h.code = Some(code.into());
        h
    }

    /// The addressable code, under the title.
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn count(mut self, count: impl Into<String>) -> Self {
        self.count = Some(count.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// A line of small facts.
    pub fn meta(mut self, facts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.meta = facts.into_iter().map(Into::into).collect();
        self
    }

    /// A back button before the title; `focused` while it has keyboard focus.
    pub fn back(mut self, focused: bool) -> Self {
        self.back = Some(focused);
        self
    }

    /// The places above the page. Drawn only outside a shell: in the frame the top bar says them.
    pub fn trail(mut self, crumbs: impl IntoIterator<Item = impl Into<String>>, in_shell: bool) -> Self {
        self.trail = crumbs.into_iter().map(Into::into).collect();
        self.in_shell = in_shell;
        self
    }

    /// Buttons that fold into More, from the last, as the header narrows. Drawn as outline.
    pub fn items(mut self, items: impl IntoIterator<Item = Button<'a>>) -> Self {
        self.items = items.into_iter().map(|b| b.variant(ButtonVariant::Outline)).collect();
        self
    }

    /// The primary action among the items: filled, never folding.
    pub fn primary(mut self, primary: Button<'a>) -> Self {
        self.primary = Some(primary.variant(ButtonVariant::Default));
        self
    }

    /// Actions laid out as given, the primary last.
    pub fn actions(mut self, actions: impl IntoIterator<Item = Button<'a>>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    pub fn more_focused(mut self, focused: bool) -> Self {
        self.more_focused = focused;
        self
    }

    /// A part at the actions' start that is not a verb: a run's state, its dot and word in the
    /// state's colors. It never folds into More.
    pub fn lead(mut self, state: impl Into<String>) -> Self {
        self.lead = Some(crate::status::Status::of(state));
        self
    }

    /// The lead as a [`crate::Status`]: a health state, or the app's own word in a tone.
    pub fn lead_status(mut self, status: crate::status::Status) -> Self {
        self.lead = Some(status);
        self
    }

    /// A picture before the title block: in cells, a tile with `letter` on the `color` token's
    /// ground, centered on the block's lines; after the back button where there is one.
    pub fn aside(mut self, letter: impl Into<String>, color: impl Into<String>) -> Self {
        self.aside = Some((letter.into(), color.into()));
        self
    }

    /// The items and the primary drawn by their words alone, their icons kept for More.
    pub fn words_only(mut self, on: bool) -> Self {
        self.words_only = on;
        self
    }

    /// The title kept to one line, cut short, rather than wrapping.
    pub fn single_line(mut self, on: bool) -> Self {
        self.single_line = on;
        self
    }

    pub fn words(mut self, words: PageHeaderWords) -> Self {
        self.words = words;
        self
    }

    fn more_button(&self) -> Button<'a> {
        Button::icon(self.look, self.look.glyphs().more, self.words.more.clone())
            .variant(ButtonVariant::Outline)
            .focused(self.more_focused)
    }

    fn back_button(&self) -> Option<Button<'a>> {
        self.back
            .map(|f| Button::icon(self.look, self.look.glyphs().back, self.words.back.clone()).focused(f))
    }

    /// Columns before the block's lines: the back button and 8 px after it, the aside's tile
    /// and 12 px after it.
    fn inset(&self) -> u16 {
        self.back_button().map_or(0, |b| b.width() + 1) + self.aside.as_ref().map_or(0, |_| 3 + cols(12.0))
    }

    /// The lead's width: its dot, a space, its word.
    fn lead_width(&self) -> u16 {
        self.lead.as_ref().map_or(0, |s| 2 + text::width(s.words()) as u16)
    }

    /// An item or the primary as drawn: by its words alone under `wordsOnly`.
    fn drawn(&self, b: &Button<'a>) -> Button<'a> {
        if self.words_only {
            b.clone().words_only()
        } else {
            b.clone()
        }
    }

    fn title_text(&self) -> &str {
        self.title.as_deref().or(self.code.as_deref()).unwrap_or("")
    }

    fn title_width(&self) -> usize {
        text::width(self.title_text()) + self.count.as_ref().map_or(0, |c| 1 + text::width(c))
    }

    fn fills(&self) -> bool {
        !self.items.is_empty() || self.primary.is_some()
    }

    /// The actions' width with `shown` items and More if any are folded, a column apart.
    fn actions_width(&self, shown: usize) -> u16 {
        let more = (shown < self.items.len()).then(|| self.more_button());
        let mut widths: Vec<u16> = self
            .items
            .iter()
            .take(shown)
            .chain(more.as_ref())
            .chain(self.primary.as_ref())
            .map(|b| self.drawn(b).width())
            .chain(self.actions.iter().map(Button::width))
            .collect();
        if self.lead.is_some() {
            widths.insert(0, self.lead_width());
        }
        if widths.is_empty() {
            0
        } else {
            widths.iter().sum::<u16>() + widths.len() as u16 - 1
        }
    }

    /// How many items show in `room` columns: all, or as many as fit beside More.
    fn shown_in(&self, room: u16) -> usize {
        (0..=self.items.len())
            .rev()
            .find(|&n| self.actions_width(n) <= room)
            .unwrap_or(0)
    }

    /// The block's width and whether the actions stand beside it, at `width` columns.
    fn split(&self, width: u16) -> (u16, bool, usize) {
        let width = width.saturating_sub(self.inset());
        let gap = cols(16.0);
        let wide = width as f64 * PX_PER_COL >= 512.0;
        // The block's own width: its widest line, the description at most a reading measure.
        let title_w = [
            self.title_width(),
            self.code.as_deref().map_or(0, text::width),
            self.description.as_deref().map_or(0, |d| text::width(d).min(65)),
            text::width(&self.meta.join("  ")),
        ]
        .into_iter()
        .max()
        .unwrap_or(0) as u16;
        let n_actions = self.items.len()
            + usize::from(self.primary.is_some())
            + self.actions.len()
            + usize::from(self.lead.is_some());
        if n_actions == 0 {
            return (width, false, 0);
        }
        if self.fills() {
            if wide {
                let title = title_w.min(width * 6 / 10);
                let room = width.saturating_sub(title + gap).max(cols(128.0));
                let shown = self.shown_in(room);
                return (width.saturating_sub(self.actions_width(shown) + gap), true, shown);
            }
            return (width, false, self.shown_in(width));
        }
        let needed = self.actions_width(0);
        let ask = if self.description.is_some() || !self.meta.is_empty() {
            cols(256.0)
        } else if wide {
            cols(128.0)
        } else {
            title_w
        };
        let room = width.saturating_sub(needed + gap);
        if room >= ask.min(width) && room > 0 {
            (room, true, 0)
        } else {
            (width, false, 0)
        }
    }

    fn block_lines(&self, width: u16) -> usize {
        let w = width as usize;
        let mut n = usize::from(!self.in_shell && !self.trail.is_empty());
        n += if self.title.is_none() || self.single_line {
            1
        } else {
            text::wrap(self.title_text(), w.saturating_sub(self.count_room())).len()
        };
        n += usize::from(self.title.is_some() && self.code.is_some());
        n += self.description.as_ref().map_or(0, |d| text::wrap(d, w.min(65)).len());
        n += usize::from(!self.meta.is_empty());
        n
    }

    fn count_room(&self) -> usize {
        self.count.as_ref().map_or(0, |c| text::width(c) + 1)
    }

    /// The rows it takes at `width` columns.
    pub fn height(&self, width: u16) -> u16 {
        let (block_w, beside, _) = self.split(width);
        let lines = self.block_lines(block_w) as u16;
        let none = self.items.is_empty() && self.primary.is_none() && self.actions.is_empty() && self.lead.is_none();
        if beside || none {
            lines
        } else {
            lines + gap_rows(self.look, 12.0) + 1
        }
    }

    /// Where the pieces land in `area`.
    pub fn layout(&self, area: Rect) -> HeaderLayout {
        let (block_w, beside, shown) = self.split(area.width);
        let lines = self.block_lines(block_w) as u16;
        let mut out = HeaderLayout {
            folded: self.items.len() - shown,
            wrapped: !beside,
            ..HeaderLayout::default()
        };
        let lead = self.inset();
        if let Some(b) = self.back_button() {
            let y = area.y + u16::from(!self.in_shell && !self.trail.is_empty());
            out.back = Some(Rect::new(area.x, y, b.width(), 1));
        }
        if self.aside.is_some() {
            let x = area.x + self.back_button().map_or(0, |b| b.width() + 1);
            out.aside = Some(Rect::new(x, area.y + lines.saturating_sub(1) / 2, 3, 1));
        }
        let total = self.actions_width(shown);
        if total == 0 {
            return out;
        }
        let (mut x, y) = if beside {
            (area.right().saturating_sub(total), area.y + lines.saturating_sub(1))
        } else {
            (area.x + lead, area.y + lines + gap_rows(self.look, 12.0))
        };
        if self.lead.is_some() {
            out.lead = Some(Rect::new(x, y, self.lead_width(), 1));
            x += self.lead_width() + 1;
        }
        for (i, b) in self.items.iter().take(shown).enumerate() {
            let w = self.drawn(b).width();
            out.items.push((i, Rect::new(x, y, w, 1)));
            x += w + 1;
        }
        if shown < self.items.len() {
            let w = self.more_button().width();
            out.more = Some(Rect::new(x, y, w, 1));
            x += w + 1;
        }
        if let Some(p) = &self.primary {
            let w = self.drawn(p).width();
            out.primary = Some(Rect::new(x, y, w, 1));
            x += w + 1;
        }
        for (i, b) in self.actions.iter().enumerate() {
            out.actions.push((i, Rect::new(x, y, b.width(), 1)));
            x += b.width() + 1;
        }
        out
    }
}

impl Widget for PageHeader<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &PageHeader<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 4 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let ink = |piece: &str| {
            look.paint(&format!("page-header.base.rest.{piece}"), "ink")
                .unwrap_or_else(|| look.color("muted-foreground"))
        };
        let layout = self.layout(area);
        let (block_w, _, _) = self.split(area.width);
        let x = area.x + self.inset();
        let mut y = area.y;
        let line = |buf: &mut Buffer, l: Line, y: &mut u16| {
            if *y < area.bottom() {
                buf.set_line(x, *y, &l, block_w);
                *y += 1;
            }
        };

        if !self.in_shell && !self.trail.is_empty() {
            let crumb = TextRole::Help.style(look).fg(t(ink("crumb")));
            let chevron = Style::new().fg(t(ink("chevron")));
            let mut spans = Vec::new();
            for c in &self.trail {
                spans.push(Span::styled(c.clone(), crumb));
                spans.push(Span::styled(format!(" {} ", glyphs.chevron), chevron));
            }
            line(buf, Line::from(spans), &mut y);
        }

        let count = self
            .count
            .as_ref()
            .map(|c| Span::styled(c.clone(), TextRole::Help.style(look).fg(t(ink("count")))));
        if let Some(title) = &self.title {
            let style = TextRole::PageTitle.style(look).fg(t(ink("title")));
            let room = (block_w as usize).saturating_sub(self.count_room());
            let wrapped = if self.single_line {
                vec![text::truncate(title, room, glyphs.ellipsis)]
            } else {
                text::wrap(title, room)
            };
            let last = wrapped.len() - 1;
            for (i, l) in wrapped.into_iter().enumerate() {
                let mut spans = vec![Span::styled(l, style)];
                if i == last
                    && let Some(c) = &count
                {
                    spans.push(Span::raw(" "));
                    spans.push(c.clone());
                }
                line(buf, Line::from(spans), &mut y);
            }
            if let Some(code) = &self.code {
                let s = text::truncate(code, block_w as usize, glyphs.ellipsis);
                line(
                    buf,
                    Line::from(Span::styled(s, TextRole::Help.style(look).fg(t(ink("code"))))),
                    &mut y,
                );
            }
        } else {
            // A code standing as the title: one line, cut short.
            let style = TextRole::PageTitle.style(look).fg(t(look
                .paint("page-header.base.codeTitle.title", "ink")
                .unwrap_or_else(|| ink("title"))));
            let room = (block_w as usize).saturating_sub(self.count_room());
            let mut spans = vec![Span::styled(
                text::truncate(self.title_text(), room, glyphs.ellipsis),
                style,
            )];
            if let Some(c) = count {
                spans.push(Span::raw(" "));
                spans.push(c);
            }
            line(buf, Line::from(spans), &mut y);
        }
        if let Some(d) = &self.description {
            let style = TextRole::Body.style(look).fg(t(ink("description")));
            for l in text::wrap(d, (block_w as usize).min(65)) {
                line(buf, Line::from(Span::styled(l, style)), &mut y);
            }
        }
        if !self.meta.is_empty() {
            let style = TextRole::Help.style(look).fg(t(ink("meta")));
            let facts = self.meta.join("  ");
            let s = text::truncate(&facts, block_w as usize, glyphs.ellipsis);
            line(buf, Line::from(Span::styled(s, style)), &mut y);
        }

        if let (Some(b), Some(r)) = (self.back_button(), layout.back) {
            b.render(r, buf);
        }
        if let (Some((letter, color)), Some(r)) = (&self.aside, layout.aside) {
            let tile = Style::new().bg(look.token(color)).fg(look.token("on-fill"));
            buf.set_string(r.x, r.y, format!(" {letter} "), tile);
        }
        if let (Some(status), Some(r)) = (&self.lead, layout.lead) {
            let (dot, word) = status.colors(look);
            buf.set_string(r.x, r.y, glyphs.status_dot, Style::new().fg(t(dot)));
            buf.set_string(r.x + 2, r.y, status.words(), TextRole::Body.style(look).fg(t(word)));
        }
        for (i, r) in &layout.items {
            self.drawn(&self.items[*i]).render(*r, buf);
        }
        if let Some(r) = layout.more {
            self.more_button().render(r, buf);
        }
        if let (Some(p), Some(r)) = (&self.primary, layout.primary) {
            self.drawn(p).render(r, buf);
        }
        for (i, r) in &layout.actions {
            (&self.actions[*i]).render(*r, buf);
        }
    }
}
