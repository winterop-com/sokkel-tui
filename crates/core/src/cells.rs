//! px in cells, as `cells` in sokkel-design's index says.
//!
//! A column stands for `pxPerColumn` px at a 14 px glyph, and a row holds one line of text. A
//! vertical length is never divided: a control, a row, a bar and a head is one row whatever its px
//! height, and room between stacked pieces is whole blank rows. Density changes blank rows and side
//! padding, never a row's height. Each part's own `cells` note says its rows, padding and width.

use crate::look::Look;
use crate::tokens::{Density, index};
use ratatui::layout::Rect;

/// The px a column stands for (`cells.pxPerColumn`).
pub const PX_PER_COL: f64 = 8.4;

/// Below this many columns the window is drawn as a phone draws it (`cells.breakpoints.md`).
pub const MD_COLS: u16 = 90;
/// From this many columns the shell's side panel is a column (`cells.breakpoints.lg`).
pub const LG_COLS: u16 = 120;

/// The terminal sizes every specimen is drawn at (`cells.checkSizes`).
pub const CHECK_SIZES: [(u16, u16); 4] = [(60, 20), (80, 24), (120, 40), (200, 50)];

/// The contract's own numbers, to check the constants against.
pub fn contract_px_per_col() -> f64 {
    index()["cells"]["pxPerColumn"].as_f64().unwrap_or(PX_PER_COL)
}

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

/// Room between stacked pieces, in blank rows: under 12 px none; from 12 px one at regular and
/// none at compact (`cells.rules`), unless the part's cells say otherwise.
pub fn gap_rows(look: &Look, px: f64) -> u16 {
    u16::from(px >= 12.0 && look.density() == Density::Regular)
}

/// A value a part's cells give per density: `regular` or `compact`.
pub fn by_density(look: &Look, regular: u16, compact: u16) -> u16 {
    match look.density() {
        Density::Regular => regular,
        Density::Compact => compact,
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
    gap_rows(look, look.density_px("page-gap"))
}
