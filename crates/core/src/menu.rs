//! DropdownMenu, from sokkel-design `parts/dropdown-menu`.
//!
//! A popup of actions under the control that opens it: items with their icon and keys, check rows,
//! labels and separators, and submenus beside their row. Its trigger is a button the app draws
//! ([`crate::Button`] with `opens_menu`); the menu is drawn last, over what is under it.
//!
//! In cells the popup is [`crate::popup`]'s. A submenu opens at its row's end, its first row level
//! with it. What lies under the open menu is drawn as it is (open-questions T2).

use crate::keys::{Key, KeyPress};
use crate::look::Look;
use crate::popup::{Popup, PopupEvent, PopupKind, PopupRow, handle, stops};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// A menu: its rows, and the rows of each submenu by the index of the row that opens it.
#[derive(Debug, Clone)]
pub struct DropdownMenu<'a> {
    look: &'a Look,
    rows: Vec<PopupRow>,
    subs: Vec<(usize, Vec<PopupRow>)>,
}

/// Whether the menu is open, and where its highlight stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuState {
    pub open: bool,
    pub highlight: usize,
    /// An open submenu: the row that opened it, and the highlight in it.
    pub sub: Option<(usize, usize)>,
}

/// What a key did to the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEvent {
    Ignored,
    Opened,
    Moved,
    /// A row's action: its index, and its index in a submenu. The menu closes, except on a check
    /// row, which turns over with the menu staying open.
    Action(usize, Option<usize>),
    /// Closed without an action: focus back on the trigger.
    Closed,
}

impl<'a> DropdownMenu<'a> {
    pub fn new(look: &'a Look, rows: impl IntoIterator<Item = PopupRow>) -> DropdownMenu<'a> {
        DropdownMenu {
            look,
            rows: rows.into_iter().collect(),
            subs: Vec::new(),
        }
    }

    /// The rows of the submenu the row at `row` opens; that row gets its chevron.
    pub fn submenu(mut self, row: usize, rows: impl IntoIterator<Item = PopupRow>) -> Self {
        if let Some(PopupRow::Item(i)) = self.rows.get_mut(row) {
            i.submenu = true;
        }
        self.subs.push((row, rows.into_iter().collect()));
        self
    }

    fn sub_rows(&self, row: usize) -> Option<&[PopupRow]> {
        self.subs
            .iter()
            .find(|(r, _)| *r == row)
            .map(|(_, rows)| rows.as_slice())
    }

    /// Answer a key: on the trigger while closed, in the menu while open.
    pub fn handle(&self, state: &mut MenuState, key: KeyPress) -> MenuEvent {
        if !state.open {
            let first = stops(&self.rows);
            return match key.key {
                Key::Down | Key::Enter | Key::Char(' ') if !key.ctrl && !key.alt => {
                    state.open = true;
                    state.highlight = first.first().copied().unwrap_or(0);
                    MenuEvent::Opened
                }
                Key::Up if !key.ctrl && !key.alt => {
                    state.open = true;
                    state.highlight = first.last().copied().unwrap_or(0);
                    MenuEvent::Opened
                }
                _ => MenuEvent::Ignored,
            };
        }
        if let Some((row, mut at)) = state.sub {
            let rows = self.sub_rows(row).unwrap_or(&[]);
            let ev = handle(rows, &mut at, key, true);
            state.sub = Some((row, at));
            return match ev {
                PopupEvent::Back => {
                    state.sub = None;
                    MenuEvent::Moved
                }
                PopupEvent::Closed => {
                    *state = MenuState::default();
                    MenuEvent::Closed
                }
                PopupEvent::Chosen(k) => self.chosen(state, rows, row, Some(k)),
                PopupEvent::Moved => MenuEvent::Moved,
                _ => MenuEvent::Ignored,
            };
        }
        match handle(&self.rows, &mut state.highlight, key, false) {
            PopupEvent::Moved => MenuEvent::Moved,
            PopupEvent::Closed => {
                *state = MenuState::default();
                MenuEvent::Closed
            }
            PopupEvent::OpenSub(row) => self.open_sub(state, row),
            PopupEvent::Chosen(row) if self.sub_rows(row).is_some() => self.open_sub(state, row),
            PopupEvent::Chosen(row) => self.chosen(state, &self.rows, row, None),
            _ => MenuEvent::Ignored,
        }
    }

    fn open_sub(&self, state: &mut MenuState, row: usize) -> MenuEvent {
        let first = self
            .sub_rows(row)
            .map(stops)
            .and_then(|s| s.first().copied())
            .unwrap_or(0);
        state.sub = Some((row, first));
        MenuEvent::Moved
    }

    fn chosen(&self, state: &mut MenuState, rows: &[PopupRow], row: usize, sub: Option<usize>) -> MenuEvent {
        let index = sub.unwrap_or(row);
        let check = matches!(rows.get(index), Some(PopupRow::Item(i)) if i.check.is_some());
        let event = MenuEvent::Action(
            if sub.is_some() {
                state.sub.map_or(row, |s| s.0)
            } else {
                row
            },
            sub,
        );
        if !check {
            *state = MenuState::default();
        }
        event
    }

    /// Draw the open menu for a trigger at `anchor`, in `window`.
    pub fn render(&self, window: Rect, anchor: Rect, buf: &mut Buffer, state: &MenuState) {
        if !state.open {
            return;
        }
        let mut popup = Popup::new(self.look, PopupKind::Menu, &self.rows).highlight(Some(state.highlight));
        popup.sub_open = state.sub.map(|s| s.0);
        if state.sub.is_some() {
            popup.highlight = None;
        }
        let area = popup.place(anchor, window, crate::cells::cols(128.0));
        (&popup).render(area, buf);
        if let Some((row, at)) = state.sub
            && let Some(rows) = self.sub_rows(row)
        {
            let sub = Popup::new(self.look, PopupKind::Menu, rows).highlight(Some(at));
            let w = sub.width().max(crate::cells::cols(96.0)).min(window.width);
            let h = sub.height();
            // At its row's end, its first row level with it; to the start when there is no room.
            let y = (area.y + 1 + row as u16)
                .saturating_sub(1)
                .min(window.bottom().saturating_sub(h));
            let x = if area.right() + w <= window.right() {
                area.right()
            } else {
                area.x.saturating_sub(w)
            };
            (&sub).render(Rect::new(x, y, w, h), buf);
        }
    }
}
