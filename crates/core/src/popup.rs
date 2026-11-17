//! The popup a dropdown menu and a select's list share: a box in every popup's look under (or
//! over) the control that opens it, one row per option, a highlight the keys move.
//!
//! In cells (the parts carry no `cells` note yet; sokkel-tui's reading, in its proposal) the popup
//! is a box whose border is its edge; its 4 px padding is no cell, its rows' 6 px a column. Every
//! row is one row. While the popup holds a check row, a gutter of two columns holds the check, so
//! every label starts at one edge. A separator is a row of the rule glyph across the box; a group's
//! label a row in the help role.

use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Widget};

/// One row of a popup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupRow {
    /// An option: words, and what stands beside them.
    Item(Item),
    /// A group's name, small and muted; not a stop.
    Label(String),
    /// A rule between groups; not a stop.
    Separator,
}

/// An option in a popup.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Item {
    pub label: String,
    /// A glyph before the label, in the look's character set.
    pub icon: Option<String>,
    /// The keys that do the same, at the row's end (a menu's).
    pub shortcut: Option<String>,
    /// A quiet fact at the row's end (a select's).
    pub detail: Option<String>,
    pub destructive: bool,
    pub disabled: bool,
    /// A check row: on or off. `None` for a plain row.
    pub check: Option<bool>,
    /// A radio row: one of a group under a label, the chosen one on. Drawn as a check row; the app
    /// keeps one row of the group on.
    pub radio: bool,
    /// The row opens a submenu: a chevron at its end.
    pub submenu: bool,
}

impl Item {
    pub fn new(label: impl Into<String>) -> Item {
        Item {
            label: label.into(),
            ..Item::default()
        }
    }

    pub fn icon(mut self, glyph: impl Into<String>) -> Self {
        self.icon = Some(glyph.into());
        self
    }

    pub fn shortcut(mut self, keys: impl Into<String>) -> Self {
        self.shortcut = Some(keys.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn check(mut self, on: bool) -> Self {
        self.check = Some(on);
        self
    }

    /// A radio row of a group, `on` while it is the chosen one: its check shows, and choosing it
    /// keeps the menu open, as a check row does.
    pub fn radio(mut self, on: bool) -> Self {
        self.check = Some(on);
        self.radio = true;
        self
    }
}

impl From<Item> for PopupRow {
    fn from(i: Item) -> PopupRow {
        PopupRow::Item(i)
    }
}

/// What a key did to an open popup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupEvent {
    Ignored,
    Moved,
    /// The highlighted row was chosen, by its index among the rows.
    Chosen(usize),
    /// Right on a submenu row.
    OpenSub(usize),
    /// Left in a submenu.
    Back,
    /// Escape.
    Closed,
}

/// The highlight, by its index among the rows; it rests only on an item that is not disabled.
pub fn stops(rows: &[PopupRow]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter(|(_, r)| matches!(r, PopupRow::Item(i) if !i.disabled))
        .map(|(k, _)| k)
        .collect()
}

/// Answer a key while the popup is open, its highlight on `highlight`.
pub fn handle(rows: &[PopupRow], highlight: &mut usize, key: KeyPress, in_sub: bool) -> PopupEvent {
    if key.alt || key.ctrl {
        return PopupEvent::Ignored;
    }
    let stops = stops(rows);
    if stops.is_empty() {
        return if key.is(Key::Escape) {
            PopupEvent::Closed
        } else {
            PopupEvent::Ignored
        };
    }
    let at = stops.iter().position(|&s| s == *highlight).unwrap_or(0);
    let mut go = |to: usize| {
        *highlight = stops[to];
        PopupEvent::Moved
    };
    match key.key {
        Key::Down => go((at + 1).min(stops.len() - 1)),
        Key::Up => go(at.saturating_sub(1)),
        Key::Home => go(0),
        Key::End => go(stops.len() - 1),
        Key::Escape => PopupEvent::Closed,
        Key::Enter | Key::Char(' ') => PopupEvent::Chosen(stops[at]),
        Key::Right => match &rows[stops[at]] {
            PopupRow::Item(i) if i.submenu => PopupEvent::OpenSub(stops[at]),
            _ => PopupEvent::Ignored,
        },
        Key::Left if in_sub => PopupEvent::Back,
        Key::Char(c) => {
            // The next row whose label starts with what was typed.
            let c = c.to_lowercase().next().unwrap_or(c);
            let n = stops.len();
            (1..=n)
                .map(|k| (at + k) % n)
                .find(|&k| {
                    matches!(&rows[stops[k]], PopupRow::Item(i)
                        if i.label.to_lowercase().starts_with(c))
                })
                .map_or(PopupEvent::Ignored, go)
        }
        _ => PopupEvent::Ignored,
    }
}

/// Which part's paints a popup wears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupKind {
    Menu,
    Select,
}

/// A popup's rows drawn in a box.
#[derive(Debug, Clone)]
pub struct Popup<'a> {
    pub(crate) look: &'a Look,
    pub(crate) kind: PopupKind,
    pub(crate) rows: &'a [PopupRow],
    pub(crate) highlight: Option<usize>,
    /// The chosen row, checked at its end (a select's).
    pub(crate) chosen: Option<usize>,
    /// A row whose submenu is open: washed as highlighted.
    pub(crate) sub_open: Option<usize>,
}

impl<'a> Popup<'a> {
    pub fn new(look: &'a Look, kind: PopupKind, rows: &'a [PopupRow]) -> Popup<'a> {
        Popup {
            look,
            kind,
            rows,
            highlight: None,
            chosen: None,
            sub_open: None,
        }
    }

    pub fn highlight(mut self, row: Option<usize>) -> Self {
        self.highlight = row;
        self
    }

    pub fn chosen(mut self, row: Option<usize>) -> Self {
        self.chosen = row;
        self
    }

    fn gutter(&self) -> u16 {
        if self
            .rows
            .iter()
            .any(|r| matches!(r, PopupRow::Item(i) if i.check.is_some()))
        {
            2
        } else {
            0
        }
    }

    /// The columns its widest row asks for, its box included.
    pub fn width(&self) -> u16 {
        let widest = self
            .rows
            .iter()
            .map(|r| match r {
                PopupRow::Item(i) => {
                    text::width(&i.label)
                        + i.icon.as_ref().map_or(0, |g| text::width(g) + 1)
                        + i.shortcut.as_ref().map_or(0, |s| text::width(s) + 2)
                        + i.detail.as_ref().map_or(0, |s| text::width(s) + 1)
                        + if i.submenu { 2 } else { 0 }
                        + if self.kind == PopupKind::Select { 2 } else { 0 }
                }
                PopupRow::Label(l) => text::width(l),
                PopupRow::Separator => 0,
            })
            .max()
            .unwrap_or(0) as u16;
        widest + self.gutter() + 4
    }

    /// The rows it takes, its box included.
    pub fn height(&self) -> u16 {
        self.rows.len() as u16 + 2
    }

    /// Where it opens for a control at `anchor` in `window`: under it, its start level with the
    /// control's, at least `min` columns wide; over it when there is no room under.
    pub fn place(&self, anchor: Rect, window: Rect, min: u16) -> Rect {
        let w = self
            .width()
            .max(min)
            .max(anchor.width)
            .min(window.width.saturating_sub(4).max(8));
        let h = self.height().min(window.height);
        let x = anchor.x.min(window.right().saturating_sub(w)).max(window.x);
        let y = if anchor.bottom() + h <= window.bottom() {
            anchor.bottom()
        } else if anchor.y >= window.y + h {
            anchor.y - h
        } else {
            window.bottom().saturating_sub(h)
        };
        Rect::new(x, y, w, h)
    }

    fn key(&self, piece: &str) -> String {
        match self.kind {
            PopupKind::Menu => format!("dropdown-menu.base.{piece}"),
            PopupKind::Select => format!("select.base.{piece}"),
        }
    }
}

impl Widget for &Popup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 6 || area.height < 3 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let frame = self.key(match self.kind {
            PopupKind::Menu => "rest.popup",
            PopupKind::Select => "rest.list",
        });
        let ground = look.paint(&frame, "ground").unwrap_or_else(|| look.color("popover"));
        let edge = look.paint(&frame, "edge").unwrap_or(ground);
        let ink = look
            .paint(&frame, "ink")
            .unwrap_or_else(|| look.color("popover-foreground"));
        let page = look.color("background");
        Clear.render(area, buf);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(look.border_set("radius-surface"))
            .border_style(Style::new().fg(t(edge)).bg(t(page)));
        let inner = block.inner(area);
        block.render(area, buf);
        buf.set_style(inner, Style::new().bg(t(ground)).fg(t(ink)));
        let gutter = self.gutter();
        let quiet = look.color("muted-foreground");

        for (k, row) in self.rows.iter().enumerate() {
            let y = inner.y + k as u16;
            if y >= inner.bottom() {
                break;
            }
            match row {
                PopupRow::Separator => {
                    let piece = self.key("rest.separator");
                    let rule = look.paint(&piece, "ground").unwrap_or(edge);
                    for x in inner.x..inner.right() {
                        buf[(x, y)].set_symbol(glyphs.rule).set_fg(t(rule));
                    }
                }
                PopupRow::Label(l) => {
                    let piece = self.key(match self.kind {
                        PopupKind::Menu => "rest.label",
                        PopupKind::Select => "rest.groupLabel",
                    });
                    let li = look.paint(&piece, "ink").unwrap_or(quiet);
                    buf.set_stringn(
                        inner.x + 1,
                        y,
                        l,
                        inner.width.saturating_sub(2) as usize,
                        TextRole::Help.style(look).fg(t(li)),
                    );
                }
                PopupRow::Item(item) => self.draw_item(buf, inner, y, k, item, gutter, ground, ink),
            }
        }
    }
}

impl Popup<'_> {
    #[allow(clippy::too_many_arguments)]
    fn draw_item(
        &self,
        buf: &mut Buffer,
        inner: Rect,
        y: u16,
        k: usize,
        item: &Item,
        gutter: u16,
        ground: Rgba,
        ink: Rgba,
    ) {
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let lit = self.highlight == Some(k) || self.sub_open == Some(k);
        let piece = match (self.kind, item.check.is_some(), item.submenu) {
            (PopupKind::Menu, true, _) => "checkItem",
            (PopupKind::Menu, _, true) => "subTrigger",
            (PopupKind::Menu, _, _) => "item",
            (PopupKind::Select, _, _) => "row",
        };
        let state = if item.disabled {
            "disabled"
        } else if self.sub_open == Some(k) && piece == "subTrigger" {
            "subOpen"
        } else if lit {
            "highlighted"
        } else if item.destructive && self.kind == PopupKind::Menu {
            "destructive"
        } else {
            "rest"
        };
        let key = self.key(&format!("{state}.{piece}"));
        let mut row_ground = look.paint(&key, "ground").unwrap_or(ground);
        let mut row_ink = look.paint(&key, "ink").unwrap_or(ink);
        if lit && item.destructive && self.kind == PopupKind::Menu {
            // Highlighted, a destructive row keeps its ink on the highlight's ground.
            row_ink = look.paint(&self.key("destructive.item"), "ink").unwrap_or(row_ink);
        }
        if item.disabled {
            row_ink = look.faded(&key, "ink").unwrap_or(row_ink);
            row_ground = ground;
        }
        let row = Rect::new(inner.x, y, inner.width, 1);
        buf.set_style(row, Style::new().bg(t(row_ground)));
        let mut style = TextRole::Body.style(look).fg(t(row_ink)).bg(t(row_ground));
        if item.disabled && look.depth() == crate::look::Depth::None {
            style = style.add_modifier(Modifier::DIM);
        }
        let mut x = inner.x + 1;
        if gutter > 0 {
            if item.check == Some(true) {
                buf.set_string(x, y, glyphs.check, style);
            }
            x += gutter;
        }
        if let Some(icon) = &item.icon {
            buf.set_string(x, y, icon, style);
            x += text::width(icon) as u16 + 1;
        }
        // The row's end: the chosen check, a submenu's chevron, the keys or the detail.
        let mut end = inner.right().saturating_sub(1);
        if self.kind == PopupKind::Select {
            if self.chosen == Some(k) {
                buf.set_string(end - 1, y, glyphs.check, style);
            }
            end = end.saturating_sub(2);
        }
        if item.submenu {
            buf.set_string(end - 1, y, glyphs.chevron, style);
            end = end.saturating_sub(2);
        }
        let quiet_piece = match self.kind {
            PopupKind::Menu => "shortcut",
            PopupKind::Select => "detail",
        };
        let qkey = self.key(&format!("{}.{quiet_piece}", if lit { "highlighted" } else { "rest" }));
        let qi = look
            .paint(&qkey, "ink")
            .unwrap_or_else(|| look.color("muted-foreground"));
        let end_words = item.shortcut.as_ref().or(item.detail.as_ref());
        if let Some(w) = end_words {
            let ww = text::width(w) as u16;
            end = end.saturating_sub(ww);
            buf.set_string(end, y, w, TextRole::Help.style(look).fg(t(qi)).bg(t(row_ground)));
            end = end.saturating_sub(1);
        }
        let room = end.saturating_sub(x) as usize;
        buf.set_string(x, y, text::truncate(&item.label, room, glyphs.ellipsis), style);
    }
}
