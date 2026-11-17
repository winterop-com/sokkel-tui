//! sokkel's design system drawn in a terminal with ratatui.
//!
//! The counterpart of sokkel-ui (`@winterop-com/sokkel-ui`) and sokkel-egui (`sokkel-egui`): the same
//! contract, sokkel-design, drawn in cells. This crate holds the foundations every part reads:
//! - [`tokens`]: sokkel-design's resolved file, embedded.
//! - [`look`]: a theme, scheme, density and color depth, and the paints drawn in it.
//! - [`text`]: text roles as a cell can show them.
//! - [`cells`]: px in columns and rows, and the breakpoints in columns.
//! - [`snapshot`]: a buffer as SVG and as text.
//!
//! The parts come next, each from its spec in sokkel-design's `parts/` (`docs/plan.md`).

pub mod cells;
pub mod look;
pub mod snapshot;
pub mod text;
pub mod tokens;

pub use look::{Depth, Look, Rgba};
pub use text::TextRole;
pub use tokens::{Density, Scheme};
