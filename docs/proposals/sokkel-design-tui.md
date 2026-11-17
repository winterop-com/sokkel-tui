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

## (c2) What building Button and TextField settled

How the first two parts read in cells, for the per-part cell note in point 2:

- **One row.** Every button size and the text field's control are one row. A size keeps its text
  role (`badge` for `xs` and `sm`, `action` above); padding is one column.
- **An edge in the pad cells.** A one-row control cannot draw a box, so its edge is a thin bar at
  each end (`▏Filter▕`, `|` in ASCII), in the paint's edge color, where the edge is not transparent.
- **The focus ring outside, in a reserved cell.** A focused button draws `┃` in the cell outside
  each end: half a cell off the box, as `focus-ring-offset` keeps the outline off it, and visible
  where the ring and the ground are one color (tidevann dark's default button). The width counts
  those cells, so focus never moves anything. Without color, focus is reversed.
- **The issue mark** takes three columns inside the control's end (`density.control-sm` is about
  three columns); its focus ring hugs the glyph. The critical and warning glyphs differ in shape, not
  only ink (`✖` and `▲`, `x` and `!` in ASCII).
- **Disabled.** `disabled-opacity` laid over the surface, as the flattening stand-in does. Without
  color, the faint attribute is the one cue left; the opaque disabled paints of point 1 would
  replace both.
- **A paint's ink of `inherit`** is the ink of the surface the part says it stands on (`on`).

And from PageState, PageHeader and Section:

- **Rows from px.** A vertical gap of 12 or more is a blank row, a smaller one none (`cells::rows`,
  18 px a row). So the page state's pieces 12 apart are a row apart and its lines 4 apart are not;
  a section's body padding is a row at regular density and none at compact.
- **A boxed section's head rule** takes a row of its own, joined to the box's sides (`├──┤`); a
  critical box draws its top line heavy in `critical`.
- **A missing paint.** `page-state`'s warning tone (`warning/8` ground, `warning/35` edge) has no
  resolved paint for its frame: `page-state.base.failed.frame` is the only failed frame. sokkel-tui
  draws a warning in the failed frame, its triangle and heading saying the tone. The resolver could
  add `page-state.base.failedWarning.frame` (and the mark's `warning-ink`), as the other tones are.
- **A page header's widths** read 512 px as 61 columns, 256 as 30 and 128 as 15, measured on the
  header's own width; the block's own width is its widest line, the description at most 65.

## (d) Where it lives

The proposals land in sokkel-design, as egui's did. sokkel-tui embeds the resolved file at a pinned
sokkel-design commit, fetched from GitHub. A Rust crate of the contract's types without egui in it
(egui's `tokens.rs` holds `egui::Color32` and `egui::Shadow` today) would serve both Rust renderers;
until then this crate reads the resolved JSON directly.

## (e) What does not fit a cell grid as written

1. **Hover.** Mouse motion is reported only when asked for, and many readers never move a mouse in a
   terminal. Hover paints are kept but nothing may rely on them.
2. **Keyboard-only focus.** Kept as the spec has it: the ring shows on keyboard focus, and a press
   of the pointer (where the terminal reports one) focuses without it. The keyboard is the main
   input, so the ring shows almost always.
3. **Motion.** Spinners and durations work (frames on `motion-fast` and `motion-base`); transitions
   between layouts do not.
4. **Scheme detection.** The terminal's background is asked for (OSC 11), never assumed; a terminal
   that does not answer takes the setting.
5. **Wide glyphs.** A glyph two cells wide (most emoji, CJK) moves every cell after it; widths must
   be measured, never counted in characters.
