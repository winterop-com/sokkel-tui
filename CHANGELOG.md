# Changelog

sokkel-tui follows semantic versioning before 1.0 as Cargo reads it: a breaking change raises the
minor version (0.2 to 0.3), anything else the patch. Each release names the sokkel-design contract
it draws (`sokkel-design.json`). Consumers take a release by its tag:

```toml
sokkel-tui = { git = "https://github.com/winterop-com/sokkel-tui", tag = "v0.2.0" }
```

## Unreleased

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
