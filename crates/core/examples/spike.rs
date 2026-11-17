//! The spike: sokkel's shell and the runs table, every part drawn from the core, written out as
//! SVG and text, cell for cell. Static: no input yet (step 7 makes it a program).
//!
//! Usage: cargo run --example spike -- <theme> <light|dark> <cols> <rows> <out-without-extension> [collapsed] [drawer] [compact]

use ratatui::{Frame, Terminal, backend::TestBackend, layout::Rect, style::Style};
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Align, Button, ButtonVariant, Cell, Column, ColumnWidth, Density, Fact, Look, NoteTone, PageHeader, Rail,
    RailEntry, RailFoot, RailGroup, Row, RowTone, Scheme, ShellLayout, SortDir, StatusLine, Table, TableState, TopBar,
    snapshot,
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

/// What the specimen shows besides the plain frame.
#[derive(Default)]
struct Show {
    collapsed: bool,
    drawer: bool,
}

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

fn draw(f: &mut Frame, look: &Look, show: &Show) {
    let window = f.area();
    f.buffer_mut().set_style(
        window,
        Style::new().bg(look.token("background")).fg(look.token("foreground")),
    );
    let rail = rail(look, show.collapsed);
    let shell = ShellLayout::new(look, window, rail.width());
    if let Some(r) = shell.rail {
        f.render_widget(&rail, r);
    }
    let top = TopBar::new(look)
        .trail(["Tidevann", "Runs"])
        .window(shell.width)
        .search("");
    f.render_widget(&top, shell.top);
    let foot = StatusLine::new(look).note("Live", NoteTone::Live).live(true).facts([
        Fact::labeled("host", "api.example.org").code(),
        Fact::new("Updated 12s ago"),
        Fact::labeled("?", "Shortcuts"),
    ]);
    f.render_widget(&foot, shell.foot);

    let page = shell.page(look);
    let narrow = shell.width == Width::Narrow;
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
    draw_table(f, look, table, narrow);

    // Below md the rail is a drawer over the content, drawn expanded.
    if show.drawer && narrow {
        let expanded = self::rail(look, false);
        f.render_widget(&expanded, ShellLayout::drawer(window, expanded.width()));
    }
}

fn draw_table(f: &mut Frame, look: &Look, area: Rect, narrow: bool) {
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!(
            "usage: cargo run --example spike -- <theme> <light|dark> <cols> <rows> <out-without-extension> [collapsed] [drawer] [compact]"
        );
        std::process::exit(2);
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
    let show = Show {
        collapsed: flag("collapsed"),
        drawer: flag("drawer"),
    };
    let (cols, rows): (u16, u16) = (args[3].parse().unwrap(), args[4].parse().unwrap());
    let mut term = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    term.draw(|f| draw(f, &look, &show)).unwrap();
    let buffer = term.backend().buffer();
    let ground = look.token("background");
    let ink = look.token("foreground");
    std::fs::write(format!("{}.svg", args[5]), snapshot::svg(buffer, ground, ink)).unwrap();
    std::fs::write(format!("{}.txt", args[5]), snapshot::text(buffer)).unwrap();
}
