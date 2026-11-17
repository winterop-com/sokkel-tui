//! 06: sokkel's frame around the runs table, run by the library's runtime: the rail, the top bar,
//! the status line, the drawer below md, the command palette (Ctrl+K) and the settings (`,`) with
//! the look and the frame in them, and the mouse. The example only gives its parts and its page;
//! everything the frame does is the library's (`app::run_shell`).
//!
//! Usage: cargo run --example 06_shell [-- light]
//! Keys: Ctrl+K the palette, `?` the shortcuts, `/` the search (it narrows the runs by job), `,`
//! the settings, Escape closes what is open; the mouse on the rail's
//! collapse, its entries, the menu button below md (90 columns: resize the terminal), the palette
//! button. Ctrl+Q quits.
//!
//! With arguments it writes one frame out as SVG and text, cell for cell, for `make specimens`:
//! cargo run --example 06_shell -- <theme> <light|dark> <cols> <rows> <out-without-extension> [collapsed] [drawer] [compact]

use ratatui::{Frame, layout::Rect};
use sokkel_tui::app::{self, Chrome, Cx, ShellApp};
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Align, Button, ButtonVariant, Cell, Column, ColumnWidth, Density, Fact, Look, NoteTone, PageHeader, Rail,
    RailEntry, RailFoot, RailGroup, Row, RowTone, Scheme, SortDir, StatusLine, Table, TableState, TopBar, snapshot,
};

struct RunRow {
    job: &'static str,
    state: &'static str,
    by: Option<&'static str>,
    project: &'static str,
    started: &'static str,
    took: Option<&'static str>,
}

#[rustfmt::skip]
const RUNS: &[RunRow] = &[
    RunRow { job: "nightly-export", state: "running", by: Some("ada"), project: "harbor", started: "12:04", took: None },
    RunRow { job: "sync-inventory", state: "failed", by: Some("grace"), project: "juniper", started: "11:58", took: Some("2m 41s") },
    RunRow { job: "rebuild-index", state: "succeeded", by: Some("linus"), project: "larkspur", started: "11:40", took: Some("14m 02s") },
    RunRow { job: "send-digest", state: "queued", by: None, project: "harbor", started: "11:39", took: None },
    RunRow { job: "import-orders", state: "succeeded", by: Some("margaret"), project: "juniper", started: "11:12", took: Some("48s") },
    RunRow { job: "nightly-export", state: "succeeded", by: Some("ada"), project: "harbor", started: "10:04", took: Some("6m 19s") },
    RunRow { job: "sync-inventory", state: "cancelled", by: Some("grace"), project: "juniper", started: "09:58", took: Some("1m 03s") },
    RunRow { job: "rebuild-index", state: "succeeded", by: Some("linus"), project: "larkspur", started: "09:40", took: Some("13m 55s") },
    RunRow { job: "send-digest", state: "waiting", by: Some("margaret"), project: "harbor", started: "09:39", took: None },
    RunRow { job: "import-orders", state: "succeeded", by: Some("ada"), project: "juniper", started: "09:12", took: Some("51s") },
    RunRow { job: "nightly-export", state: "succeeded", by: Some("ada"), project: "harbor", started: "08:04", took: Some("6m 02s") },
    RunRow { job: "sync-inventory", state: "succeeded", by: Some("grace"), project: "juniper", started: "07:58", took: Some("2m 12s") },
];

const SELECTED: usize = 1;

fn rail<'a>(look: &'a Look, collapsed: bool) -> Rail<'a> {
    Rail::new(look, "T", "Tidevann")
        .groups([
            RailGroup::new(
                Some("Workspace"),
                [
                    RailEntry::new("Runs").icon("▶").count("1,200").active(),
                    RailEntry::new("Jobs").icon("◆").count("5"),
                    RailEntry::new("Schedules").icon("◷"),
                    RailEntry::new("Hosts").icon("▣").unread(),
                ],
            ),
            RailGroup::new(
                Some("Projects"),
                [
                    RailEntry::new("harbor").icon("●"),
                    RailEntry::new("juniper").icon("●"),
                    RailEntry::new("larkspur").icon("●"),
                ],
            ),
        ])
        .collapsed(collapsed, true)
        .foot(RailFoot {
            letter: "a".into(),
            color: "kind-violet".into(),
            name: "ada".into(),
        })
}

/// The runs page in sokkel's frame, narrowed by the top bar's search.
#[derive(Default)]
struct Runs {
    query: String,
}

impl ShellApp for Runs {
    fn id(&self) -> Option<&str> {
        Some("tidevann")
    }

    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a> {
        Chrome {
            rail: rail(look, false),
            top: TopBar::new(look).trail(["Tidevann", "Runs"]).search(""),
            foot: StatusLine::new(look).note("Live", NoteTone::Live).live(true).facts([
                Fact::labeled("host", "api.example.org").code(),
                Fact::new("Updated 12s ago"),
                Fact::labeled("?", "Shortcuts"),
            ]),
        }
    }

    fn page(&mut self, f: &mut Frame, page: Rect, cx: &mut Cx) {
        let look = cx.look;
        let narrow = Width::of(f.area()) == Width::Narrow;
        let header = PageHeader::new(look, "Runs").count("1,200 runs").actions([
            Button::new(look, "Filter").variant(ButtonVariant::Secondary),
            Button::new(look, "New"),
        ]);
        let hh = header.height(page.width);
        f.render_widget(&header, Rect { height: hh, ..page });
        let gap = 1 + u16::from(look.density() == Density::Regular);
        let table = Rect {
            y: page.y + hh + gap - 1,
            height: page.height.saturating_sub(hh + gap - 1),
            ..page
        };
        draw_table(f, look, table, narrow, &self.query);
    }

    fn search(&mut self, query: &str) {
        self.query = query.to_lowercase();
    }
}

fn draw_table(f: &mut Frame, look: &Look, area: Rect, narrow: bool, query: &str) {
    // The table, from the core: the runs, sorted by when they started, the failed run's row in the
    // critical tone and the selected one active, as keyboard focus left it.
    let mut columns = vec![
        Column::new("Job").width(ColumnWidth::Fill(3)).sortable(),
        Column::new("State").sortable(),
        Column::new(if narrow { "By" } else { "Started by" }).sortable(),
    ];
    if !narrow {
        columns.push(Column::new("Project").width(ColumnWidth::Fill(2)).sortable());
    }
    columns.push(Column::new("Started").sortable());
    columns.push(Column::new("Took").align(Align::End));
    let rows: Vec<Row> = RUNS
        .iter()
        .filter(|r| r.job.contains(query))
        .map(|r| {
            let mut cells = vec![Cell::text(r.job), Cell::status(r.state), Cell::maybe(r.by)];
            if !narrow {
                cells.push(Cell::text(r.project));
            }
            cells.push(Cell::text(r.started));
            cells.push(Cell::maybe(r.took));
            let tone = if r.state == "failed" {
                RowTone::Critical
            } else {
                RowTone::Plain
            };
            Row::new(cells).tone(tone)
        })
        .collect();
    let started = columns.len() - 2;
    let table = Table::new(look, &columns, &rows).focused(true).total(1200);
    let mut state = TableState::default().sorted_by(started, SortDir::Descending);
    state.active = SELECTED;
    f.render_stateful_widget(&table, area, &mut state);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        return app::run_shell(Runs::default(), app::look_from_args("tidevann"));
    }
    let scheme = if args[2] == "light" {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let flag = |f: &str| args[6..].iter().any(|a| a == f);
    let mut look = Look::new(&args[1], scheme).unwrap_or_else(|| panic!("the contract has no theme {}", args[1]));
    if flag("compact") {
        look = look.with_density(Density::Compact);
    }
    let (cols, rows): (u16, u16) = (args[3].parse().unwrap(), args[4].parse().unwrap());
    let (collapsed, drawer) = (flag("collapsed"), flag("drawer"));
    let buffer = &app::shot(
        Runs::default(),
        look.clone(),
        (cols, rows),
        |shell| {
            shell.collapsed = collapsed;
            shell.drawer = drawer;
        },
        &[],
    );
    std::fs::write(
        format!("{}.svg", args[5]),
        snapshot::svg(buffer, look.token("background"), look.token("foreground")),
    )?;
    std::fs::write(format!("{}.txt", args[5]), snapshot::text(buffer))?;
    Ok(())
}
