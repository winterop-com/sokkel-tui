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
use ratatui::layout::{Position, Rect};
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

    /// Where the open menu and its open submenu stand, for a trigger at `anchor` in `window`.
    pub fn areas(&self, window: Rect, anchor: Rect, state: &MenuState) -> (Rect, Option<Rect>) {
        let popup = Popup::new(self.look, PopupKind::Menu, &self.rows);
        let area = popup.place(anchor, window, crate::cells::cols(128.0));
        let sub = state.sub.and_then(|(row, _)| {
            let rows = self.sub_rows(row)?;
            let sub = Popup::new(self.look, PopupKind::Menu, rows);
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
            Some(Rect::new(x, y, w, h))
        });
        (area, sub)
    }

    /// The row a pointer at `at` is on: its index, and its index in the open submenu. Only rows a
    /// press can choose: not a label, a separator or a disabled row.
    pub fn hit(&self, window: Rect, anchor: Rect, at: Position, state: &MenuState) -> Option<(usize, Option<usize>)> {
        if !state.open {
            return None;
        }
        let (area, sub) = self.areas(window, anchor, state);
        let row_in = |area: Rect, rows: &[PopupRow]| -> Option<usize> {
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
            stops(rows).contains(&k).then_some(k)
        };
        if let (Some(sa), Some((row, _))) = (sub, state.sub)
            && let Some(k) = row_in(sa, self.sub_rows(row).unwrap_or(&[]))
        {
            return Some((row, Some(k)));
        }
        row_in(area, &self.rows).map(|k| (k, None))
    }

    /// A press on what [`DropdownMenu::hit`] found: a submenu's row opens it, any other row acts.
    pub fn press(&self, state: &mut MenuState, hit: (usize, Option<usize>)) -> MenuEvent {
        match hit {
            (row, None) if self.sub_rows(row).is_some() => self.open_sub(state, row),
            (row, None) => {
                state.highlight = row;
                state.sub = None;
                self.chosen(state, &self.rows, row, None)
            }
            (row, Some(k)) => {
                let rows = self.sub_rows(row).unwrap_or(&[]);
                state.sub = Some((row, k));
                self.chosen(state, rows, row, Some(k))
            }
        }
    }

    /// The pointer over a row: the highlight follows it.
    pub fn hover(&self, state: &mut MenuState, hit: (usize, Option<usize>)) {
        match hit {
            (row, None) => {
                state.highlight = row;
                if state.sub.is_some_and(|(r, _)| r != row) {
                    state.sub = None;
                }
            }
            (row, Some(k)) => state.sub = Some((row, k)),
        }
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
        let (area, sub_area) = self.areas(window, anchor, state);
        (&popup).render(area, buf);
        if let (Some((row, at)), Some(sa)) = (state.sub, sub_area)
            && let Some(rows) = self.sub_rows(row)
        {
            let sub = Popup::new(self.look, PopupKind::Menu, rows).highlight(Some(at));
            (&sub).render(sa, buf);
        }
    }
}
