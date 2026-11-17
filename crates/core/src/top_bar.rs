//! TopBar, from sokkel-design `parts/top-bar`.
//!
//! The thin bar over the content: the menu button below md, the search field, the trail of places
//! above the page, the command palette's button with its keys, and the app's actions. The trail
//! lives here, never in the page header (principle 8).
//!
//! In cells (the part's `cells` note) the bar is one row, a column of padding at each end, its
//! groups two columns apart. The menu button is three columns, below md only. The search field is
//! 27 columns, 30 from lg. Crumbs have the chevron between them with a column each side; below md
//! the last crumb stands alone, after `chevron-left` when there is a way back. The palette's button
//! carries its keys; below md it is the search icon alone. The rule under the bar is a row at
//! regular and none at compact, as the status line's over the foot.

use crate::button::Button;
use crate::cells::{by_density, cols};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// The words a top bar says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopBarWords {
    pub search: String,
    pub palette: String,
    pub trail: String,
}

impl Default for TopBarWords {
    fn default() -> Self {
        TopBarWords {
            search: "Search".into(),
            palette: "Open the command palette".into(),
            trail: "Where you are".into(),
        }
    }
}

/// Which of the bar's controls has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TopBarFocus {
    #[default]
    None,
    Menu,
    Search,
    /// A crumb that is a link, by its place in the trail.
    Crumb(usize),
    Palette,
    /// An action, by its place.
    Action(usize),
}

/// Where the bar put its controls, for focus stops and presses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TopBarLayout {
    pub menu: Option<Rect>,
    pub search: Option<Rect>,
    pub crumbs: Vec<Rect>,
    pub palette: Option<Rect>,
    pub actions: Vec<Rect>,
}

/// The bar over the content.
#[derive(Debug, Clone)]
pub struct TopBar<'a> {
    look: &'a Look,
    trail: Vec<String>,
    narrow: bool,
    wide: bool,
    search: Option<String>,
    palette: bool,
    shortcut: String,
    actions: Vec<Button<'a>>,
    focus: TopBarFocus,
    words: TopBarWords,
}

impl<'a> TopBar<'a> {
    pub fn new(look: &'a Look) -> TopBar<'a> {
        TopBar {
            look,
            trail: Vec::new(),
            narrow: false,
            wide: false,
            search: None,
            palette: true,
            shortcut: "Ctrl K".into(),
            actions: Vec::new(),
            focus: TopBarFocus::None,
            words: TopBarWords::default(),
        }
    }

    /// The places above the page, the page itself last.
    pub fn trail(mut self, crumbs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.trail = crumbs.into_iter().map(Into::into).collect();
        self
    }

    /// The window's width decides the bar's form: below md (`narrow`), from lg (`wide`).
    pub fn window(mut self, width: crate::cells::Width) -> Self {
        self.narrow = width == crate::cells::Width::Narrow;
        self.wide = width == crate::cells::Width::Wide;
        self
    }

    /// The search field, holding `query` (empty: its placeholder).
    pub fn search(mut self, query: impl Into<String>) -> Self {
        self.search = Some(query.into());
        self
    }

    /// The palette's button, and its keys as the app writes them (`Ctrl K`).
    pub fn palette(mut self, on: bool, shortcut: impl Into<String>) -> Self {
        self.palette = on;
        self.shortcut = shortcut.into();
        self
    }

    pub fn actions(mut self, actions: impl IntoIterator<Item = Button<'a>>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    pub fn focus(mut self, focus: TopBarFocus) -> Self {
        self.focus = focus;
        self
    }

    pub fn words(mut self, words: TopBarWords) -> Self {
        self.words = words;
        self
    }

    /// The rows it takes: the bar, and its rule at regular.
    pub fn height(look: &Look) -> u16 {
        1 + by_density(look, 1, 0)
    }

    fn search_w(&self) -> u16 {
        if self.wide { cols(256.0) } else { cols(224.0) }
    }

    fn palette_w(&self) -> u16 {
        if self.narrow {
            3
        } else {
            (text::width(&self.words.palette) + text::width(&self.shortcut) + 4) as u16
        }
    }

    /// Where the controls land in `area`.
    pub fn layout(&self, area: Rect) -> TopBarLayout {
        let mut out = TopBarLayout::default();
        let y = area.y;
        let mut x = area.x + 1;
        if self.narrow {
            out.menu = Some(Rect::new(x, y, 3, 1));
            x += 3 + 2;
        } else if self.search.is_some() {
            let w = self.search_w().min(area.width / 3);
            out.search = Some(Rect::new(x, y, w, 1));
            x += w + 2;
        }
        // The trail, or below md its last crumb alone.
        let shown: Vec<(usize, &String)> = if self.narrow {
            self.trail
                .iter()
                .enumerate()
                .skip(self.trail.len().saturating_sub(1))
                .collect()
        } else {
            self.trail.iter().enumerate().collect()
        };
        if self.narrow && self.trail.len() > 1 {
            x += 2;
        }
        for (k, (_, c)) in shown.iter().enumerate() {
            let w = text::width(c) as u16;
            out.crumbs.push(Rect::new(x, y, w, 1));
            x += w;
            if k + 1 < shown.len() {
                x += 3;
            }
        }
        // At the end: the palette's button, then the actions a column apart.
        let mut end = area.right().saturating_sub(1);
        let mut acts: Vec<Rect> = Vec::new();
        for b in self.actions.iter().rev() {
            end = end.saturating_sub(b.width());
            acts.push(Rect::new(end, y, b.width(), 1));
            end = end.saturating_sub(1);
        }
        acts.reverse();
        out.actions = acts;
        if self.palette {
            let w = self.palette_w();
            out.palette = Some(Rect::new(end.saturating_sub(w), y, w, 1));
        }
        out
    }
}

impl Widget for TopBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &TopBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let bar = "top-bar.base.rest.bar";
        let ground = look.paint(bar, "ground").unwrap_or_else(|| look.color("sidebar"));
        buf.set_style(area, Style::new().bg(t(ground)));
        if area.height >= 2 && by_density(look, 1, 0) == 1 {
            let rule = look.paint(bar, "rule").unwrap_or(ground);
            for x in area.x..area.right() {
                buf[(x, area.y + 1)].set_symbol(glyphs.rule).set_fg(t(rule));
            }
        }
        let row = Rect { height: 1, ..area };
        let layout = self.layout(row);
        let ink = |key: &str| look.paint(&format!("top-bar.base.{key}"), "ink");
        let ring = look.color("ring");
        let ring_at = |buf: &mut Buffer, r: Rect| {
            if r.x > area.x {
                buf[(r.x - 1, r.y)].set_symbol(glyphs.ring_start).set_fg(t(ring));
            }
            if r.right() < area.right() {
                buf[(r.right(), r.y)].set_symbol(glyphs.ring_end).set_fg(t(ring));
            }
        };

        if let Some(r) = layout.menu {
            let st = Style::new().fg(t(ink("rest.menu").unwrap_or(ground)));
            buf.set_string(r.x + 1, r.y, glyphs.menu, st);
            if self.focus == TopBarFocus::Menu {
                ring_at(buf, r);
            }
        }
        if let (Some(r), Some(query)) = (layout.search, &self.search) {
            // The search field: a control on the field's ground, its edge in the pad cells.
            let state = if self.focus == TopBarFocus::Search {
                "focus"
            } else {
                "rest"
            };
            let key = format!("text-field.base.{state}.control");
            let fg = look.paint(&key, "ground").unwrap_or(ground);
            let edge = look.paint(&key, "edge").unwrap_or(fg);
            buf.set_style(r, Style::new().bg(t(fg)));
            buf[(r.x, r.y)].set_symbol(glyphs.edge_start).set_fg(t(edge));
            buf[(r.right() - 1, r.y)].set_symbol(glyphs.edge_end).set_fg(t(edge));
            let quiet = look.paint(&key, "placeholder").unwrap_or(edge);
            buf.set_string(r.x + 1, r.y, glyphs.search, Style::new().fg(t(quiet)));
            let (words, wink) = if query.is_empty() {
                (self.words.search.clone(), quiet)
            } else {
                (query.clone(), look.paint(&key, "ink").unwrap_or(quiet))
            };
            let room = r.width.saturating_sub(4) as usize;
            buf.set_string(
                r.x + 3,
                r.y,
                text::truncate(&words, room, glyphs.ellipsis),
                TextRole::Body.style(look).fg(t(wink)),
            );
        }
        // The trail: crumbs, the chevron between; below md the last after chevron-left.
        if self.narrow && self.trail.len() > 1 {
            if let Some(first) = layout.crumbs.first() {
                let st = Style::new().fg(t(ink("rest.backChevron").unwrap_or(ground)));
                buf.set_string(first.x - 2, first.y, glyphs.back, st);
            }
        }
        let first = if self.narrow {
            self.trail.len().saturating_sub(1)
        } else {
            0
        };
        let chevron = Style::new().fg(t(ink("rest.chevron").unwrap_or(ground)));
        for (k, r) in layout.crumbs.iter().enumerate() {
            let i = first + k;
            let current = i + 1 == self.trail.len();
            let key = if current { "current.crumb" } else { "rest.crumb" };
            let st = TextRole::Body.style(look).fg(t(ink(key).unwrap_or(ground)));
            buf.set_string(r.x, r.y, &self.trail[i], st);
            if k + 1 < layout.crumbs.len() {
                buf.set_string(r.right() + 1, r.y, glyphs.chevron, chevron);
            }
            if self.focus == TopBarFocus::Crumb(i) {
                ring_at(buf, *r);
            }
        }
        if let Some(r) = layout.palette {
            let state = if self.narrow { "phone" } else { "rest" };
            let key = format!("top-bar.base.{state}.palette");
            let pg = look.paint(&key, "ground").unwrap_or(ground);
            let pink = look.paint(&key, "ink").unwrap_or(ground);
            if self.narrow {
                buf.set_string(r.x + 1, r.y, glyphs.search, Style::new().fg(t(pink)));
            } else {
                let edge = look.paint(&key, "edge").unwrap_or(pg);
                buf.set_style(r, Style::new().bg(t(pg)));
                buf[(r.x, r.y)].set_symbol(glyphs.edge_start).set_fg(t(edge));
                buf[(r.right() - 1, r.y)].set_symbol(glyphs.edge_end).set_fg(t(edge));
                buf.set_string(
                    r.x + 1,
                    r.y,
                    &self.words.palette,
                    TextRole::Help.style(look).fg(t(pink)),
                );
                let kx = r.right() - 2 - text::width(&self.shortcut) as u16;
                let kg = look.paint("top-bar.base.rest.keys", "ground").unwrap_or(pg);
                let ki = look.paint("top-bar.base.rest.keys", "ink").unwrap_or(pink);
                buf.set_string(kx, r.y, &self.shortcut, Style::new().fg(t(ki)).bg(t(kg)));
            }
            if self.focus == TopBarFocus::Palette {
                ring_at(buf, r);
            }
        }
        for (i, (b, r)) in self.actions.iter().zip(layout.actions.iter()).enumerate() {
            b.clone().focused(self.focus == TopBarFocus::Action(i)).render(*r, buf);
        }
    }
}
