//! SettingsDialog, from sokkel-design `parts/settings-dialog`.
//!
//! Settings as a dialog over the page: a column of sections in groups with a search over every
//! setting, the shown section's settings in a pane, and a footer with Reset to defaults, the saved
//! note and Done. Its sections and settings are data, the same on every target; each setting's
//! control is the app's, drawn where [`SettingsDialog::layout`] puts it.
//!
//! In cells (the part carries no `cells` note yet; sokkel-tui's reading, in its proposal) the box is
//! 896 by 704 px (107 columns, 39 rows), at most the window less 4 across and 2 down. The head is a
//! row with the title and the close over a rule row; the column 224 px (27 columns) under a rule at
//! its end; an entry one row; the pane padded two columns across; a setting its title, its
//! description, its control at the row's end while the pane is 512 px (61 columns) wide or more,
//! else under its words, a note under them, and a rule row between settings. The footer is the
//! dialog's. Below md the column fills the window as a list, and a section fills it with Back.

use crate::button::{Button, ButtonVariant};
use crate::cells::{Width, cols};
use crate::dialog::{Confirm, Dialog, DialogWidth};
use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Widget};

/// One setting: its words, and the columns its control takes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Setting {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub note: Option<String>,
    pub keywords: Vec<String>,
    /// The columns the app's control takes, at the row's end or under its words.
    pub control_width: u16,
}

impl Setting {
    pub fn new(id: impl Into<String>, title: impl Into<String>, control_width: u16) -> Setting {
        Setting {
            id: id.into(),
            title: title.into(),
            control_width,
            ..Setting::default()
        }
    }

    pub fn description(mut self, words: impl Into<String>) -> Self {
        self.description = Some(words.into());
        self
    }

    pub fn note(mut self, words: impl Into<String>) -> Self {
        self.note = Some(words.into());
        self
    }

    pub fn keywords(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = words.into_iter().map(Into::into).collect();
        self
    }
}

/// A section: an entry in the column, its settings in the pane.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingsSection {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub heading: Option<String>,
    pub lead: Option<String>,
    pub settings: Vec<Setting>,
    /// False leaves Reset to defaults out while it is shown.
    pub reset: bool,
}

impl SettingsSection {
    pub fn new(id: impl Into<String>, label: impl Into<String>, settings: impl IntoIterator<Item = Setting>) -> Self {
        SettingsSection {
            id: id.into(),
            label: label.into(),
            settings: settings.into_iter().collect(),
            reset: true,
            ..SettingsSection::default()
        }
    }

    pub fn icon(mut self, glyph: impl Into<String>) -> Self {
        self.icon = Some(glyph.into());
        self
    }

    pub fn lead(mut self, words: impl Into<String>) -> Self {
        self.lead = Some(words.into());
        self
    }

    pub fn no_reset(mut self) -> Self {
        self.reset = false;
        self
    }
}

/// Sections under a title, or none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsGroup {
    pub title: Option<String>,
    pub sections: Vec<SettingsSection>,
}

impl SettingsGroup {
    pub fn new(title: Option<&str>, sections: impl IntoIterator<Item = SettingsSection>) -> Self {
        SettingsGroup {
            title: title.map(String::from),
            sections: sections.into_iter().collect(),
        }
    }
}

/// The words a settings dialog says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsWords {
    pub title: String,
    pub search: String,
    pub no_match: String,
    pub saved: String,
    pub done: String,
    pub reset: String,
    pub reset_section: String,
    pub reset_section_detail: String,
    pub reset_confirm: String,
    pub cancel: String,
    pub close: String,
    pub back: String,
}

impl Default for SettingsWords {
    fn default() -> Self {
        SettingsWords {
            title: "Settings".into(),
            search: "Search".into(),
            no_match: "No setting matches the search.".into(),
            saved: "Changes are saved as you make them".into(),
            done: "Done".into(),
            reset: "Reset to defaults".into(),
            reset_section: "Reset {section} to its defaults?".into(),
            reset_section_detail: "Every setting in {section} goes back to its default.".into(),
            reset_confirm: "Reset".into(),
            cancel: "Cancel".into(),
            close: "Close".into(),
            back: "Back".into(),
        }
    }
}

/// Which of the dialog's own stops has keyboard focus. A setting's control is the app's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    #[default]
    Search,
    /// An entry in the column, by (group, section).
    Entry(usize, usize),
    Close,
    Back,
    Reset,
    Done,
    /// The app's control for a setting, by its place in the pane.
    Control(usize),
}

/// What lives between frames: the shown section, the query, the phone's view.
#[derive(Debug, Clone, Default)]
pub struct SettingsState {
    /// The shown section, by (group, section).
    pub shown: (usize, usize),
    pub query: String,
    pub focus: SettingsFocus,
    /// Below md: in a section (true) or on the list.
    pub in_section: bool,
    cursor_at: Option<Position>,
}

impl SettingsState {
    pub fn cursor_position(&self) -> Option<Position> {
        self.cursor_at
    }
}

/// What a key did to the settings dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsEvent {
    Ignored,
    Changed,
    /// Closed: Done, the close or Escape. On `#/settings`, back to the page it was opened from.
    Close,
    /// Reset to defaults was pressed: ask with [`SettingsDialog::reset_confirm`].
    AskReset,
}

/// Where the dialog put its pieces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsLayout {
    pub search: Option<Rect>,
    pub entries: Vec<((usize, usize), Rect)>,
    pub close: Option<Rect>,
    pub back: Option<Rect>,
    pub reset: Option<Rect>,
    pub done: Option<Rect>,
    /// Each shown setting's control, by its index in the section.
    pub controls: Vec<(usize, Rect)>,
}

/// The settings dialog.
#[derive(Debug, Clone)]
pub struct SettingsDialog<'a> {
    look: &'a Look,
    groups: Vec<SettingsGroup>,
    note: Option<String>,
    phone: bool,
    words: SettingsWords,
}

impl<'a> SettingsDialog<'a> {
    pub fn new(look: &'a Look, groups: impl IntoIterator<Item = SettingsGroup>) -> SettingsDialog<'a> {
        SettingsDialog {
            look,
            groups: groups.into_iter().collect(),
            note: None,
            phone: false,
            words: SettingsWords::default(),
        }
    }

    /// The app's footer note, in place of "Changes are saved as you make them".
    pub fn note(mut self, words: impl Into<String>) -> Self {
        self.note = Some(words.into());
        self
    }

    /// The window's width decides its form: below md a list, then a section, filling the window.
    pub fn window(mut self, width: Width) -> Self {
        self.phone = width == Width::Narrow;
        self
    }

    pub fn words(mut self, words: SettingsWords) -> Self {
        self.words = words;
        self
    }

    fn section(&self, at: (usize, usize)) -> Option<&SettingsSection> {
        self.groups.get(at.0)?.sections.get(at.1)
    }

    /// Whether a setting holds every word typed, in its words or its section's.
    fn keeps(&self, section: &SettingsSection, setting: &Setting, query: &str) -> bool {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return true;
        }
        let mut hay = vec![setting.title.to_lowercase(), section.label.to_lowercase()];
        hay.extend(setting.description.iter().map(|d| d.to_lowercase()));
        hay.extend(section.heading.iter().map(|d| d.to_lowercase()));
        hay.extend(setting.keywords.iter().map(|k| k.to_lowercase()));
        words.iter().all(|w| hay.iter().any(|h| h.contains(w.as_str())))
    }

    /// The settings of a section the query keeps, by their index.
    pub fn kept(&self, at: (usize, usize), query: &str) -> Vec<usize> {
        self.section(at).map_or(Vec::new(), |s| {
            (0..s.settings.len())
                .filter(|&k| self.keeps(s, &s.settings[k], query))
                .collect()
        })
    }

    /// The sections the query leaves, in order.
    pub fn sections(&self, query: &str) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (g, group) in self.groups.iter().enumerate() {
            for s in 0..group.sections.len() {
                if query.trim().is_empty() || !self.kept((g, s), query).is_empty() {
                    out.push((g, s));
                }
            }
        }
        out
    }

    /// The section shown: the chosen one, else the first the search leaves.
    pub fn shown(&self, state: &SettingsState) -> Option<(usize, usize)> {
        let left = self.sections(&state.query);
        if left.contains(&state.shown) {
            Some(state.shown)
        } else {
            left.first().copied()
        }
    }

    /// Answer a key on the dialog's own stops.
    pub fn handle(&self, state: &mut SettingsState, key: KeyPress) -> SettingsEvent {
        if key.alt || key.ctrl {
            return SettingsEvent::Ignored;
        }
        if key.key == Key::Escape {
            if self.phone && state.in_section {
                state.in_section = false;
                return SettingsEvent::Changed;
            }
            return SettingsEvent::Close;
        }
        match state.focus {
            SettingsFocus::Search => match key.key {
                Key::Char(c) => {
                    state.query.push(c);
                    SettingsEvent::Changed
                }
                Key::Backspace => {
                    state.query.pop();
                    SettingsEvent::Changed
                }
                _ => SettingsEvent::Ignored,
            },
            SettingsFocus::Entry(g, s) if key.is_press() => {
                state.shown = (g, s);
                state.in_section = self.phone;
                SettingsEvent::Changed
            }
            SettingsFocus::Back if key.is_press() => {
                state.in_section = false;
                SettingsEvent::Changed
            }
            SettingsFocus::Close | SettingsFocus::Done if key.is_press() => SettingsEvent::Close,
            SettingsFocus::Reset if key.is_press() => SettingsEvent::AskReset,
            _ => SettingsEvent::Ignored,
        }
    }

    /// The ConfirmDialog Reset to defaults asks before it resets the shown section.
    pub fn reset_confirm(&self, state: &SettingsState) -> Dialog<'a> {
        let name = self
            .shown(state)
            .and_then(|at| self.section(at))
            .map_or(String::new(), |s| s.label.clone());
        Dialog::new(self.look, self.words.reset_section.replace("{section}", &name))
            .description(self.words.reset_section_detail.replace("{section}", &name))
            .width(DialogWidth::Sm)
            .confirm(Confirm::new(self.words.reset_confirm.clone()).destructive())
    }

    /// Where it stands in `window`: centered, at most the window less 4 across and 2 down; below
    /// md the whole window.
    pub fn area(&self, window: Rect) -> Rect {
        if self.phone {
            return window;
        }
        let w = cols(896.0).min(window.width.saturating_sub(4));
        let h = 39.min(window.height.saturating_sub(2));
        Rect::new(
            window.x + (window.width - w) / 2,
            window.y + (window.height - h) / 2,
            w,
            h,
        )
    }

    fn inner(&self, area: Rect) -> Rect {
        if self.phone {
            area
        } else {
            Rect::new(
                area.x + 1,
                area.y + 1,
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
            )
        }
    }

    fn shows_reset(&self, state: &SettingsState) -> bool {
        let shown = self.shown(state).and_then(|at| self.section(at));
        let section_view = !self.phone || state.in_section;
        shown.is_some_and(|s| s.reset) && section_view && !self.phone
    }

    /// Where the pieces land in the dialog's `area`.
    pub fn layout(&self, area: Rect, state: &SettingsState) -> SettingsLayout {
        let mut out = SettingsLayout::default();
        let inner = self.inner(area);
        let words = &self.words;
        // The head: Back on a phone in a section, the title, the close at its end.
        if self.phone && state.in_section {
            out.back = Some(Rect::new(inner.x + 1, inner.y, 3, 1));
        }
        out.close = Some(Rect::new(inner.right().saturating_sub(4), inner.y, 3, 1));
        // The footer: Reset, the note, Done.
        let fy = inner.bottom().saturating_sub(1);
        let done_w = text::width(&words.done) as u16 + 2;
        out.done = Some(Rect::new(inner.right().saturating_sub(done_w + 2), fy, done_w, 1));
        if self.shows_reset(state) {
            let w = text::width(&words.reset) as u16 + 2;
            out.reset = Some(Rect::new(inner.x + 2, fy, w, 1));
        }
        let body = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(4),
            ..inner
        };
        let column_w = if self.phone {
            body.width
        } else {
            cols(224.0).min(body.width / 2)
        };
        let show_column = !self.phone || !state.in_section;
        let show_pane = !self.phone || state.in_section;
        if show_column {
            out.search = Some(Rect::new(body.x + 1, body.y, column_w.saturating_sub(3), 1));
            let mut y = body.y + 2;
            let left = self.sections(&state.query);
            for (g, group) in self.groups.iter().enumerate() {
                if !left.iter().any(|&(lg, _)| lg == g) {
                    continue;
                }
                if group.title.is_some() {
                    y += 1;
                }
                for (s, _) in group.sections.iter().enumerate() {
                    if !left.contains(&(g, s)) || y >= body.bottom() {
                        continue;
                    }
                    out.entries
                        .push(((g, s), Rect::new(body.x + 1, y, column_w.saturating_sub(3), 1)));
                    y += 1;
                }
                y += 1;
            }
        }
        if show_pane && let Some(at) = self.shown(state) {
            let pane = if self.phone {
                body
            } else {
                Rect {
                    x: body.x + column_w + 1,
                    width: body.width.saturating_sub(column_w + 1),
                    ..body
                }
            };
            let px = pane.x + 2;
            let pw = pane.width.saturating_sub(4);
            let trailing = pane.width as f64 * crate::cells::PX_PER_COL >= 512.0;
            let section = self.section(at).expect("a shown section");
            let mut y = pane.y
                + 1
                + section
                    .lead
                    .as_ref()
                    .map_or(0, |l| text::wrap(l, pw as usize).len() as u16)
                + 1;
            for k in self.kept(at, &state.query) {
                let setting = &section.settings[k];
                let cw = setting.control_width.min(pw);
                let words_w = if trailing { pw.saturating_sub(cw + 4) } else { pw };
                let desc = setting
                    .description
                    .as_ref()
                    .map_or(0, |d| text::wrap(d, words_w as usize).len() as u16);
                let r = if trailing {
                    Rect::new(px + pw - cw, y, cw, 1)
                } else {
                    Rect::new(px, y + 1 + desc, cw, 1)
                };
                if r.y < pane.bottom() {
                    out.controls.push((k, r));
                }
                y += 1 + desc + u16::from(!trailing) + u16::from(setting.note.is_some()) + 1;
            }
        }
        out
    }
}

impl SettingsDialog<'_> {
    /// Draw it in `area` (from [`SettingsDialog::area`]); then the app draws each control where
    /// [`SettingsDialog::layout`] puts it.
    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &mut SettingsState) {
        let area = area.intersection(buf.area);
        state.cursor_at = None;
        if area.width < 20 || area.height < 8 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let p = |piece: &str, field: &str| look.paint(&format!("settings-dialog.base.{piece}"), field);
        let ground = p("rest.box", "ground").unwrap_or_else(|| look.color("popover"));
        let ink = p("rest.box", "ink").unwrap_or_else(|| look.color("popover-foreground"));
        let edge = p("rest.box", "edge").unwrap_or(ground);
        let quiet = look.color("muted-foreground");
        let rule = p("rest.head", "rule").unwrap_or(edge);
        Clear.render(area, buf);
        if self.phone {
            buf.set_style(area, Style::new().bg(t(ground)).fg(t(ink)));
        } else {
            let block = Block::new()
                .borders(Borders::ALL)
                .border_set(look.border_set("radius-surface"))
                .border_style(Style::new().fg(t(edge)).bg(t(look.color("background"))));
            block.render(area, buf);
            buf.set_style(self.inner(area), Style::new().bg(t(ground)).fg(t(ink)));
        }
        let inner = self.inner(area);
        let layout = self.layout(area, state);
        let hline = |buf: &mut Buffer, y: u16, x0: u16, x1: u16, c: Rgba| {
            for x in x0..x1 {
                buf[(x, y)].set_symbol(glyphs.rule).set_fg(t(c));
            }
        };

        // The head and its rule.
        let mut tx = inner.x + 2;
        if let Some(r) = layout.back {
            Button::icon(look, glyphs.back, self.words.back.clone())
                .variant(ButtonVariant::Ghost)
                .focused(state.focus == SettingsFocus::Back)
                .render(r, buf);
            tx = r.right() + 1;
        }
        let shown = self.shown(state);
        let title = match (self.phone && state.in_section, shown.and_then(|at| self.section(at))) {
            (true, Some(s)) => s.label.clone(),
            _ => self.words.title.clone(),
        };
        buf.set_string(tx, inner.y, &title, TextRole::DetailTitle.style(look).fg(t(ink)));
        if let Some(r) = layout.close {
            Button::icon(look, glyphs.close, self.words.close.clone())
                .variant(ButtonVariant::Ghost)
                .focused(state.focus == SettingsFocus::Close)
                .render(r, buf);
        }
        hline(buf, inner.y + 1, inner.x, inner.right(), rule);

        // The column: the search, then each group's entries, the shown one marked.
        if let Some(r) = layout.search {
            let key = if state.focus == SettingsFocus::Search {
                "focus"
            } else {
                "rest"
            };
            let ck = format!("text-field.base.{key}.control");
            let fg = look.paint(&ck, "ground").unwrap_or(ground);
            let fe = look.paint(&ck, "edge").unwrap_or(fg);
            buf.set_style(r, Style::new().bg(t(fg)));
            buf[(r.x, r.y)].set_symbol(glyphs.edge_start).set_fg(t(fe));
            buf[(r.right() - 1, r.y)].set_symbol(glyphs.edge_end).set_fg(t(fe));
            let words = if state.query.is_empty() {
                &self.words.search
            } else {
                &state.query
            };
            let wi = if state.query.is_empty() {
                look.paint(&ck, "placeholder").unwrap_or(quiet)
            } else {
                look.paint(&ck, "ink").unwrap_or(ink)
            };
            buf.set_string(r.x + 1, r.y, glyphs.search, Style::new().fg(t(quiet)));
            let room = r.width.saturating_sub(4) as usize;
            let shown_q = text::truncate(words, room, glyphs.ellipsis);
            buf.set_string(r.x + 3, r.y, &shown_q, TextRole::Body.style(look).fg(t(wi)));
            if state.focus == SettingsFocus::Search {
                let end = if state.query.is_empty() {
                    0
                } else {
                    text::width(&shown_q) as u16
                };
                state.cursor_at = Some(Position::new(r.x + 3 + end, r.y));
            }
            // The column's rule at its end.
            if !self.phone {
                let cx = r.right() + 1;
                let crule = p("rest.column", "rule").unwrap_or(rule);
                for y in inner.y + 2..inner.bottom().saturating_sub(2) {
                    buf[(cx, y)].set_symbol(glyphs.scroll_track).set_fg(t(crule));
                }
            }
            let mut last_group = None;
            for ((g, s), er) in &layout.entries {
                if last_group != Some(*g) {
                    if let Some(title) = &self.groups[*g].title {
                        let gi = p("rest.groupTitle", "ink").unwrap_or(quiet);
                        buf.set_string(
                            er.x,
                            er.y - 1,
                            title.to_uppercase(),
                            TextRole::Badge.style(look).fg(t(gi)),
                        );
                    }
                    last_group = Some(*g);
                }
                let section = &self.groups[*g].sections[*s];
                let is_shown = !self.phone && shown == Some((*g, *s));
                let st = if self.phone {
                    "phoneList"
                } else if is_shown {
                    "shown"
                } else {
                    "rest"
                };
                let eg = p(&format!("{st}.entry"), "ground").unwrap_or(ground);
                let ei = p(&format!("{st}.entry"), "ink").unwrap_or(quiet);
                buf.set_style(*er, Style::new().bg(t(eg)));
                let role = if is_shown { TextRole::Action } else { TextRole::Body };
                let mut x = er.x + 1;
                if let Some(icon) = &section.icon {
                    buf.set_string(x, er.y, icon, Style::new().fg(t(ei)).bg(t(eg)));
                    x += text::width(icon) as u16 + 1;
                }
                let room = er.right().saturating_sub(x + 2) as usize;
                buf.set_string(
                    x,
                    er.y,
                    text::truncate(&section.label, room, glyphs.ellipsis),
                    role.style(look).fg(t(ei)).bg(t(eg)),
                );
                if self.phone {
                    let ci = p("phoneList.chevron", "ink").unwrap_or(quiet);
                    buf.set_string(er.right() - 2, er.y, glyphs.chevron, Style::new().fg(t(ci)).bg(t(eg)));
                }
                if state.focus == SettingsFocus::Entry(*g, *s) {
                    buf[(er.x, er.y)]
                        .set_symbol(glyphs.focus_mark)
                        .set_fg(look.token("ring"));
                }
            }
        }

        // The pane: the heading, the lead, the settings ruled apart; or no match.
        let show_pane = !self.phone || state.in_section;
        if show_pane {
            let body = Rect {
                y: inner.y + 2,
                height: inner.height.saturating_sub(4),
                ..inner
            };
            let column_w = if self.phone {
                0
            } else {
                cols(224.0).min(body.width / 2) + 1
            };
            let pane = Rect {
                x: body.x + column_w,
                width: body.width.saturating_sub(column_w),
                ..body
            };
            let px = pane.x + 2;
            let pw = pane.width.saturating_sub(4);
            match shown.and_then(|at| self.section(at).map(|s| (at, s))) {
                None => {
                    let ni = p("rest.noMatch", "ink").unwrap_or(quiet);
                    buf.set_string(px, pane.y, &self.words.no_match, TextRole::Body.style(look).fg(t(ni)));
                }
                Some((at, section)) => {
                    let mut y = pane.y;
                    let heading = section.heading.as_ref().unwrap_or(&section.label);
                    if !self.phone {
                        buf.set_string(px, y, heading, TextRole::DetailTitle.style(look).fg(t(ink)));
                        y += 1;
                    }
                    if let Some(lead) = &section.lead {
                        let li = p("rest.lead", "ink").unwrap_or(quiet);
                        for l in text::wrap(lead, pw as usize) {
                            buf.set_string(px, y, l, TextRole::Body.style(look).fg(t(li)));
                            y += 1;
                        }
                    }
                    if self.phone {
                        y = pane.y
                            + 1
                            + section
                                .lead
                                .as_ref()
                                .map_or(0, |l| text::wrap(l, pw as usize).len() as u16);
                    } else {
                        y += 1;
                    }
                    let trailing = pane.width as f64 * crate::cells::PX_PER_COL >= 512.0;
                    let row_rule = p("rest.rowRule", "rule").unwrap_or(rule);
                    let kept = self.kept(at, &state.query);
                    for (n, k) in kept.iter().enumerate() {
                        let setting = &section.settings[*k];
                        if y >= pane.bottom() {
                            break;
                        }
                        if n > 0 {
                            hline(buf, y - 1, px, px + pw, row_rule);
                        }
                        let words_w = if trailing {
                            pw.saturating_sub(setting.control_width.min(pw) + 4)
                        } else {
                            pw
                        };
                        buf.set_string(
                            px,
                            y,
                            text::truncate(&setting.title, words_w as usize, glyphs.ellipsis),
                            TextRole::Action.style(look).fg(t(ink)),
                        );
                        y += 1;
                        if let Some(d) = &setting.description {
                            let di = p("rest.rowDescription", "ink").unwrap_or(quiet);
                            for l in text::wrap(d, words_w as usize) {
                                if y < pane.bottom() {
                                    buf.set_string(px, y, l, TextRole::Body.style(look).fg(t(di)));
                                }
                                y += 1;
                            }
                        }
                        if !trailing {
                            y += 1;
                        }
                        if let Some(note) = &setting.note
                            && y < pane.bottom()
                        {
                            let ng = p("rest.rowNote", "ground").unwrap_or(ground);
                            let ni = p("rest.rowNote", "ink").unwrap_or(quiet);
                            let words = format!(
                                " {} ",
                                text::truncate(note, pw.saturating_sub(2) as usize, glyphs.ellipsis)
                            );
                            buf.set_string(px, y, words, TextRole::Help.style(look).fg(t(ni)).bg(t(ng)));
                            y += 1;
                        }
                        y += 1;
                    }
                }
            }
        }

        // The footer: a rule, then Reset to defaults, the note, Done.
        let fk = "settings-dialog.base.rest.footer";
        let fg = look.paint(fk, "ground").unwrap_or(ground);
        let fy = inner.bottom().saturating_sub(2);
        buf.set_style(Rect::new(inner.x, fy, inner.width, 2), Style::new().bg(t(fg)));
        hline(buf, fy, inner.x, inner.right(), look.paint(fk, "rule").unwrap_or(rule));
        let note = self.note.as_ref().unwrap_or(&self.words.saved);
        let si = p("rest.saved", "ink").unwrap_or(quiet);
        let nx = layout.reset.map_or(inner.x + 2, |r| r.right() + 2);
        let room = layout.done.map_or(inner.right(), |r| r.x).saturating_sub(nx + 2) as usize;
        buf.set_string(
            nx,
            fy + 1,
            text::truncate(note, room, glyphs.ellipsis),
            TextRole::Help.style(look).fg(t(si)).bg(t(fg)),
        );
        if let Some(r) = layout.reset {
            Button::new(look, self.words.reset.clone())
                .variant(ButtonVariant::Ghost)
                .focused(state.focus == SettingsFocus::Reset)
                .render(r, buf);
        }
        if let Some(r) = layout.done {
            Button::new(look, self.words.done.clone())
                .focused(state.focus == SettingsFocus::Done)
                .render(r, buf);
        }
    }
}
