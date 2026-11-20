# sokkel-design: the terminal side

What a terminal renderer needs from the contract, from building the spike in this repository, in the
shape the egui side's proposal took. Nothing here changes sokkel-design. The owner decides.

The spike's pictures are in `docs/assets/`: tidevann dark at 120 and 80 columns, tidevann light at
120, and the terminal theme at 120, drawn from the contract's resolved paints with the stand-ins
below.

## (a) What the terminal needs first

The same order as egui's, since the shell is the frame every page sits in: Button and TextField,
then focus and keys, then page state, page header and section, the table, the shell, the overlays.
A terminal has no pointer to fall back on, so every part's `keyboard` list must be complete; gaps
show up here first.

## (b) What does not map to cells

| Contract | In a terminal |
|---|---|
| Lengths in px, density | A control is one row whatever its px height. Density changes blank rows and side padding, never a row's height |
| `breakpoint-md` 768, `breakpoint-lg` 1024 | About 90 and 120 columns (at about 8.4 px a column). An 80x24 terminal draws the drawer form; from 120 the panel is a column. A phone is SSH from a phone or a tmux split |
| Alpha in paints | 86 of tidevann light's 470 paints carry partial alpha in a ground, edge or ink (`button.default.hover` is `#006aa0cc`). A terminal cannot blend, and principle 20 forbids mixing in the renderer |
| `disabled-opacity` | A terminal's faint attribute varies by terminal; a disabled ink needs to be a color |
| The radius ladder | One bit: rounded or square box corners. It survives (the terminal theme's surface radius is 0, ocean's 8, indigo's 10) |
| Text roles | One size. A role keeps its weight as bold; size, line height and tracking are not drawn. `statistic` and the hero roles could take big digits |
| Focus outline with an offset | No room outside a cell. The spike marks a focused row with a lead bar in `ring` |
| Elevations, the scrim | No shadows; a popup is its edge. The scrim covers cells of any color, so it cannot be resolved ahead of time |
| Tooltip | No floating box under a pointer that may not exist; see point 7 |
| Lucide icons | A terminal cannot choose its font. `typeface-terminal` names a Nerd Font, but an icon needs a Unicode and an ASCII fallback |
| Mod | Cmd almost never reaches a terminal program. Mod is Ctrl |
| Finger targets | Do not apply |

## (c) What the contract could hold

1. **Opaque paints.** Each piece says the surface it stands on (`standsOn`: `background`,
   `popover`, `sidebar`, `card`), and the resolver writes each paint flattened over it, with the
   disabled ink flattened at `disabled-opacity`, for every theme and scheme, checked like the rest.
   This replaces `flat` in `look.rs`.
2. **Cells per part.** Per size and density: rows, side padding in columns, and how its width is
   found. The breakpoints and check sizes in columns in `index.json`.
3. **Text roles in cells.** One table, role to bold, italic and ink, so every terminal renderer sets
   them alike. Today `text.rs` uses "bold from 500 up".
4. **Color depth.** Truecolor as resolved; for 16 colors, the ANSI map sokkel-ui's terminal package
   holds (`TERMINAL_COLORS` in `packages/terminal/src/look.ts`) made a contract value, so a sokkel
   terminal program in 16 colors looks native in a sokkel app's `TerminalView`; for no color
   (`NO_COLOR`), states said by words and glyphs, as principle 11 already asks. The nearest-of-256
   in `look.rs` is a stand-in until a 256 rule exists.
5. **Character sets.** The glyphs a part draws (sort arrows, chevrons, the spinner's frames, box
   corners, the status dot, the missing dash), each with a Unicode and an ASCII form, and a Nerd
   Font form for icons.
6. **Status per renderer.** A renderer field per part (`renderers: { egui: ..., tui: ... }`), so a
   third renderer neither blocks a stable part nor is left out of it.
7. **Tips in the status line.** A renderer without floating tips says a tooltip's words, and a
   disabled control's reason, in the status line while the control has focus.
8. **Keys.** Principle 17 gains: Mod is Ctrl in a terminal; no Alt chords; no `[` or `]` as a part's
   primary key, since with Option as Meta a Norwegian keyboard cannot type them.
9. **The scrim.** One rule for what is under a layer in cells: drawn faint, or each cell's colors
   flattened under the scrim at run time as the one allowed blend.

## (c2) What landed, and what is still asked

sokkel-design 0cd3072 (sokkel-design #31, tokens from sokkel-ui #659) holds points 1 to 9: opaque
`flat` and `faded` paints over each piece's `standsOn`, `cells` in the index and per part, the text
roles in cells, `colorDepth` with the `ansi-*` tokens and the OKLab rule for 256, glyphs.json,
`renderers` per part, principle 23 (tips in the status line) and principle 17's terminal note.
sokkel-tui reads all of it at that pin; its two stand-ins are gone. Still open there: T1 (sixteen
colors), T2 (the scrim), T5 (the egui renderer's name); T3 and T4 the owner has decided (below).

From building Button, TextField, PageState, PageHeader and Section on it:

1. **T3, decided by the owner (2026-11-17): b.** The end bars (`▏Filter▕`, brackets in ASCII) in
   the edge's flat color, inside the padding. glyphs.json could hold them as `edge-start` and
   `edge-end`.
2. **T4, decided by the owner (2026-11-17).** A focused one-row control (a button, a field's mark)
   draws a bar (`┃`, `>` and `<` in ASCII) in `ring` in the cell beside each end, outside its
   width, in the column the parts' cells keep between controls: half a cell off the box, as
   `focus-ring-offset` keeps it off, and visible where the ring and the ground are one color
   (tidevann dark's default button). Without color, reverse video. A row of a table or a list takes
   `focus-mark` in `ring` at its start (recommendation a). glyphs.json could hold the bar as
   `focus-ring-start` and `focus-ring-end`.
3. **A part on another surface.** `flat` is resolved over a piece's `standsOn` default. A ghost
   button in a section's head, an outline one in a popover, stands elsewhere: its hover ground's
   flat form is then off by the difference. sokkel-tui draws a fully transparent ground as no ground
   (the cell keeps its own), which is exact, but has no answer for a translucent hover there. The
   resolver could write `flat` per surface a part names (`standsOn` listing `card`, `popover`,
   `section-head-ground`), or the specs could name the surface a part's actions stand on.
4. **A missing paint.** `page-state`'s warning tone (`warning/8` ground, `warning/35` edge) has no
   resolved frame: `page-state.base.failed.frame` is the only failed frame. sokkel-tui draws a
   warning in the failed frame, its triangle and heading saying the tone.
5. **A critical section's rule.** The web's 2 px `critical` rule across the box's top has no cell
   form; sokkel-tui draws the top line heavy (`━`, `=` in ASCII). glyphs.json could hold a heavy
   rule beside `rule`.
6. **A graph's dot.** The section's dot grid has no glyph in glyphs.json; sokkel-tui draws `·`.

## (c3) The overlays in cells

dialog, dropdown-menu, select, command-palette and settings-dialog carry no `cells` note at
0cd3072. How sokkel-tui reads them, for
the notes they could take:

- **A popup** (a menu, a select's list) is a box whose border is its edge; its 4 px padding is no
  cell and its rows' 6 px a column. Every row is one row. While it holds a check row, a gutter of
  two columns holds the check. A separator is a row of the rule glyph; a group's label a row in the
  help role. It opens under its trigger, over it when there is no room under; a submenu at its
  row's end, its first row level with the row.
- **A dialog** is centered: `sm` 46 columns, `md` 61, `lg` 80, `xl` 107, `full` the window less 4,
  at most the window less 4 across and 2 down. Its 16 px padding is two columns across and a row
  down at regular; its pieces a blank row apart at regular. The footer is a rule row and a row of
  verbs on the footer's ground. Below md it is a sheet filling the window, no edge.
- **The command palette** is the dialog's box, 46 columns (384 px) a third of the way down; below
  md the window's width less 4, a row from the top. The search is a row on its tint; a group's title
  a row; each command one row, its keys as caps (a sequence as `G then R`); the list at most 60
  percent of the window.
- **The settings dialog** is 107 by 39 cells (896 by 704 px), at most the window less 4 across and
  2 down: a head row over a rule, a 27-column column (224 px) under a rule at its end, the pane
  padded two columns, settings ruled apart, a control at the row's end from 61 columns of pane
  (512 px) and under its words below that, the dialog's footer. Below md a list, then a section
  with Back, each filling the window.
- **What lies under them** is drawn as it is, waiting on T2.
- **A select's focus.** The field variant shows focus by its focus edge, as a text field does; the
  quiet one, which draws no edge, by the bars beside it (T4).

## (c4) The dashboard in cells

The chart family (dashboard-panel, stat-panel, time-series-panel, chart, at 9095168) has no `cells`
notes yet. sokkel-tui draws this reading of it, and asks for it, or a better one, as their notes:

1. **The grid.** Twelve columns of the grid's own width, panels a column apart and rows a row apart
   (none at compact). Wide from 1024 px (122 columns), tablet-wide from 640 px (76 columns),
   phone-wide below, as the spans already say.
2. **The panel.** A box in `panel` ground and edge; the head one row (title, subtitle a column after
   it, the stale mark, the tools at its end, 3 columns each) over the head's rule; the body padded
   two columns across and a row down at regular (one and none at compact), none when flush; the
   foot a rule and a row. Loading draws the spinner and `Loading` in the content's place: a cell has
   no half strength, so the content cannot stay under the bar.
3. **The stat tile.** A box; the head a row (the title, the change at its end, which drops what it
   is against before the title is cut); the value in `text.statistic` (text is one size, so large is
   its weight) with the unit a column after; a note or the stale mark; the meter two rows (a track
   of `rule-heavy`, the reading's color as far as it goes, `muted` past it, the tick at the target,
   then `Target` and its figure); the sparkline the foot row. The tile's height in rows follows
   what it holds, not 112 px.
4. **Glyphs asked for.** A sparkline's eight levels (`▁▂▃▄▅▆▇█`, ASCII `_.-=+*#@`) and the stale mark
   (`◷`, ASCII `!`), drawn as sokkel-tui's own until glyphs.json holds them.
5. **The time series.** Lines in braille, two by four dots a cell (`*` a cell in ASCII), each
   value's dot joined to the next; a dashed reference every other pair of dots. One series with an
   area draws it as block columns in eighths of a cell; several series draw their lines alone,
   since overlapping areas cannot share a cell. The value labels at the plot's start (the middle
   one only with a row of its own), grid lines across in `grid-dot`, the axis line under the plot
   and the category labels under it, as many as fit. The legend a row of swatches (`■`) and names,
   or a table of min, max, avg and current. Stacked series draw a column of bands a category, a boundary inside a cell drawn by the block's
   ink (the lower series) over its ground (the upper one); a stepped line holds its value, then jumps.
6. **The trend's faintness.** The web draws the spark area at low alpha; a cell draws the levels in
   the reading's color (or `muted-foreground` with no reading), opaque.

## (c5) A banner part

The shell pattern names a `banner` slot (between the top bar and the content, not scrolling, for a
state that holds) and sokkel-ui has a `Banner` in four tones, but the contract has no banner part
and no banner paints. sokkel-tui draws one row there, as its reading:

1. **Grounds.** The web's tone at 12 percent is a mix a renderer may not make. sokkel-tui stands a
   critical banner on page-state's `failed` frame ground and a warning one on its `failed+warning`
   frame, the nearest resolved tints; info and good stand on `card`. Banner paints in the contract
   (ground, edge and ink per tone, flat) would settle it.
2. **The row.** The tone's mark (`circle-alert`, `triangle-alert`, `info`, `check`), then the words
   in the label role and the tone's ink, cut short; when cut, the whole of them is the status
   line's tip. It takes no row while there is none, so the page moves down a row when it shows.
3. **Asked for:** a `banner` part with its words, tones, paints and a `cells` note; whether a banner
   carries actions (Retry) or a dismiss in a terminal.

## (c6) A masked field, and a prompt

parts/text-field has no masked input. sokkel-tui draws one as its reading: `masked`, each character
the `bullet` glyph, a column each, the value never on screen, one line; a paste lands in it (control
characters and newlines left out). A spec could say whether a masked field offers a reveal, and its
words. sokkel-tui also puts parts/dialog and parts/text-field together as a `Prompt` (one field,
Submit and Cancel, Enter submits, Escape cancels): a pattern rather than a part, which the contract
could name.

## (c7) Health states

The status vocabulary (`status-<state>`) holds a run's states. A health check's own (ok, warn, fail,
error) has none, and an app that maps them onto run words prints "succeeded" where an operator
expects OK. sokkel-tui lets an app name its state and give a tone (`Cell::toned`, `StatusTone`:
good, warning, critical, info, quiet), its dot and word in the tone's tokens, and a rail entry carry
such a dot (`RailEntry::status`), its word in the status line on focus. Asked for: health states in
the vocabulary (`status-ok`, `-warn`, `-fail`, `-error`), or a tone a status may name.

## (c8) The terminal handed over, a pane in a dialog

Asked by an app on sokkel-tui 0.2.0; read against contract 0.32.2 and sokkel-ui.

1. **The terminal handed over.** A terminal app runs a child process (a game, an editor, a pager)
   that needs the terminal while it runs. No other renderer has this: a web page and an egui window
   never give up their screen. sokkel-tui takes it as runtime, not look: a command answers
   `Ran::Suspend`, the runtime leaves raw mode and the alternate screen, the app runs the child in
   `ShellApp::suspended`, and the runtime takes the terminal back with the frame as it was (the
   rail, the route and its history, focus, the open layer). Nothing is drawn meanwhile, so the
   contract need not say anything; it could note, under patterns/shell's renderer-specific notes,
   that a terminal renderer hands the terminal over this way and keeps the frame's state.
2. **A command the app asks for.** sokkel-ui's `useShell().openPalette` and an app's own handlers
   run anything from a press. sokkel-tui's app asks with `ShellApp::take_command` (a dialog's button
   pressed), and the runtime runs it as a shortcut runs it; `OPEN_PALETTE` is the palette's own id
   for that and for a key of the app's (the palette part keeps Mod+K and turns the list's Ctrl+P
   off, as written). Nothing for the contract.
3. **A detail pane in a dialog.** parts/detail-pane says the pane stands on the side panel's
   `sidebar` ground; sokkel-ui's `DetailPane` paints no ground at all, so in a dialog it stands on
   `popover`. sokkel-tui follows sokkel-ui: the pane has no ground of its own, and it scrolls in its
   room as the side panel's content and the dialog's body do. Below `breakpoint-lg` patterns/shell
   makes the panel a sheet rather than a dialog, which is the contract's own answer to a narrow
   window. Asked for: whether a detail pane may stand in a dialog's body, and if so whether its
   sections keep their 16 of padding inside the body's own 16, or drop it.

## (d) Where it lives

The proposals land in sokkel-design, as egui's did. sokkel-tui embeds the resolved file at a pinned
sokkel-design commit, fetched from GitHub. A Rust crate of the contract's types without egui in it
(egui's `tokens.rs` holds `egui::Color32` and `egui::Shadow` today) would serve both Rust renderers;
until then this crate reads the resolved JSON directly.

## (e) What does not fit a cell grid as written

1. **Hover.** Mouse motion is reported only when asked for, and many readers never move a mouse in a
   terminal. Hover paints are kept but nothing may rely on them.
   sokkel-tui asks for it (mouse capture) in its apps, with a way to turn it off: while it is on,
   the terminal's own selection needs a modifier held. A spec could say whether a terminal app
   captures the mouse by default.
2. **Keyboard-only focus.** Kept as the spec has it: the ring shows on keyboard focus, and a press
   of the pointer (where the terminal reports one) focuses without it. The keyboard is the main
   input, so the ring shows almost always.
3. **Motion.** Spinners and durations work (frames on `motion-fast` and `motion-base`); transitions
   between layouts do not.
4. **Scheme detection.** The terminal's background is asked for (OSC 11), never assumed; a terminal
   that does not answer takes the setting.
5. **Wide glyphs.** A glyph two cells wide (most emoji, CJK) moves every cell after it; widths must
   be measured, never counted in characters.
