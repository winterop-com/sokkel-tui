//! px in cells.
//!
//! The contract writes lengths in px. A cell is about 8.4 px wide at a 14 px glyph, and a row about
//! one line of text. Lengths are not divided blindly: a control is one row whatever its px height,
//! and density changes blank rows and side padding, never a row's height. These are this
//! renderer's readings until the contract holds cells of its own.

use crate::look::Look;
use ratatui::layout::Rect;

/// The px a column stands for, for the readings below.
pub const PX_PER_COL: f64 = 8.4;

/// Below this many columns the window is drawn as a phone draws it: `breakpoint-md` (768).
pub const MD_COLS: u16 = 90;
/// From this many columns the shell's side panel is a column: `breakpoint-lg` (1024).
pub const LG_COLS: u16 = 120;

/// The terminal sizes every specimen is drawn at, as `checkWidths` are for a window.
pub const CHECK_SIZES: [(u16, u16); 4] = [(60, 20), (80, 24), (120, 40), (200, 50)];

/// What the window's width makes of the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Width {
    /// Below `MD_COLS`: the rail is a drawer, dialogs fill the screen.
    Narrow,
    /// `MD_COLS` up to `LG_COLS`: the rail in the frame, the panel a sheet.
    Medium,
    /// From `LG_COLS`: the panel is a column.
    Wide,
}

impl Width {
    pub fn of(area: Rect) -> Width {
        if area.width < MD_COLS {
            Width::Narrow
        } else if area.width < LG_COLS {
            Width::Medium
        } else {
            Width::Wide
        }
    }
}

/// A horizontal px length in whole columns, at least one where the px is not zero.
pub fn cols(px: f64) -> u16 {
    if px <= 0.0 {
        0
    } else {
        ((px / PX_PER_COL).round() as u16).max(1)
    }
}

/// The page's side inset: `density.page-inset-wide` when the content box is 640 px wide or more,
/// else `-narrow`, in columns.
pub fn page_inset(look: &Look, content: Rect) -> u16 {
    let wide = content.width as f64 * PX_PER_COL >= 640.0;
    cols(look.density_px(if wide { "page-inset-wide" } else { "page-inset-narrow" })).min(2)
}

/// Blank rows between a page's sections: one at regular, none at compact.
pub fn page_gap(look: &Look) -> u16 {
    match look.density() {
        crate::tokens::Density::Regular => 1,
        crate::tokens::Density::Compact => 0,
    }
}
