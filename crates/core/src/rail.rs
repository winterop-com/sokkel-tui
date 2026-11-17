//! Rail, from sokkel-design `parts/rail`.
//!
//! The app's places down the window's start: a head with the mark and the name, groups of entries,
//! and a foot cell for the account. Collapsed, it is its icons alone, each named in the status line
//! while it has focus. Below md it is a drawer over the content, drawn expanded.
//!
//! In cells (the part's `cells` note) the rail is 29 columns (24 to 43), collapsed 7; entries are
//! one row at both densities, a column of padding at each end. The active entry's bar is
//! `focus-mark` in its edge color at its first column, then the icon, a gap, the label, and a count
//! or a dot at the end. A group's title is upper case in `faint`, after a blank row at regular. The
//! rail's edge is a column of the track glyph in `sidebar-border`. Keyboard focus on an entry is
//! `focus-mark` in `ring` at its start (open-questions T4, as decided for rows).
//!
//! Built so far: groups, entries with icon, label, count and unread dot, the active entry, collapsed
//! and drawer forms, the head and the foot cell. Not yet: parents and their flyouts, descriptions,
//! the strip variant, a body in place of the groups, resizing, reordering.

use crate::cells::{by_density, cols};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Clear, Widget};

/// One place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailEntry {
    pub label: String,
    /// The glyph the app draws for it, in the look's character set.
    pub icon: Option<String>,
    pub count: Option<String>,
    pub unread: bool,
    pub active: bool,
    /// A line under the label, while the rail is expanded.
    pub description: Option<String>,
    /// 0 for a place of its own; 1 for an entry under the parent before it.
    pub depth: u8,
    /// A parent: `Some(open)`, its children shown under it while open.
    pub parent: Option<bool>,
}

impl RailEntry {
    pub fn new(label: impl Into<String>) -> RailEntry {
        RailEntry {
            label: label.into(),
            icon: None,
            count: None,
            unread: false,
            active: false,
            description: None,
            depth: 0,
            parent: None,
        }
    }

    /// A line under the label, shown while the rail is expanded.
    pub fn description(mut self, words: impl Into<String>) -> Self {
        self.description = Some(words.into());
        self
    }

    /// A parent of the entries after it that are [`RailEntry::child`]: its chevron folds them,
    /// `open` showing them. A press on it comes to the app as its entry; the app folds it.
    pub fn parent(mut self, open: bool) -> Self {
        self.parent = Some(open);
        self
    }

    /// An entry under the parent before it, lined up under the parent's label.
    pub fn child(mut self) -> Self {
        self.depth = 1;
        self
    }

    pub fn icon(mut self, glyph: impl Into<String>) -> Self {
        self.icon = Some(glyph.into());
        self
    }

    pub fn count(mut self, count: impl Into<String>) -> Self {
        self.count = Some(count.into());
        self
    }

    pub fn unread(mut self) -> Self {
        self.unread = true;
        self
    }

    pub fn active(mut self) -> Self {
        self.active = true;
        self
    }
}

/// A group of places, titled or not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailGroup {
    pub title: Option<String>,
    pub entries: Vec<RailEntry>,
}

impl RailGroup {
    pub fn new(title: Option<&str>, entries: impl IntoIterator<Item = RailEntry>) -> RailGroup {
        RailGroup {
            title: title.map(String::from),
            entries: entries.into_iter().collect(),
        }
    }
}

/// The account in the foot cell: its tile's letter, its tile's color token, and its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailFoot {
    pub letter: String,
    pub color: String,
    pub name: String,
}

/// The words a rail says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailWords {
    pub collapse: String,
    pub expand: String,
    pub resize: String,
    pub unread: String,
    pub places: String,
}

impl Default for RailWords {
    fn default() -> Self {
        RailWords {
            collapse: "Collapse the navigation".into(),
            expand: "Expand the navigation".into(),
            resize: "Resize the navigation".into(),
            unread: "Unread".into(),
            places: "All places".into(),
        }
    }
}

/// Where the rail put its entries, by group and place, for focus stops and presses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RailLayout {
    pub entries: Vec<((usize, usize), Rect)>,
    /// The control that collapses the rail, or, collapsed, the head that expands it.
    pub collapse: Option<Rect>,
    /// The account control in the foot cell: a press opens the account menu.
    pub account: Option<Rect>,
}

/// The app's places.
#[derive(Debug, Clone)]
pub struct Rail<'a> {
    look: &'a Look,
    mark: String,
    name: String,
    groups: Vec<RailGroup>,
    collapsed: bool,
    collapsible: bool,
    foot: Option<RailFoot>,
    focused: Option<(usize, usize)>,
    words: RailWords,
}

impl<'a> Rail<'a> {
    pub fn new(look: &'a Look, mark: impl Into<String>, name: impl Into<String>) -> Rail<'a> {
        Rail {
            look,
            mark: mark.into(),
            name: name.into(),
            groups: Vec::new(),
            collapsed: false,
            collapsible: false,
            foot: None,
            focused: None,
            words: RailWords::default(),
        }
    }

    pub fn groups(mut self, groups: impl IntoIterator<Item = RailGroup>) -> Self {
        self.groups = groups.into_iter().collect();
        self
    }

    /// Collapsed to its icons; `collapsible` draws the head's control.
    pub fn collapsed(mut self, collapsed: bool, collapsible: bool) -> Self {
        self.collapsed = collapsed;
        self.collapsible = collapsible;
        self
    }

    pub fn foot(mut self, foot: RailFoot) -> Self {
        self.foot = Some(foot);
        self
    }

    /// The entry with keyboard focus, by group and place.
    pub fn focused(mut self, entry: Option<(usize, usize)>) -> Self {
        self.focused = entry;
        self
    }

    pub fn words(mut self, words: RailWords) -> Self {
        self.words = words;
        self
    }

    /// The app's name, as its head writes it.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The account's name in the foot cell, where it has one.
    pub fn account_name(&self) -> Option<String> {
        self.foot.as_ref().map(|f| f.name.clone())
    }

    /// Its width in columns: 240 px expanded (29), 57 collapsed (7).
    pub fn width(&self) -> u16 {
        if self.collapsed { cols(57.0) } else { cols(240.0) }
    }

    /// What the status line says while an entry has focus: a collapsed entry's label, its count
    /// or unread said in words (principle 23).
    pub fn tip(&self) -> Option<String> {
        let (g, e) = self.focused?;
        let entry = self.groups.get(g)?.entries.get(e)?;
        if !self.collapsed {
            return None;
        }
        let mut words = entry.label.clone();
        if let Some(c) = &entry.count {
            words += &format!("  {c}");
        } else if entry.unread {
            words += &format!("  {}", self.words.unread);
        }
        Some(words)
    }

    /// A group's entries as shown: a shut parent's children left out, and every child while
    /// collapsed (its flyout is not drawn yet).
    fn shown(&self, group: &RailGroup) -> Vec<usize> {
        let mut out = Vec::new();
        let mut open = true;
        for (e, entry) in group.entries.iter().enumerate() {
            if entry.depth == 0 {
                open = entry.parent.unwrap_or(true);
                out.push(e);
            } else if open && !self.collapsed {
                out.push(e);
            }
        }
        out
    }

    /// The rows an entry takes: one, and its description's while expanded.
    fn entry_rows(&self, entry: &RailEntry) -> u16 {
        1 + u16::from(!self.collapsed && entry.description.is_some())
    }

    /// Where the entries land in `area`, its foot cell `foot` rows at the bottom.
    pub fn layout(&self, area: Rect, foot: u16) -> RailLayout {
        let mut out = RailLayout::default();
        let look = self.look;
        if self.foot.is_some() && area.height > 0 {
            out.account = Some(Rect::new(area.x, area.bottom() - 1, area.width.saturating_sub(1), 1));
        }
        // The head, then the row level with the top bar's rule.
        let mut y = area.y + 1 + by_density(look, 1, 0);
        let bottom = area.bottom().saturating_sub(foot);
        let inner_w = area.width.saturating_sub(1);
        // Collapsed, the mark alone stands in the head and expands the rail: the head is the
        // control then; expanded, the control at the head's end.
        if self.collapsible {
            out.collapse = Some(if self.collapsed {
                Rect::new(area.x, area.y, inner_w, 1)
            } else {
                Rect::new(area.x + inner_w - 2, area.y, 1, 1)
            });
        }
        for (g, group) in self.groups.iter().enumerate() {
            if group.title.is_some() {
                if g > 0 {
                    y += by_density(look, 1, 0);
                }
                y += 1;
            }
            for e in self.shown(group) {
                if y >= bottom {
                    return out;
                }
                let rows = self.entry_rows(&group.entries[e]).min(bottom - y);
                out.entries.push(((g, e), Rect::new(area.x, y, inner_w, rows)));
                y += rows - 1;
                y += 1;
            }
        }
        out
    }
}

impl Widget for Rail<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &Rail<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 4 || area.height < 2 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let paint = |key: &str, field: &str| look.paint(&format!("rail.base.{key}"), field);
        let ground = paint("rest.frame", "ground").unwrap_or_else(|| look.color("sidebar"));
        // The rail owns its cells: a drawer over the content covers what is under it.
        Clear.render(area, buf);
        buf.set_style(area, Style::new().bg(t(ground)));
        // The edge: a column of the track glyph in sidebar-border at the rail's end.
        let edge = look.color("sidebar-border");
        let ex = area.right() - 1;
        for y in area.y..area.bottom() {
            buf[(ex, y)].set_symbol(glyphs.scroll_track).set_fg(t(edge));
        }
        let inner_w = area.width - 1;

        // The head: the mark's tile and the name; the collapse control at its end.
        let mark_style = Style::new()
            .bg(t(paint("rest.mark", "ground").unwrap_or(ground)))
            .fg(t(paint("rest.mark", "ink").unwrap_or(ground)))
            .add_modifier(ratatui::style::Modifier::BOLD);
        let mark = format!(" {} ", self.mark);
        let mx = if self.collapsed {
            area.x + inner_w.saturating_sub(3) / 2
        } else {
            area.x + 1
        };
        buf.set_string(mx, area.y, &mark, mark_style);
        if !self.collapsed {
            let name_ink = paint("rest.name", "ink").unwrap_or(ground);
            let room = inner_w.saturating_sub(6 + if self.collapsible { 3 } else { 0 });
            buf.set_string(
                area.x + 5,
                area.y,
                text::truncate(&self.name, room as usize, glyphs.ellipsis),
                TextRole::Label.style(look).fg(t(name_ink)),
            );
        }
        let layout = self.layout(area, 0);
        if let Some(r) = layout.collapse.filter(|_| !self.collapsed) {
            let g = glyphs.collapse;
            let ink = paint("rest.collapse", "ink").unwrap_or(ground);
            buf.set_string(r.x, r.y, g, Style::new().fg(t(ink)));
        }

        // The groups: a title after a blank row at regular, then the entries.
        let mut y = area.y + 1 + by_density(look, 1, 0);
        let foot_rows = if self.foot.is_some() {
            1 + by_density(look, 1, 0)
        } else {
            0
        };
        let bottom = area.bottom().saturating_sub(foot_rows);
        for (g, group) in self.groups.iter().enumerate() {
            if let Some(title) = &group.title {
                if g > 0 {
                    y += by_density(look, 1, 0);
                }
                if y >= bottom {
                    break;
                }
                if self.collapsed {
                    // A rule takes the title's place.
                    let rule = paint("rest.groupRule", "ground").unwrap_or(edge);
                    for x in area.x + 2..area.x + inner_w - 1 {
                        buf[(x, y)].set_symbol(glyphs.rule).set_fg(t(rule));
                    }
                } else {
                    let ink = paint("rest.groupTitle", "ink").unwrap_or_else(|| look.color("faint"));
                    buf.set_string(
                        area.x + 1,
                        y,
                        text::truncate(&title.to_uppercase(), (inner_w - 2) as usize, glyphs.ellipsis),
                        TextRole::Badge.style(look).fg(t(ink)),
                    );
                }
                y += 1;
            }
            for e in self.shown(group) {
                let entry = &group.entries[e];
                if y >= bottom {
                    break;
                }
                let state = if entry.active { "active" } else { "rest" };
                let key = format!("{state}.entry");
                let eg = paint(&key, "ground").unwrap_or(ground);
                let ei = paint(&key, "ink").unwrap_or(ground);
                let row = Rect::new(area.x, y, inner_w, 1);
                buf.set_style(row, Style::new().bg(t(eg)));
                if entry.active {
                    let bar = paint("active.entry", "edge").unwrap_or_else(|| look.color("primary"));
                    buf[(area.x, y)].set_symbol(glyphs.focus_mark).set_fg(t(bar));
                }
                if self.focused == Some((g, e)) {
                    buf[(area.x, y)]
                        .set_symbol(glyphs.focus_mark)
                        .set_fg(look.token("ring"));
                }
                let st = TextRole::Body.style(look).fg(t(ei));
                let st = if entry.active {
                    st.patch(TextRole::Label.style(look))
                } else {
                    st
                };
                if self.collapsed {
                    let icon = entry
                        .icon
                        .clone()
                        .unwrap_or_else(|| entry.label.chars().take(1).collect());
                    buf.set_string(area.x + inner_w.saturating_sub(1) / 2, y, icon, st);
                    if entry.unread || entry.count.is_some() {
                        let dot = paint("rest.unreadDot", "ground").unwrap_or(ei);
                        buf.set_string(area.x + inner_w - 2, y, glyphs.status_dot, Style::new().fg(t(dot)));
                    }
                } else {
                    // A child lines up under its parent's label; a parent's chevron at the end.
                    let mut x = area.x + 1 + 2 * entry.depth as u16;
                    if let Some(icon) = &entry.icon {
                        buf.set_string(x, y, icon, st);
                        x += 2;
                    }
                    let end_w = entry
                        .count
                        .as_ref()
                        .map_or(if entry.unread || entry.parent.is_some() { 1 } else { 0 }, |c| {
                            text::width(c) as u16
                        });
                    let room = (area.x + inner_w - 1).saturating_sub(x + end_w + 1);
                    buf.set_string(x, y, text::truncate(&entry.label, room as usize, glyphs.ellipsis), st);
                    let ex = area.x + inner_w - 1 - end_w;
                    if let Some(c) = &entry.count {
                        let ink = paint("rest.count", "ink").unwrap_or(ei);
                        buf.set_string(ex, y, c, TextRole::Help.style(look).fg(t(ink)));
                    } else if entry.unread {
                        let dot = paint("rest.unreadDot", "ground").unwrap_or(ei);
                        buf.set_string(ex, y, glyphs.status_dot, Style::new().fg(t(dot)));
                    } else if let Some(open) = entry.parent {
                        let g = if open { glyphs.chevron_down } else { glyphs.chevron };
                        let ink = look.color("muted-foreground");
                        buf.set_string(ex, y, g, Style::new().fg(t(ink)));
                    }
                    if let Some(d) = entry.description.as_ref().filter(|_| y + 1 < bottom) {
                        y += 1;
                        let dx = x;
                        let ink = look.color("muted-foreground");
                        let room = (area.x + inner_w - 1).saturating_sub(dx) as usize;
                        buf.set_style(Rect::new(area.x, y, inner_w, 1), Style::new().bg(t(eg)));
                        buf.set_string(
                            dx,
                            y,
                            text::truncate(d, room, glyphs.ellipsis),
                            TextRole::Help.style(look).fg(t(ink)),
                        );
                    }
                }
                y += 1;
            }
        }

        // The foot cell, level with the status line: its rule at regular, then the account.
        if let Some(foot) = &self.foot {
            let mut fy = area.bottom() - foot_rows;
            if foot_rows == 2 {
                let rule = paint("rest.footCell", "rule").unwrap_or(edge);
                for x in area.x..area.right() - 1 {
                    buf[(x, fy)].set_symbol(glyphs.rule).set_fg(t(rule));
                }
                fy += 1;
            }
            let tile = Style::new().bg(look.token(&foot.color)).fg(look.token("on-fill"));
            let tx = if self.collapsed {
                area.x + inner_w.saturating_sub(3) / 2
            } else {
                area.x + 1
            };
            buf.set_string(tx, fy, format!(" {} ", foot.letter), tile);
            if !self.collapsed {
                let ink = paint("docked.footCell", "ink").unwrap_or(ground);
                buf.set_string(
                    area.x + 5,
                    fy,
                    text::truncate(&foot.name, (inner_w - 6) as usize, glyphs.ellipsis),
                    TextRole::Body.style(look).fg(t(ink)),
                );
            }
        }
    }
}
