//! Select, from sokkel-design `parts/select`.
//!
//! A trigger that shows the chosen option, and a list that opens under it to choose another. The
//! field variant stands in a form; the quiet one in a toolbar.
//!
//! In cells the trigger is one row: a field's edge in its pad cells as a text field's control, the
//! chosen option's lead and label, and the chevron at its end. Keyboard focus on the field variant
//! is its focus edge, as a text field's; on the quiet one, which draws no edge, the ring beside each
//! end (open-questions T4, as decided). The list is [`crate::popup`]'s, under the trigger, at
//! least 144 px (17 columns) wide, the chosen row checked at its end.

use crate::keys::{Key, KeyPress, Pointer, PointerKind};
use crate::look::{Look, Rgba};
use crate::popup::{Item, Popup, PopupEvent, PopupKind, PopupRow, handle};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectVariant {
    /// In a form: the field's ground and edge.
    #[default]
    Field,
    /// In a toolbar: no ground or edge at rest.
    Quiet,
}

/// One option.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectOption {
    pub label: String,
    pub lead: Option<String>,
    /// The lead's color, a color token: a swatch.
    pub swatch: Option<String>,
    pub detail: Option<String>,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(label: impl Into<String>) -> SelectOption {
        SelectOption {
            label: label.into(),
            ..SelectOption::default()
        }
    }

    pub fn lead(mut self, glyph: impl Into<String>) -> Self {
        self.lead = Some(glyph.into());
        self
    }

    /// A swatch before the label: a block in the `color` token (a theme's color, a series'), so
    /// the option shows what it picks.
    pub fn swatch(mut self, color: impl Into<String>) -> Self {
        self.swatch = Some(color.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Whether the list is open, and its highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectState {
    pub open: bool,
    pub highlight: usize,
    /// A press opened the list (not a key): it is laid over the trigger where it may be.
    pub by_pointer: bool,
}

/// What a key did to the select.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectEvent {
    Ignored,
    Opened,
    Moved,
    /// An option was chosen, by its index.
    Chose(usize),
    /// Closed without choosing: focus back on the trigger.
    Closed,
}

/// A select. Build it each frame; draw its trigger, then its list last.
#[derive(Debug, Clone)]
pub struct Select<'a> {
    look: &'a Look,
    options: Vec<SelectOption>,
    value: Option<usize>,
    placeholder: String,
    variant: SelectVariant,
    focused: bool,
    disabled: bool,
    read_only: bool,
    invalid: bool,
    align_item_with_trigger: bool,
}

impl<'a> Select<'a> {
    pub fn new(look: &'a Look, options: impl IntoIterator<Item = SelectOption>) -> Select<'a> {
        Select {
            look,
            options: options.into_iter().collect(),
            value: None,
            placeholder: String::new(),
            variant: SelectVariant::Field,
            focused: false,
            disabled: false,
            read_only: false,
            invalid: false,
            align_item_with_trigger: true,
        }
    }

    /// The list laid over the trigger, the chosen row level with the value, when a pointer opened
    /// it and there is room (`alignItemWithTrigger`, the default); off, it always opens under it.
    pub fn align_item_with_trigger(mut self, on: bool) -> Self {
        self.align_item_with_trigger = on;
        self
    }

    pub fn value(mut self, value: Option<usize>) -> Self {
        self.value = value;
        self
    }

    pub fn placeholder(mut self, words: impl Into<String>) -> Self {
        self.placeholder = words.into();
        self
    }

    pub fn variant(mut self, variant: SelectVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    /// Tab passes over a disabled select.
    pub fn focusable(&self) -> bool {
        !self.disabled
    }

    fn rows(&self) -> Vec<PopupRow> {
        self.options
            .iter()
            .map(|o| {
                let mut i = Item::new(o.label.clone()).disabled(o.disabled);
                i.icon = o
                    .lead
                    .clone()
                    .or_else(|| o.swatch.as_ref().map(|_| self.look.glyphs().swatch.to_string()));
                i.icon_color = o.swatch.clone();
                i.detail = o.detail.clone();
                PopupRow::Item(i)
            })
            .collect()
    }

    /// The trigger's own width: the widest label, its lead, the chevron and the padding.
    pub fn width(&self) -> u16 {
        let widest = self
            .options
            .iter()
            .map(|o| {
                text::width(&o.label)
                    + o.lead
                        .as_ref()
                        .map_or(usize::from(o.swatch.is_some()) * 2, |l| text::width(l) + 1)
            })
            .chain(std::iter::once(text::width(&self.placeholder)))
            .max()
            .unwrap_or(0);
        widest as u16 + 5
    }

    /// Answer a key on the trigger, or in the open list.
    pub fn handle(&self, state: &mut SelectState, key: KeyPress) -> SelectEvent {
        if self.disabled || key.alt || key.ctrl {
            return SelectEvent::Ignored;
        }
        if !state.open {
            return match key.key {
                Key::Enter | Key::Char(' ') | Key::Down | Key::Up => {
                    state.open = true;
                    state.by_pointer = false;
                    state.highlight = self.value.unwrap_or(0);
                    SelectEvent::Opened
                }
                // A character chooses the next option it starts, the list staying closed.
                Key::Char(c) if !self.read_only => {
                    let c = c.to_lowercase().next().unwrap_or(c);
                    let n = self.options.len();
                    let from = self.value.map_or(0, |v| v + 1);
                    (0..n)
                        .map(|k| (from + k) % n)
                        .find(|&k| !self.options[k].disabled && self.options[k].label.to_lowercase().starts_with(c))
                        .map_or(SelectEvent::Ignored, SelectEvent::Chose)
                }
                _ => SelectEvent::Ignored,
            };
        }
        let rows = self.rows();
        match handle(&rows, &mut state.highlight, key, false) {
            PopupEvent::Moved => SelectEvent::Moved,
            PopupEvent::Closed => {
                state.open = false;
                SelectEvent::Closed
            }
            PopupEvent::Chosen(k) => {
                state.open = false;
                if self.read_only {
                    SelectEvent::Closed
                } else {
                    SelectEvent::Chose(k)
                }
            }
            _ => SelectEvent::Ignored,
        }
    }

    fn state_key(&self, open: bool) -> &'static str {
        match (self.invalid, self.focused, open) {
            (true, true, _) => "invalid+focus",
            (true, false, _) => "invalid",
            (false, _, true) => "open",
            (false, true, _) => "focus",
            _ if self.value.is_none() => "placeholder",
            _ => "rest",
        }
    }

    /// Draw the trigger in `area` (one row).
    pub fn render_trigger(&self, area: Rect, buf: &mut Buffer, state: &SelectState) {
        let area = area.intersection(buf.area);
        if area.width < 5 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let variant = match self.variant {
            SelectVariant::Field => "field",
            SelectVariant::Quiet => "quiet",
        };
        let key = format!("select.{variant}.{}.container", self.state_key(state.open));
        let rest = format!("select.{variant}.rest.container");
        let pick = |f: &str| {
            if self.disabled {
                look.faded(&rest, f)
            } else {
                look.paint(&key, f)
            }
        };
        let page = look.color("background");
        let ground = pick("ground").unwrap_or(page);
        let ink = pick("ink").unwrap_or_else(|| look.color("foreground"));
        let row = Rect { height: 1, ..area };
        let draws_ground = look.shows(&key, "ground") || self.disabled;
        let base = if draws_ground {
            Style::new().bg(t(ground))
        } else {
            Style::new()
        };
        buf.set_style(row, base);
        if look.shows(&key, "edge") {
            let edge = pick("edge").unwrap_or(ground);
            buf[(row.x, row.y)]
                .set_symbol(glyphs.edge_start)
                .set_style(base.fg(t(edge)));
            buf[(row.right() - 1, row.y)]
                .set_symbol(glyphs.edge_end)
                .set_style(base.fg(t(edge)));
        }
        let mut x = row.x + 1;
        let chosen = self.value.and_then(|v| self.options.get(v));
        let style = TextRole::Body.style(look).patch(base).fg(t(ink));
        let swatch_glyph = chosen
            .and_then(|o| o.swatch.as_ref())
            .map(|_| glyphs.swatch.to_string());
        if let Some(lead) = chosen.and_then(|o| o.lead.clone()).or(swatch_glyph).as_ref() {
            let swatch = chosen.and_then(|o| o.swatch.as_deref()).filter(|c| look.has_color(c));
            let style = swatch.map_or(style, |c| style.fg(look.token(c)));
            buf.set_string(x, row.y, lead, style);
            x += text::width(lead) as u16 + 1;
        }
        let words = chosen.map_or(self.placeholder.as_str(), |o| o.label.as_str());
        let room = row.right().saturating_sub(x + 3) as usize;
        buf.set_string(x, row.y, text::truncate(words, room, glyphs.ellipsis), style);
        let chevron_key = "select.base.rest.chevron";
        let ci = if self.disabled {
            look.faded(chevron_key, "ink")
        } else {
            look.paint(chevron_key, "ink")
        }
        .unwrap_or(ink);
        buf.set_string(row.right() - 2, row.y, glyphs.chevron_down, base.fg(t(ci)));
        // The quiet variant has no edge to show focus: the ring beside each end.
        if self.focused && self.variant == SelectVariant::Quiet {
            let ring = look.color("ring");
            if row.x > buf.area.x {
                buf[(row.x - 1, row.y)].set_symbol(glyphs.ring_start).set_fg(t(ring));
            }
            if row.right() < buf.area.right() {
                buf[(row.right(), row.y)].set_symbol(glyphs.ring_end).set_fg(t(ring));
            }
        }
    }

    /// Where the open list stands for the trigger at `anchor` in `window`.
    pub fn list_area(&self, window: Rect, anchor: Rect) -> Rect {
        let rows = self.rows();
        Popup::new(self.look, PopupKind::Select, &rows).place(anchor, window, crate::cells::cols(144.0))
    }

    /// Where the open list stands: over the trigger, the chosen row on the value's row, when a
    /// pointer opened it and it fits there; else under the trigger ([`Select::list_area`]).
    pub fn list_area_open(&self, window: Rect, anchor: Rect, state: &SelectState) -> Rect {
        let under = self.list_area(window, anchor);
        if !self.align_item_with_trigger || !state.by_pointer {
            return under;
        }
        let k = self.value.unwrap_or(0) as u16;
        let value_y = anchor.y + anchor.height.saturating_sub(1) / 2;
        // The list's border is the row above its first option.
        let Some(y) = value_y.checked_sub(1 + k).filter(|&y| y >= window.y) else {
            return under;
        };
        if y + under.height > window.bottom() {
            return under;
        }
        let x = anchor.x.max(window.x).min(window.right().saturating_sub(under.width));
        Rect { x, y, ..under }
    }

    /// The option a pointer at `at` is on in the open list; not a disabled one.
    pub fn option_at(&self, window: Rect, anchor: Rect, at: Position, state: &SelectState) -> Option<usize> {
        if !state.open {
            return None;
        }
        let area = self.list_area_open(window, anchor, state);
        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        if !inner.contains(at) {
            return None;
        }
        let k = (at.y - inner.y) as usize;
        self.options.get(k).filter(|o| !o.disabled).map(|_| k)
    }

    /// A press on the trigger opens or closes the list; on an option chooses it.
    pub fn press(&self, state: &mut SelectState, option: Option<usize>) -> SelectEvent {
        if self.disabled {
            return SelectEvent::Ignored;
        }
        match option {
            Some(k) => {
                state.open = false;
                if self.read_only {
                    SelectEvent::Closed
                } else {
                    SelectEvent::Chose(k)
                }
            }
            None if state.open => {
                state.open = false;
                SelectEvent::Closed
            }
            None => {
                state.open = true;
                state.highlight = self.value.unwrap_or(0);
                SelectEvent::Opened
            }
        }
    }

    /// A pointer event on the select with its trigger at `anchor`. A press on the trigger opens or
    /// closes the list; open, the highlight follows the pointer, a press on an option chooses it,
    /// and a press outside the list closes it.
    pub fn pointer(&self, state: &mut SelectState, window: Rect, anchor: Rect, p: Pointer) -> SelectEvent {
        let option = if state.open {
            self.option_at(window, anchor, p.at, state)
        } else {
            None
        };
        match p.kind {
            PointerKind::Move => match option {
                Some(k) if k != state.highlight => {
                    state.highlight = k;
                    SelectEvent::Moved
                }
                _ => SelectEvent::Ignored,
            },
            PointerKind::Press if !state.open && !anchor.contains(p.at) => SelectEvent::Ignored,
            PointerKind::Press
                if state.open && option.is_none() && self.list_area_open(window, anchor, state).contains(p.at) =>
            {
                SelectEvent::Ignored
            }
            PointerKind::Press => {
                if !state.open {
                    state.by_pointer = true;
                }
                self.press(state, option)
            }
            _ => SelectEvent::Ignored,
        }
    }

    /// Draw the open list for the trigger at `anchor`, in `window`: last, over what is under it.
    pub fn render_list(&self, window: Rect, anchor: Rect, buf: &mut Buffer, state: &SelectState) {
        if !state.open {
            return;
        }
        let rows = self.rows();
        let popup = Popup::new(self.look, PopupKind::Select, &rows)
            .highlight(Some(state.highlight))
            .chosen(self.value);
        let area = self.list_area_open(window, anchor, state);
        (&popup).render(area, buf);
    }
}
