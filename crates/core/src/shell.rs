//! The shell, from sokkel-design `patterns/shell`: where the frame's parts go.
//!
//! The rail runs the window's full height at its start, its head as tall as the top bar and its
//! foot cell level with the status line, so head and bar, foot cell and status line read as one
//! strip each across the window. Below md (90 columns) the rail is a drawer over the content from
//! the start, drawn expanded. The page sits in the content at the page inset, a blank row on top at
//! regular (the pattern's `cells` note).

use crate::cells::{Width, by_density, page_inset};
use crate::look::Look;
use crate::status_line::StatusLine;
use crate::top_bar::TopBar;
use ratatui::layout::Rect;

/// Where the frame's parts go in a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLayout {
    /// The rail in the frame; `None` below md, where it is a drawer.
    pub rail: Option<Rect>,
    pub top: Rect,
    pub content: Rect,
    pub foot: Rect,
    pub width: Width,
}

impl ShellLayout {
    /// The frame in `window`, the rail `rail_width` columns wide where it stands in the frame.
    pub fn new(look: &Look, window: Rect, rail_width: u16) -> ShellLayout {
        let width = Width::of(window);
        let rail_w = if width == Width::Narrow {
            0
        } else {
            rail_width.min(window.width / 2)
        };
        let main = Rect {
            x: window.x + rail_w,
            width: window.width - rail_w,
            ..window
        };
        let top_h = TopBar::height(look).min(main.height);
        let foot_h = StatusLine::height(look).min(main.height.saturating_sub(top_h));
        ShellLayout {
            rail: (rail_w > 0).then_some(Rect {
                width: rail_w,
                ..window
            }),
            top: Rect { height: top_h, ..main },
            content: Rect {
                y: main.y + top_h,
                height: main.height - top_h - foot_h,
                ..main
            },
            foot: Rect {
                y: main.bottom() - foot_h,
                height: foot_h,
                ..main
            },
            width,
        }
    }

    /// The drawer below md: from the window's start, the rail's expanded width, at most the
    /// window less four columns.
    pub fn drawer(window: Rect, rail_width: u16) -> Rect {
        Rect {
            width: rail_width.min(window.width.saturating_sub(4)),
            ..window
        }
    }

    /// The page in the content: the page inset across, a blank row on top at regular.
    pub fn page(&self, look: &Look) -> Rect {
        let inset = page_inset(look, self.content);
        let top = by_density(look, 1, 0);
        Rect {
            x: self.content.x + inset,
            y: self.content.y + top,
            width: self.content.width.saturating_sub(2 * inset),
            height: self.content.height.saturating_sub(top),
        }
    }
}
