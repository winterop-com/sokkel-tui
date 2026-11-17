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

- `tokens`: the resolved file, embedded, fetched at a pinned sokkel-design commit.
- `look`: a theme, scheme, density and color depth; paints read by key, a missing state drawn as
  `rest`; alpha flattened over the named surface (a stand-in); the nearest of 256 colors (a
  stand-in); corners from the radius ladder as rounded or square.
- `text`: text roles as bold from weight 500 up.
- `cells`: px in columns, the breakpoints as about 90 and 120 columns, the check sizes (60x20,
  80x24, 120x40, 200x50), the page inset and gap.
- `snapshot`: a buffer as SVG (for the eye and the side-by-side board) and as text (for tests).
- `glyphs`: every glyph a part draws in a Unicode and an ASCII set (a stand-in, proposal point 5).
- `keys`: keys as the parts read them, matched by the typed character; Mod is Ctrl, no Alt.
- `button` (step 1): every variant in rest, hover, focus, busy, reason, disabled, invalid and open;
  icon buttons with `pressed`, `destructiveHover`, an icon at either end. One row at every size; the
  edge in the pad cells; the focus ring in a reserved cell outside each end; the tip for the status
  line. Disabled is faded over the surface at `disabled-opacity` (with the flattening stand-in).
- `text_field` (step 1): the frame (label, required star or word, hint or key hint, description,
  control, help, fallback), the issue mark in the control's end with critical and warning tones,
  its popover drawn last, editing with a cursor that scrolls the value, wide glyphs measured, read
  only and disabled. Not yet: multiline, `inRow`, the proposed busy state.
- `examples/spike`: the shell and the table drawn by hand from the paints, in every theme and
  scheme, below and above the md line; its page header's buttons come from the core.
- `examples/parts`: the parts sheet, every part in its variants and states, in color, in ASCII and
  without color.

## What comes next, in order

Each part is built from its spec in sokkel-design's `parts/`, with its words, states and keyboard,
then drawn by the spike in place of its hand-drawn copy.

| Step | Parts | Notes |
|---|---|---|
| 1 | Button, TextField (built) | Every other part uses them. Busy keeps its label with a spinner in the icon's place; a disabled button with a reason stays focusable and says the reason in the status line; a field's issue mark sits at its control's end |
| 2 | Focus and keys | Tab order, the last input kind, a shortcuts registry matching typed characters, quiet in a field and under an overlay; Mod as Ctrl |
| 3 | PageState, PageHeader, Section | Loading, failed (headed by what the server answered), empty, the content |
| 4 | Table | The one table: head, sort by keys, selection, tones, the quiet dash, the foot's range, drawing only the visible rows, cards below its own stack width |
| 5 | Shell: Rail, TopBar, StatusLine | The drawer below about 90 columns, the trail in the top bar only, the account level with the foot |
| 6 | Overlays: Dialog, CommandPalette, Select, DropdownMenu, SettingsDialog | A back stack for settings; focus returned on close; the palette on Ctrl+K, busy release after `busy-release` |
| 7 | An interactive example | Tidevann's runs on the shell, run in a real terminal |

Tooltip has no floating box here: its words are said in the status line (proposal, point 7).
