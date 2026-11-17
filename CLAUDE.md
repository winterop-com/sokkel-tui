# sokkel-tui

sokkel's design system drawn in a terminal with ratatui: the third renderer of the sokkel-design
contract, after sokkel-ui (React) and sokkel-egui (egui). Early stage; `docs/plan.md` says what is
built and what comes next, `docs/proposals/sokkel-design-tui.md` what the terminal asks of the
contract.

## Layout

- A cargo workspace shaped like sokkel-egui's. `crates/core` is the package `sokkel-tui`: tokens,
  the look, text roles, cells, snapshots, and the parts as they come. Add-ons come later as crates
  of their own; the core never depends on an add-on. No empty placeholder crates.
- `examples/spike` draws the shell and the table by hand from the paints, to SVG and text. Each
  part it draws by hand moves into the core, and the spike then draws it from there.
- `examples/parts` draws the parts sheets (`controls`, `page`, one per group of parts): every part
  the core holds, in its variants and states. A new part or state goes on a sheet.
- An example uses only the library's public parts; a part it lacks goes into the library.

## The contract

- `crates/core/tokens/tokens.resolved.json` is sokkel-design's, fetched from GitHub by `make tokens`
  at the commit `sokkel-design.json` pins (`make tokens REF=<commit>` moves the pin). Never edit it,
  and never copy it from a local checkout: that can carry unfinished work. `make tokens-check`
  fails when it is not the pinned commit's.
- sokkel-ui, sokkel-design and sokkel-egui are read, never written: each has its own sessions. The
  specs are at https://winterop-com.github.io/sokkel-design/. A question for the contract, or a
  change it might want, is written into `docs/proposals/` here. The owner decides.
- Every color a part draws is a resolved paint (`look.paint(key, field, over)`) or a color token,
  never a literal and never mixed here (principle 20). Two stand-ins are allowed until the contract
  holds them, each marked in `look.rs`: flattening alpha over the surface a piece stands on, and the
  nearest of 256 colors.
- Words are the spec's keys with its English defaults; keyboard behavior is the spec's `keyboard`
  list. Text goes through `TextRole`, never a bold of its own.
- Keys are matched by the character they type. Mod is Ctrl in a terminal. No Alt chords, and no
  `[` or `]` as a part's primary key: with Option as Meta a Norwegian keyboard cannot type them.

## Rules carried over from sokkel-ui

- Never write the name of an app or product that uses the library. Examples use the fictional
  Tidevann workspace only (jobs nightly-export, sync-inventory, rebuild-index, send-digest,
  import-orders; run states; people ada, grace, linus, margaret; projects harbor, juniper,
  larkspur; hosts on example.org).
- Words a part says are fields with English defaults. American English. No emojis.
- Fix the part, never the example.

## Gates

`make all` (fmt-check, clippy with warnings denied, every test). After a visual change run
`make specimens` and look at the PNGs before calling it done; a terminal picture is checked by eye,
in both schemes, at 80 and at 120 columns at least. Conventional commits, no attribution lines.
Work on a branch with a PR; ask before any other outward or hard-to-undo action.
