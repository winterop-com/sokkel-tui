# Changelog

sokkel-tui follows semantic versioning before 1.0 as Cargo reads it: a breaking change raises the
minor version (0.2 to 0.3), anything else the patch. Each release names the sokkel-design contract
it draws (`sokkel-design.json`). Consumers take a release by its tag:

```toml
sokkel-tui = { git = "https://github.com/winterop-com/sokkel-tui", tag = "v0.2.0" }
```

## Unreleased

### Changed

- Draws sokkel-design contract 0.35.0 (627e6bc), from 0.32.2. The settings and the palette offer
  its new theme, `darkroom`. Its lighter type weights change no cell: a cell is bold or not by
  `cells.textRoles`, which did not move. Its text sizes (0.34.0) have nothing to draw in a terminal,
  which has one type size. The parts new since 0.32.2 (chip, markdown-view, call-tile, the map and
  tracking parts, status-wall, alert-ticker and the kiosk pattern) are not built yet.

### Fixed

- The status line keeps each fact whole but the last, whose value alone gives way, cut short with
  an ellipsis (parts/status-line, contract 0.33.6). It dropped whole facts from the start when they
  did not fit, so a long last fact took every fact with it.

## 0.3.1 (2026-11-21)

### Fixed

- The top bar's trail takes the room left before the palette's button and the actions: it was laid
  out without them, so the button was drawn over its current crumb. Crumbs that do not fit give way
  together and are cut with an ellipsis, as sokkel-ui's do.
- The palette's button follows parts/top-bar: beside the search field it shows its keys alone
  (`Ctrl K`), and with no field it is 224 px wide with the magnifier, Search and the keys. It
  showed "Open the command palette" with the keys in both cases, which took the trail's room.

## 0.3.0 (2026-11-20)

### Breaking

- `Ran` has two more answers, `Suspend` and `Quit`: a `match` on it needs arms for them.
- `DetailPane` paints no ground of its own (sokkel-ui's paints none): it stands on what it is drawn
  on, the side panel's ground or a dialog's. An app that drew it on a bare buffer gives it a ground.

### Added

- `ShellApp::kept_path`: where the app's look is kept, in place of `kept_path(id)` (sokkel-ui's
  `setAppearanceStorage`).
- `Ran::Suspend` and `ShellApp::suspended`: a command hands the terminal over (a child process) and
  the runtime takes it back with the frame as it was. `Program::wants_outside` and
  `Program::outside` do the same for a bare `run`.
- `Ran::Quit`: a command ends `run_shell`.
- `ShellApp::take_command`: the app asks the runtime to run a command from its own keys or presses
  (a dialog's button), answered as a shortcut's is.
- `app::OPEN_PALETTE`: the palette's command id, to bind to a key of the app's or to ask for.
- `ShellApp::panel_rows`: a side panel taller than the window scrolls (the wheel, focus kept in
  view, a scroller), as the side panel's content does.
- `Scroll`: a scroll view an app keeps, for content taller than its room (a detail pane in a
  dialog's body), with the wheel, PageUp, PageDown, Home, End and presses moved to match.
- `Table::stack_below`, `Table::min_width` and `Table::least_width`: a table stacks below the
  smaller of its `stackBelow` (69) and its columns' width, as sokkel-ui's `stacksAt`; a table whose
  columns fit a narrow box stays a table with a small `min_width`.

### Changed

- A shortcut's or an account row's command now honors its `Ran` answer (`Suspend`, `Quit`).

## 0.2.0 (2026-11-19)

The first release. It draws sokkel-design contract 0.32.2 (8f0cc81).

### The contract

- Tokens, glyphs and the cells layout read at a pinned sokkel-design commit (`make tokens`,
  `make tokens-check`); every color a resolved paint in its opaque form.
- Truecolor, 256 and 16 colors (a role map: critical red, warning yellow, good green, info blue,
  reverse video for hover and selection) and no color; Unicode and ASCII glyphs.
- The scrim under a layer; text roles; the density.

### The runtime

- `ShellApp` and `run_shell`: the terminal taken and given back, a frame each `motion-fast`, the
  mouse, bracketed paste, the look kept between runs under the web's keys.
- The frame: the rail (groups, parents and flyouts, descriptions, status dots, a strip, an edge
  that resizes it, fitting its labels, giving way to a page that needs the room, labels cut in the
  middle), the top bar with its search, the status line, a banner, full screen, the drawer below
  md, a side panel.
- One focus through the frame and the page; Tab, Escape back, Left and Right back and forward
  along the routes where nothing on the page claims them; shortcuts and the `?` list.
- The command palette (the app's commands and its own search, refusals, commands that keep
  running, Stop waiting) and the settings dialog, adjusted from inside.

### The parts

- Button and button groups; text fields (masked, multiline, in a row, busy, paste); select; dialog;
  dropdown menu; prompt (`patterns/prompt`: empty, refused, busy, failed); page state, page header,
  section; detail pane; status (`Status`, the contract's health and run states).
- The table: sorting, selection, the bar, the list variant, cards below its stack width, groups,
  row actions, details, trees, paging and Load more, cell editing, resizable columns, heads that
  wrap, a middle cut.
- The dashboard: the panel frame, the stat panel, the time series panel (stacked, stepped).

### Examples

- `01_basic` to `09_dashboard`, a progression up to whole apps on the runtime; `08_gallery` browses
  every sheet and writes them out for `make specimens`.

### Breaking, for those who took a commit before this release

- `PointerKind` is `#[non_exhaustive]`, with `Drag` and `Release`: a match over it needs a wildcard.
- `SelectState` has `by_pointer`: one built by its fields needs `..Default::default()`.
- A prompt's Submit is shut while its field is empty (`Prompt::allow_empty` for the old behavior),
  and it is a box at every width, never a sheet.
