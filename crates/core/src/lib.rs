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
//! - [`focus`]: which stop has focus, how it was reached, and the layers that hold it.
//! - [`shortcuts`]: the app's keys, matched by the character they type.
//! - [`snapshot`]: a buffer as SVG and as text.
//!
//! The runtime: [`app`], one screen ([`app::run`]) or an app in sokkel's frame
//! ([`app::run_shell`]), the palette, the settings, the drawer and the mouse kept for it.
//!
//! The parts, each from its spec in sokkel-design's `parts/` (`docs/plan.md`):
//! - [`button`]: [`Button`], with words, an icon and words, or an icon alone.
//! - [`text_field`]: [`TextField`], the field frame around a one-line input.
//! - [`page_state`]: [`PageState`], a read that is loading, failed or empty.
//! - [`page_header`]: [`PageHeader`], a page's head.
//! - [`section`]: [`Section`], a titled part of a page.
//! - [`table`]: [`Table`], the one table.
//! - [`rail`], [`top_bar`], [`status_line`]: the frame's parts, and [`shell`] where they go.
//! - [`dialog`], [`menu`], [`select`]: the overlays, over the popup [`popup`] they share.
//! - [`banner`]: [`Banner`], a state that holds, across the top of the content.
//! - [`dashboard`]: the dashboard grid, [`DashboardPanel`] and [`StatPanel`].
//! - [`prompt`]: [`Prompt`], a dialog asking for one value.
//! - [`palette`]: the command palette; [`settings`]: the settings dialog.

pub mod app;
pub mod banner;
pub mod button;
pub mod cells;
pub mod dashboard;
pub mod detail_pane;
pub mod dialog;
pub mod focus;
pub mod glyphs;
pub mod keys;
pub mod look;
pub mod menu;
pub mod page_header;
pub mod page_state;
pub mod palette;
pub mod popup;
pub mod prompt;
pub mod rail;
pub mod section;
pub mod select;
pub mod settings;
pub mod shell;
pub mod shortcuts;
pub mod snapshot;
pub mod status;
pub mod status_line;
pub mod table;
pub mod text;
pub mod text_field;
pub mod tokens;
pub mod top_bar;

pub use banner::{Banner, BannerTone};
pub use button::{Button, ButtonGroup, ButtonSize, ButtonVariant, IconPlace, Tip};
pub use dashboard::{
    ColorMode, DashboardPanel, Fill, Legend, PanelLayout, PanelRead, PanelTool, PanelWords, Series, Span, StatPanel,
    Threshold, TimeSeriesPanel,
};
pub use detail_pane::{DetailLayout, DetailPane, PaneSection};
pub use dialog::{Confirm, Dialog, DialogEvent, DialogFocus, DialogLayout, DialogWidth, DialogWords};
pub use focus::{Focus, FocusId, InputKind};
pub use glyphs::Charset;
pub use keys::{Key, KeyPress, Pointer, PointerKind};
pub use look::{Depth, Look, Rgba};
pub use menu::{DropdownMenu, MenuEvent, MenuState};
pub use page_header::{HeaderLayout, PageHeader, PageHeaderWords};
pub use page_state::{Outcome, PageState, PageStateWords, StateTone};
pub use palette::{
    Command, CommandGroup, CommandPalette, CommandTone, PaletteEvent, PaletteHit, PaletteState, PaletteWords,
};
pub use popup::{Item, PopupRow};
pub use prompt::{Prompt, PromptEvent, PromptFocus, PromptState};
pub use rail::{Rail, RailEntry, RailFoot, RailGroup, RailLayout, RailWords};
pub use section::{Section, SectionTone, SectionVariant};
pub use select::{Select, SelectEvent, SelectOption, SelectState, SelectVariant};
pub use settings::{
    Choice, Setting, SettingsDialog, SettingsEvent, SettingsFocus, SettingsGroup, SettingsLayout, SettingsSection,
    SettingsState, SettingsWords,
};
pub use shell::{Shell, ShellEvent, ShellLayout, ShellState};
pub use shortcuts::{Binding, KeyContext, Keys, ShortcutWords, Shortcuts};
pub use status::Status;
pub use status_line::{Fact, NoteTone, StatusLine};
pub use table::{
    Align, Cell, CellValue, Column, ColumnWidth, Row, RowTone, SortDir, StatusTone, Table, TableBar, TableEvent,
    TableHit, TableState, TableVariant, TableWords,
};
pub use text::{Plural, TextRole};
pub use text_field::{FieldEvent, FieldFocus, FieldState, FieldWords, Issue, RequiredMark, TextField, Tone};
pub use tokens::{Density, Scheme};
pub use top_bar::{TopBar, TopBarFocus, TopBarLayout, TopBarWords};
