//! A scroll view: content taller than its room, drawn whole off screen and shown from its top row,
//! with a scroller in the room's last column. The shell's content scrolls this way (patterns/shell,
//! `content`), a side panel's tab content does (parts/side-panel, `content`), and so does a
//! dialog's body (parts/dialog, `body`): an app draws a [`crate::DetailPane`] or any other part
//! taller than its room through a [`Scroll`] it keeps.
//!
//! The scroller is a `scroll-track` column in `border` with a `scroll-thumb` in
//! `muted-foreground`, one column wide (the terminal's `scrollbar-width`); content that fits takes
//! the whole room and draws no scroller. The wheel moves three rows, PageUp and PageDown a view
//! less a row, Home and End to the ends.

use crate::keys::{Key, KeyPress, Pointer, PointerKind};
use crate::look::Look;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;

/// The rows the wheel moves.
const WHEEL: u16 = 3;

/// A scroll view's state, kept by the app between frames: its top row, and where it was last drawn
/// and how tall its content was, so the wheel, keys and presses can be answered between frames.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Scroll {
    /// The content's row shown at the top of the view.
    pub top: u16,
    view: Rect,
    rows: u16,
}

impl Scroll {
    /// Where it was last drawn.
    pub fn view(&self) -> Rect {
        self.view
    }

    /// Whether the content was taller than its view when last drawn.
    pub fn overflows(&self) -> bool {
        self.rows > self.view.height
    }

    /// The last row the top can be.
    fn end(&self) -> u16 {
        self.rows.saturating_sub(self.view.height)
    }

    /// The content's room when drawn at `area` with `rows` rows: the area less its last column
    /// where the content overflows, and as tall as the content.
    pub fn content_area(area: Rect, rows: u16) -> Rect {
        if rows > area.height {
            Rect::new(area.x, area.y, area.width.saturating_sub(1), rows)
        } else {
            area
        }
    }

    /// Draw content `rows` tall in `area` with `content`, which draws in the room it is given
    /// (taller than `area` where it overflows) as it would on screen. The view keeps the ground
    /// already under it, and the top stays within the content.
    pub fn render(
        &mut self,
        look: &Look,
        buf: &mut Buffer,
        area: Rect,
        rows: u16,
        content: impl FnOnce(&mut Buffer, Rect),
    ) {
        let area = area.intersection(buf.area);
        self.view = area;
        self.rows = rows;
        self.top = self.top.min(self.end());
        if !self.overflows() {
            content(buf, area);
            return;
        }
        let room = Scroll::content_area(area, rows);
        let mut off = Buffer::empty(room);
        // The ground under the view, carried down the whole content.
        for x in room.x..room.right() {
            let under = buf[(x, area.y)].style();
            let ground = Style::new().bg(under.bg.unwrap_or_default());
            off.set_style(Rect::new(x, room.y, 1, room.height), ground);
        }
        content(&mut off, room);
        for y in 0..area.height {
            for x in room.x..room.right() {
                buf[(x, area.y + y)] = off[(x, room.y + y + self.top)].clone();
            }
        }
        self.scroller(look, buf, area.right().saturating_sub(1));
    }

    /// The scroller in column `x`: the track the view's height, the thumb as long as the view is
    /// of the content, placed by the top.
    fn scroller(&self, look: &Look, buf: &mut Buffer, x: u16) {
        let (h, rows) = (self.view.height as u32, self.rows as u32);
        let len = (h * h / rows.max(1)).max(1) as u16;
        let at = (self.top as u32 * (self.view.height - len) as u32 / self.end().max(1) as u32) as u16;
        let glyphs = look.glyphs();
        for k in 0..self.view.height {
            let (g, c) = if (at..at + len).contains(&k) {
                (glyphs.scroll_thumb, look.color("muted-foreground"))
            } else {
                (glyphs.scroll_track, look.color("border"))
            };
            buf[(x, self.view.y + k)].set_symbol(g).set_fg(look.terminal(c));
        }
    }

    /// The wheel over the view moves it: true where it was over the view and the content
    /// overflows (the top may stay at an end).
    pub fn pointer(&mut self, p: Pointer) -> bool {
        if !self.overflows() || !self.view.contains(p.at) {
            return false;
        }
        match p.kind {
            PointerKind::ScrollUp => self.top = self.top.saturating_sub(WHEEL),
            PointerKind::ScrollDown => self.top = (self.top + WHEEL).min(self.end()),
            _ => return false,
        }
        true
    }

    /// PageUp, PageDown, Home and End move the view: true where the key was one of them and the
    /// content overflows.
    pub fn key(&mut self, key: KeyPress) -> bool {
        if !self.overflows() || key.ctrl {
            return false;
        }
        let page = self.view.height.saturating_sub(1).max(1);
        self.top = match key.key {
            Key::PageDown => (self.top + page).min(self.end()),
            Key::PageUp => self.top.saturating_sub(page),
            Key::Home => 0,
            Key::End => self.end(),
            _ => return false,
        };
        true
    }

    /// Where a point on screen falls in the content (to hand a press to the part drawn in it, at
    /// the room [`Scroll::content_area`] gave): None outside the view or on the scroller.
    pub fn at(&self, at: Position) -> Option<Position> {
        let room = Scroll::content_area(self.view, self.rows);
        let shown = Rect::new(room.x, self.view.y, room.width, self.view.height);
        shown.contains(at).then(|| Position::new(at.x, at.y + self.top))
    }

    /// Bring content rows `y..y + height` (a focused control) into view.
    pub fn reveal(&mut self, y: u16, height: u16) {
        let room = self.view.y;
        let top = self.top + room;
        if y < top {
            self.top -= top - y;
        } else if y + height > top + self.view.height {
            self.top = (self.top + (y + height - (top + self.view.height))).min(self.end());
        }
    }
}
