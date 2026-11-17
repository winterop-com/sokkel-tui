//! The spike: sokkel's shell and table drawn from sokkel-design's resolved paints, written out as
//! SVG and text, cell for cell. Static: no input yet. The parts it draws by hand here move into
//! the core crate one by one (`docs/plan.md`), and this example then draws them from there: the
//! page header and the table so far.
//!
//! Usage: sokkel-tui-spike <theme> <light|dark> <cols> <rows> <out-without-extension>

use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use sokkel_tui::cells::{Width, page_inset};
use sokkel_tui::{
    Align, Button, ButtonVariant, Cell, Column, ColumnWidth, Look, PageHeader, Row, RowTone, Scheme, SortDir, Table,
    TableState, TextRole, snapshot,
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

/// Spans at the start and at the end of one row, the room between them blank.
fn spread(left: Vec<Span<'static>>, right: Vec<Span<'static>>, width: u16) -> Line<'static> {
    let lw: usize = left.iter().map(Span::width).sum();
    let rw: usize = right.iter().map(Span::width).sum();
    let mut spans = left;
    spans.push(Span::raw(" ".repeat((width as usize).saturating_sub(lw + rw))));
    spans.extend(right);
    Line::from(spans)
}

fn draw(f: &mut Frame, look: &Look) {
    let area = f.area();
    let page = Style::new()
        .bg(look.fill("shell.base.rest.page", "ground"))
        .fg(look.fill("shell.base.rest.page", "ink"));
    f.buffer_mut().set_style(area, page);

    let narrow = Width::of(area) == Width::Narrow;
    let [rail, main] =
        Layout::horizontal([Constraint::Length(if narrow { 0 } else { 24 }), Constraint::Fill(1)]).areas(area);
    if !narrow {
        draw_rail(f, look, rail);
    }
    let [top, rule, body, foot_rule, foot] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(main);
    draw_top_bar(f, look, top, rule, narrow);
    draw_page(f, look, body, narrow);
    draw_status_line(f, look, foot_rule, foot);
}

fn draw_rail(f: &mut Frame, look: &Look, area: Rect) {
    const FRAME: &str = "rail.base.rest.frame";
    let on = |key: &str, field: &str| look.fill(key, field);
    let block = Block::new()
        .borders(Borders::RIGHT)
        .border_style(Style::new().fg(on(FRAME, "edge")))
        .style(Style::new().bg(look.fill(FRAME, "ground")));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mark = Style::new()
        .bg(on("rail.base.rest.mark", "ground"))
        .fg(on("rail.base.rest.mark", "ink"))
        .add_modifier(Modifier::BOLD);
    let name = TextRole::Label.style(look).fg(on("rail.base.rest.name", "ink"));
    let group = TextRole::Help.style(look).fg(on("rail.base.rest.groupTitle", "ink"));
    let entry = Style::new().fg(on("rail.base.rest.entry", "ink"));
    let active = TextRole::Label
        .style(look)
        .bg(on("rail.base.active.entry", "ground"))
        .fg(on("rail.base.active.entry", "ink"));
    let active_edge = active
        .fg(on("rail.base.active.entry", "edge"))
        .remove_modifier(Modifier::BOLD);
    let count = Style::new().fg(on("rail.base.rest.count", "ink"));

    let w = inner.width as usize;
    let row = |label: &'static str, n: &'static str, lead: Span<'static>, st: Style, n_st: Style| {
        let pad = w.saturating_sub(3 + label.chars().count() + n.len() + 1);
        Line::from(vec![
            Span::styled(" ", st),
            lead,
            Span::styled(" ", st),
            Span::styled(label, st),
            Span::styled(" ".repeat(pad), st),
            Span::styled(n, n_st),
            Span::styled(" ", st),
        ])
    };
    let plain = |label: &'static str, n: &'static str| row(label, n, Span::raw(" "), entry, count);

    let mut lines = vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(" T ", mark),
            Span::raw(" "),
            Span::styled("Tidevann", name),
        ]),
        Line::raw(""),
        Line::from(Span::styled(" Workspace", group)),
        row("Runs", "1,200", Span::styled("▎", active_edge), active, active),
        plain("Jobs", "5"),
        plain("Schedules", ""),
        plain("Hosts", ""),
        Line::raw(""),
        Line::from(Span::styled(" Projects", group)),
        plain("harbor", ""),
        plain("juniper", ""),
        plain("larkspur", ""),
    ];
    while lines.len() < (inner.height as usize).saturating_sub(2) {
        lines.push(Line::raw(""));
    }
    // The foot cell, level with the status line: its rule and the account.
    lines.push(Line::from(Span::styled(
        "─".repeat(w),
        Style::new().fg(on("status-line.base.rest.strip", "rule")),
    )));
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            " a ",
            Style::new().bg(look.token("kind-violet")).fg(look.token("on-fill")),
        ),
        Span::styled(" ada", Style::new().fg(on("rail.base.rest.name", "ink"))),
    ]));
    f.render_widget(Paragraph::new(lines), inner);
}

fn draw_top_bar(f: &mut Frame, look: &Look, area: Rect, rule: Rect, narrow: bool) {
    const BAR: &str = "top-bar.base.rest.bar";
    let on = |key: &str, field: &str| look.fill(key, field);
    let bar = Style::new().bg(look.fill(BAR, "ground"));
    f.buffer_mut().set_style(area, bar);
    f.buffer_mut().set_style(rule, bar);
    let crumb = Style::new().fg(on("top-bar.base.rest.crumb", "ink"));
    let current = Style::new().fg(on("top-bar.base.current.crumb", "ink"));
    let chevron = Style::new().fg(on("top-bar.base.rest.chevron", "ink"));
    let menu = Style::new().fg(on("top-bar.base.rest.menu", "ink"));
    let palette = Style::new()
        .bg(on("top-bar.base.rest.palette", "ground"))
        .fg(on("top-bar.base.rest.palette", "ink"));
    let keys = Style::new()
        .bg(on("top-bar.base.rest.keys", "ground"))
        .fg(on("top-bar.base.rest.keys", "ink"));
    let field = Style::new().bg(look.token("field")).fg(look.token("muted-foreground"));

    let line = if narrow {
        // Below md: the menu button opens the rail as a drawer; the trail keeps its last crumb.
        spread(
            vec![
                Span::raw(" "),
                Span::styled("≡", menu),
                Span::raw("  "),
                Span::styled("Runs", current),
            ],
            vec![Span::styled("Ctrl K", palette), Span::raw(" ")],
            area.width,
        )
    } else {
        spread(
            vec![
                Span::raw(" "),
                Span::styled(" / ", field),
                Span::styled("Search            ", field),
                Span::raw("   "),
                Span::styled("Tidevann", crumb),
                Span::styled(" › ", chevron),
                Span::styled("Runs", current),
            ],
            vec![
                Span::styled(" Open the command palette ", palette),
                Span::styled(" Ctrl K ", keys),
                Span::raw(" "),
            ],
            area.width,
        )
    };
    f.render_widget(Paragraph::new(line), area);
    f.render_widget(
        Paragraph::new("─".repeat(rule.width as usize)).style(Style::new().fg(on(BAR, "rule"))),
        rule,
    );
}

fn draw_page(f: &mut Frame, look: &Look, body: Rect, narrow: bool) {
    let inset = page_inset(look, body);
    let inner = Rect {
        x: body.x + inset,
        y: body.y + 1,
        width: body.width - inset * 2,
        height: body.height - 1,
    };
    let [head, _, table] =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1), Constraint::Fill(1)]).areas(inner);

    // The page header, from the core: the title, its count, the actions with the primary last.
    let header = PageHeader::new(look, "Runs").count("1,200 runs").actions([
        Button::new(look, "Filter").variant(ButtonVariant::Secondary),
        Button::new(look, "New"),
    ]);
    f.render_widget(&header, head);
    draw_table(f, look, table, narrow);
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

fn draw_status_line(f: &mut Frame, look: &Look, rule: Rect, area: Rect) {
    const STRIP: &str = "status-line.base.rest.strip";
    let on = |key: &str, field: &str| look.fill(key, field);
    let ground = Style::new().bg(look.fill(STRIP, "ground"));
    f.buffer_mut().set_style(rule, ground);
    f.buffer_mut().set_style(area, ground);
    f.render_widget(
        Paragraph::new("─".repeat(rule.width as usize)).style(Style::new().fg(on(STRIP, "rule"))),
        rule,
    );
    let live = Style::new().fg(on("status-line.base.toneLive.note", "ink"));
    let item = Style::new().fg(on("status-line.base.rest.item", "ink"));
    let label = Style::new().fg(on("status-line.base.rest.itemLabel", "ink"));
    let code = Style::new().fg(on("status-line.base.code.item", "ink"));
    let line = spread(
        vec![
            Span::raw(" "),
            Span::styled("● Live", live),
            Span::raw("   "),
            Span::styled("host ", label),
            Span::styled("api.example.org", code),
        ],
        vec![
            Span::styled("Updated 12s ago", item),
            Span::raw("   "),
            Span::styled("? ", code),
            Span::styled("Shortcuts ", item),
        ],
        area.width,
    );
    f.render_widget(Paragraph::new(line), area);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        eprintln!("usage: sokkel-tui-spike <theme> <light|dark> <cols> <rows> <out-without-extension>");
        std::process::exit(2);
    }
    let scheme = if args[2] == "light" {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let look = Look::new(&args[1], scheme).unwrap_or_else(|| panic!("the contract has no theme {}", args[1]));
    let (cols, rows): (u16, u16) = (args[3].parse().unwrap(), args[4].parse().unwrap());
    let mut term = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    term.draw(|f| draw(f, &look)).unwrap();
    let buffer = term.backend().buffer();
    let ground = look.token("background");
    let ink = look.token("foreground");
    std::fs::write(format!("{}.svg", args[5]), snapshot::svg(buffer, ground, ink)).unwrap();
    std::fs::write(format!("{}.txt", args[5]), snapshot::text(buffer)).unwrap();
}
