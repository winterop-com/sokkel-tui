//! sokkel's design system drawn in a terminal with ratatui.
//!
//! The counterpart of sokkel-ui (`@winterop-com/sokkel-ui`) and sokkel-egui (`sokkel-egui`): the same
//! contract, sokkel-design, drawn in cells. The foundations every part reads:
//! - [`tokens`]: sokkel-design's resolved file, embedded.
//! - [`look`]: a theme, scheme, density, color depth and character set, and the paints drawn in it.
//! - [`text`]: text roles as a cell can show them, and text measured in cells.
//! - [`cells`]: px in columns and rows, and the breakpoints in columns.
//! - [`glyphs`]: the glyphs the parts draw, in Unicode and in ASCII.
//! - [`keys`]: keys as the parts read them.
//! - [`snapshot`]: a buffer as SVG and as text.
//!
//! The parts, each from its spec in sokkel-design's `parts/` (`docs/plan.md`):
//! - [`button`]: [`Button`], with words, an icon and words, or an icon alone.
//! - [`text_field`]: [`TextField`], the field frame around a one-line input.

pub mod button;
pub mod cells;
pub mod glyphs;
pub mod keys;
pub mod look;
pub mod snapshot;
pub mod text;
pub mod text_field;
pub mod tokens;

pub use button::{Button, ButtonSize, ButtonVariant, IconPlace, Tip};
pub use glyphs::Charset;
pub use keys::{Key, KeyPress};
pub use look::{Depth, Look, Rgba};
pub use text::TextRole;
pub use text_field::{FieldEvent, FieldFocus, FieldState, FieldWords, Issue, RequiredMark, TextField, Tone};
pub use tokens::{Density, Scheme};
