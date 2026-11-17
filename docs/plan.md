# Plan

sokkel-tui draws the sokkel-design contract in a terminal with ratatui. The question this repository
answers: does the contract hold when a renderer has cells, one type size and no alpha, and what must
the contract say so that it does?

## Why ratatui

- **Immediate mode, like egui.** The contract was rewritten for egui: styling from context became
  explicit options, parts in flowing rows got fixed sizes, focus says how it was reached. ratatui
  reads those specs as they are.
- **Nothing to fight.** A part built from its spec reads its sizes, words and paints from the
  contract, so most drift cannot be written. A retained toolkit would bring its own keys, focus,
  palette and blending to undo first.
- **One Rust layer under two faces.** The contract's types, and later an app's model, can be shared
  with sokkel-egui: the same Tidevann app as a window and as a terminal.
- **Cheap to check.** Tests draw into a buffer: no GPU, no browser. A buffer is text, so a snapshot
  can be read in a diff.

## What is built

- `tokens`: sokkel-design's resolved tokens, index and glyphs, embedded, fetched at a pinned commit.
- `look`: a theme, scheme, density, color depth and character set; paints read by key in their
  opaque `flat` form and a disabled one in its `faded` form, a missing state drawn as `rest`; 256
  colors as `colorDepth.ansi256` (nearest of 16 to 255 by OKLab); corners from the radius ladder as
  rounded or square, in glyphs.json's box glyphs.
- `text`: text roles as `cells.textRoles` says (bold, italic).
- `cells`: px in columns, the breakpoints and check sizes from `cells`, blank rows by
  `cells.rules` (one for 12 px or more at regular, none at compact).
- `snapshot`: a buffer as SVG (for the eye and the side-by-side board) and as text (for tests).
- `glyphs`: glyphs.json's glyphs, icons and boxes in Unicode or ASCII; sokkel-tui's own only where
  the contract is open (the one-row edge, T3; the focus ring, T4).
- `keys`: keys as the parts read them, matched by the typed character; Mod is Ctrl, no Alt.
- `button` (step 1): every variant in rest, hover, focus, busy, reason, disabled, invalid and open;
  icon buttons with `pressed`, `destructiveHover`, an icon at either end. One row at every size; the
  edge in the pad cells; the focus ring in the cell beside each end; the tip for the status line.
  Disabled in its `faded` paint; a transparent ground draws none, keeping the cell's own.
- `text_field` (step 1): the frame (label, required star or word, hint or key hint, description,
  control, help, fallback), the issue mark in the control's end with critical and warning tones,
  its popover drawn last, editing with a cursor that scrolls the value, wide glyphs measured, read
  only and disabled. Not yet: multiline, `inRow`, the proposed busy state.
- `focus` (step 2): stops named each frame in tab order, Tab and Shift+Tab round the top layer, a
  layer that keeps Tab inside and gives focus back on close, a press of the pointer focusing the
  stop under it, the ring on keyboard focus only, and whether the reader is typing.
- `shortcuts` (step 2): a registry of typed characters, Mod (Ctrl) chords and two-key sequences
  (the second within a second), quiet in a field and under a layer unless the binding asks; never
  an Alt chord; labels in words that are fields (`Ctrl K`, `G then R`).
- `page_state` (step 3): loading, failed (headed by what the server answered, the diagnostic with
  its code parts on a chip, the problems, Try again), empty; framed or not; critical or warning.
- `page_header` (step 3): title or code, count, code, description, meta, a back button, the trail
  outside a shell, actions beside the title's foot or wrapped under it, items folding into More
  from the last with the primary kept, laid out by its own width.
- `section` (step 3): boxed with its head over a joined rule, or flat; meta, description, actions;
  the critical and graph tones; `flush`. Not yet: `fill` (a body that scrolls).
- `table` (step 4): the table variant: frame, head and its rule of tees, rows drawn only while in
  view, sorting by keys from the head row (ascending, descending, as given; a missing value last),
  selection with the box column and the head's box, row tones, zebra, the quiet dash, status cells
  by dot and word, the range in the foot; loading placeholders, the empty line, a failed read
  cards below 69 columns of its own box; the bar (a search and the count, the selection bar with
  Clear); the list variant (wider padding, cards as soon as its heads no longer fit); unframed. Not
  yet: groups, trees, details, row actions, paging, editing, resizing, reordering, the grid.
- `rail`, `top_bar`, `status_line`, `shell` (step 5): the rail's head, groups, entries (icon,
  label, count, unread dot, the active bar), collapsed form, foot cell and edge; the top bar's
  search field, trail, palette button with its keys and actions, and below md the menu button, the
  last crumb and the search icon; the status line's note with its live dot and tones, its facts,
  and a focused control's tip in the note's place (principle 23); `ShellLayout` placing them, the
  drawer below md. Not yet: the rail's parents and flyouts, descriptions, strip, body, resizing and
  reordering; the panel; what lies under the drawer (T2).
- `dialog`, `menu`, `select`, `popup` (step 6, first half): the dialog (header, body, the footer's
  verbs with the confirm last, a shut confirm's reason as its tip, `busy-release` and Stop
  waiting, the corner close, Escape held while busy, a sheet below md); the dropdown menu (items
  with icon and keys, labels, separators, check rows in a gutter, destructive and disabled rows,
  one level of submenu, typeahead); the select (field and quiet triggers, its list with details
  and the chosen row's check, typeahead while closed, read only, disabled).
- `palette`, `settings` (step 6, second half): the command palette (groups of commands with icon,
  hint or code, meta in its tone, kind and key caps, a disabled command's reason; a fuzzy filter
  over labels and keywords; a command that answers later holding it open until `busy-release`, then
  Stop waiting; the refusal; Ctrl+K and Escape closing it, not while a command runs) and the
  settings dialog (the column with its search and groups, the pane with each setting's control
  where the layout puts it, Reset to defaults asking first, below md a list then a section with
  Back). Not yet: the palette's app search and two-line rows and foot, radio groups, a dialog's
  header bar and status, the nested shade, and what lies under a layer (T2).
- The pointer, where the terminal reports it (`keys::Pointer`): the table says what a position
  hits (a row, its box, the head's box, a head, a card) and acts on a press; the menu, the select's
  list and the palette say which row is under it; every part already reports where its controls
  landed. The wheel moves the active row; hover highlights. A press focuses without the ring.
- `crates/core/examples/07_runs.rs` (step 7, `cargo run --example 07_runs`): Tidevann's runs in a real
  terminal: the shell, the runs and jobs pages, the table sorting and selecting by keys, a filter
  menu, a New run dialog whose confirm stays busy a moment, the palette (Ctrl+K), the shortcuts
  (`?`), the settings dialog with a select, the search (`/`), G then R and G then J, Tab through
  the stops with their tips in the status line, Ctrl+Q to quit. With `--frame` it draws one frame
  after scripted keys, for `make specimens`.
- `crates/core/examples/06_shell.rs`: the shell and the runs page, every part from the core, in every theme and
  scheme, below and above the md line, collapsed and as a drawer.
- `crates/core/examples/08_gallery/`: browsed in the terminal with no arguments; the parts sheets (`controls`, `page`, `table`, `overlays`, `palette`), every part in its variants and states, in
  color, in ASCII and without color.

## What comes next, in order

Each part is built from its spec in sokkel-design's `parts/`, with its words, states and keyboard,
then drawn by the spike in place of its hand-drawn copy.

| Step | Parts | Notes |
|---|---|---|
| 1 | Button, TextField (built) | Every other part uses them. Busy keeps its label with a spinner in the icon's place; a disabled button with a reason stays focusable and says the reason in the status line; a field's issue mark sits at its control's end |
| 2 | Focus and keys (built) | Tab order, the last input kind, a shortcuts registry matching typed characters, quiet in a field and under an overlay; Mod as Ctrl |
| 3 | PageState, PageHeader, Section (built) | Loading, failed (headed by what the server answered), empty, the content |
| 4 | Table (built, the table variant) | The one table: head, sort by keys, selection, tones, the quiet dash, the foot's range, drawing only the visible rows, cards below its own stack width |
| 5 | Shell: Rail, TopBar, StatusLine (built) | The drawer below about 90 columns, the trail in the top bar only, the account level with the foot |
| 6 | Overlays: Dialog, CommandPalette, Select, DropdownMenu, SettingsDialog (built) | A back stack for settings; focus returned on close; the palette on Ctrl+K, busy release after `busy-release` |
| 7 | An interactive example (built) | Tidevann's runs on the shell, run in a real terminal |

Tooltip has no floating box here: its words are said in the status line (proposal, point 7).
