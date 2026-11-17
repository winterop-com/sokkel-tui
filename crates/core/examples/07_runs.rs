//! 07: Tidevann's runs, the whole app on the library's runtime: the frame with its palette,
//! settings, account menu, shortcuts, search and drawer are the runtime's (`app::run_shell`). The
//! app gives its places, its two pages (runs and jobs), its commands and shortcuts, and its own
//! layers: a Filter menu and a New run dialog whose confirm stays busy a moment.
//!
//! Usage: cargo run --example 07_runs [-- light] [-- --frame <cols> <rows> <out> [keys...]]
//!
//! Keys: Tab moves through the rail, the search, the palette button, the page and the account;
//! Ctrl+K the palette; ? the shortcuts; / the search (it narrows the runs by job); G then R and G
//! then J go to a page, Escape back; on Jobs, Left and Right choose a cell and Enter or typing edits a
//! Schedule (Enter saves, Tab saves and moves on, Escape drops it); Ctrl+K searches the runs too (the app's search, answering after
//! 400 ms), a run chosen there opening its detail beside the page; `m` (or a press on a row's ⋯) opens a run's actions; in the table Up, Down, Home, End, PageUp, PageDown, Space selects. The
//! mouse works on every row, button, entry and menu row; the wheel scrolls. Ctrl+Q quits.
//!
//! With `--frame` it draws one frame after the keys given (`tab`, `down`, `ctrl-k`, `enter`, a
//! character) into `<out>.svg` and `<out>.txt`: a specimen.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use sokkel_tui::app::{self, Chrome, Cx, Flow, Ran, SearchAnswer, ShellApp};
use sokkel_tui::{
    Align, Binding, Button, ButtonVariant, Cell, Column, ColumnWidth, Command, CommandGroup, Confirm, Density,
    DetailPane, Dialog, DialogEvent, DialogFocus, DialogWidth, DropdownMenu, Fact, FocusId, Item, Key, KeyPress, Keys,
    Look, MenuEvent, MenuState, NoteTone, PageHeader, PaneSection, Pointer, PointerKind, PopupRow, Rail, RailEntry,
    RailFoot, RailGroup, Row, RowTone, Scheme, StatusLine, Table, TableEvent, TableHit, TableState, TableWords,
    TextRole, TopBar, snapshot,
};

#[derive(Clone)]
struct Run {
    job: String,
    state: String,
    by: Option<String>,
    project: String,
    started: String,
    took: Option<(String, f64)>,
}

fn run(job: &str, state: &str, by: Option<&str>, project: &str, started: &str, took: Option<(&str, f64)>) -> Run {
    Run {
        job: job.into(),
        state: state.into(),
        by: by.map(String::from),
        project: project.into(),
        started: started.into(),
        took: took.map(|(s, v)| (s.into(), v)),
    }
}

fn runs() -> Vec<Run> {
    vec![
        run("nightly-export", "running", Some("ada"), "harbor", "12:04", None),
        run(
            "sync-inventory",
            "failed",
            Some("grace"),
            "juniper",
            "11:58",
            Some(("2m 41s", 161.0)),
        ),
        run(
            "rebuild-index",
            "succeeded",
            Some("linus"),
            "larkspur",
            "11:40",
            Some(("14m 02s", 842.0)),
        ),
        run("send-digest", "queued", None, "harbor", "11:39", None),
        run(
            "import-orders",
            "succeeded",
            Some("margaret"),
            "juniper",
            "11:12",
            Some(("48s", 48.0)),
        ),
        run(
            "nightly-export",
            "succeeded",
            Some("ada"),
            "harbor",
            "10:04",
            Some(("6m 19s", 379.0)),
        ),
        run(
            "sync-inventory",
            "cancelled",
            Some("grace"),
            "juniper",
            "09:58",
            Some(("1m 03s", 63.0)),
        ),
        run(
            "rebuild-index",
            "succeeded",
            Some("linus"),
            "larkspur",
            "09:40",
            Some(("13m 55s", 835.0)),
        ),
        run("send-digest", "waiting", Some("margaret"), "harbor", "09:39", None),
        run(
            "import-orders",
            "succeeded",
            Some("ada"),
            "juniper",
            "09:12",
            Some(("51s", 51.0)),
        ),
        run(
            "nightly-export",
            "succeeded",
            Some("ada"),
            "harbor",
            "08:04",
            Some(("6m 02s", 362.0)),
        ),
        run(
            "sync-inventory",
            "succeeded",
            Some("grace"),
            "juniper",
            "07:58",
            Some(("2m 12s", 132.0)),
        ),
    ]
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Runs,
    Jobs,
}

const FILTER: &str = "filter";
const NEW: &str = "new";
const TABLE: &str = "table";

/// Tidevann's runs: what the app keeps, and nothing of the frame's.
struct Runs {
    page: Page,
    runs: Vec<Run>,
    query: String,
    failed_only: bool,
    table: TableState,
    jobs: TableState,
    menu: MenuState,
    /// A run's actions while open: its row, the menu, and where it opens.
    row_menu: Option<(usize, MenuState, Rect)>,
    /// What the last action did, for the status line.
    said: Option<String>,
    /// Schedules edited in the jobs table, by (row, column).
    job_edits: std::collections::BTreeMap<(usize, usize), String>,
    /// Where the side panel's detail pane stood, for its verbs.
    panel_at: Rect,
    dialog: Option<DialogFocus>,
    busy_since: Option<u64>,
    /// The palette's search over the runs: the query asked and when, and the last answer. It
    /// answers after 400 ms, as a server would.
    asked: Option<(String, u64)>,
    answer: Option<SearchAnswer>,
    now: u64,
    /// Where the last frame drew what a press can hit.
    filter_at: Option<Rect>,
    new_at: Option<Rect>,
    table_at: Rect,
    dialog_at: Option<(Rect, sokkel_tui::DialogLayout)>,
}

impl Runs {
    fn new() -> Runs {
        Runs {
            page: Page::Runs,
            runs: runs(),
            query: String::new(),
            failed_only: false,
            table: TableState::default(),
            jobs: TableState::default(),
            menu: MenuState::default(),
            row_menu: None,
            said: None,
            job_edits: Default::default(),
            panel_at: Rect::default(),
            dialog: None,
            busy_since: None,
            asked: None,
            answer: None,
            now: 0,
            filter_at: None,
            new_at: None,
            table_at: Rect::default(),
            dialog_at: None,
        }
    }

    fn shown(&self) -> Vec<&Run> {
        self.runs
            .iter()
            .filter(|r| !self.failed_only || r.state == "failed")
            .filter(|r| r.job.contains(self.query.as_str()))
            .collect()
    }

    fn rows(&self) -> Vec<Row> {
        self.shown()
            .iter()
            .map(|r| {
                let took = r
                    .took
                    .as_ref()
                    .map_or_else(Cell::missing, |(s, v)| Cell::text(s).sort_by(*v));
                let tone = if r.state == "failed" {
                    RowTone::Critical
                } else {
                    RowTone::Plain
                };
                Row::new([
                    Cell::text(&r.job),
                    Cell::status(&r.state),
                    Cell::maybe(r.by.as_deref()),
                    Cell::text(&r.project),
                    Cell::text(&r.started),
                    took,
                ])
                .tone(tone)
            })
            .collect()
    }

    fn filter_menu<'a>(&self, look: &'a Look) -> DropdownMenu<'a> {
        DropdownMenu::new(
            look,
            [
                PopupRow::Label("Show".into()),
                Item::new("Failed runs only").check(self.failed_only).into(),
            ],
        )
    }

    fn new_run<'a>(&self, look: &'a Look, now: u64, width: sokkel_tui::cells::Width) -> Dialog<'a> {
        let busy = self.busy_since.map(|t| now.saturating_sub(t));
        Dialog::new(look, "Start nightly-export now?")
            .description("It runs on harbor, as ada.")
            .width(DialogWidth::Sm)
            .confirm(Confirm::new("Start").busy(busy.is_some(), Some("Starting")))
            .busy_for(busy.unwrap_or(0))
            .focus(self.dialog.unwrap_or(DialogFocus::Confirm))
            .window(width)
    }

    fn dialog_event(&mut self, event: DialogEvent, now: u64) {
        match event {
            DialogEvent::Confirm => self.busy_since = Some(now),
            DialogEvent::Close | DialogEvent::StopWaiting => {
                self.busy_since = None;
                self.dialog = None;
            }
            _ => {}
        }
    }

    fn menu_event(&mut self, event: MenuEvent) {
        if event == MenuEvent::Action(1, None) {
            self.failed_only = !self.failed_only;
            self.table.selected.clear();
        }
    }

    /// What a run's action does: here it says so in the status line.
    fn row_menu_event(&mut self, event: MenuEvent) {
        let Some((row, _, _)) = self.row_menu else { return };
        match event {
            MenuEvent::Closed => self.row_menu = None,
            MenuEvent::Action(i, _) => {
                let job = self.shown().get(row).map_or(String::new(), |r| r.job.clone());
                let verb = ["Retried", "Cancelled", "Opened"].get(i).copied().unwrap_or("");
                self.said = Some(format!("{verb} {job}"));
                self.row_menu = None;
            }
            _ => {}
        }
    }

    /// The jobs, a schedule edited in the table kept over the one given.
    fn job_rows(&self) -> (Vec<Column>, Vec<Row>) {
        let (c, mut r) = jobs();
        for (&(row, column), value) in &self.job_edits {
            if let Some(cell) = r.get_mut(row).and_then(|r| r.cells.get_mut(column)) {
                *cell = Cell::text(value.clone());
            }
        }
        (c, r)
    }

    /// A cell's edit saved: kept, and said in the status line.
    fn edited(&mut self, event: TableEvent) {
        if let TableEvent::Edited { row, column } = event
            && let Some(value) = self.jobs.saved.take()
        {
            let job = jobs()
                .1
                .get(row)
                .map_or(String::new(), |r| r.cells[0].words().to_string());
            self.said = Some(format!("{job} runs {value}"));
            self.job_edits.insert((row, column), value);
        }
    }

    fn go_to(&mut self, page: Page) {
        self.page = page;
        self.menu = MenuState::default();
    }
}

impl ShellApp for Runs {
    fn id(&self) -> Option<&str> {
        Some("tidevann")
    }

    fn route(&self) -> String {
        if self.page == Page::Runs { "runs" } else { "jobs" }.into()
    }

    fn go(&mut self, route: &str) {
        self.go_to(if route == "jobs" { Page::Jobs } else { Page::Runs });
    }

    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a> {
        let title = if self.page == Page::Runs { "Runs" } else { "Jobs" };
        Chrome {
            rail: Rail::new(look, "T", "Tidevann")
                .groups([
                    RailGroup::new(
                        Some("Workspace"),
                        [
                            entry("Runs", "▶", Some(count(self.runs.len())), self.page == Page::Runs),
                            entry("Jobs", "◆", Some("5".into()), self.page == Page::Jobs),
                        ],
                    ),
                    RailGroup::new(
                        Some("Projects"),
                        ["harbor", "juniper", "larkspur"].map(|p| RailEntry::new(p).icon("●")),
                    ),
                ])
                .foot(RailFoot {
                    letter: "a".into(),
                    color: "kind-violet".into(),
                    name: "ada".into(),
                }),
            top: TopBar::new(look).trail(["Tidevann", title]).search(""),
            foot: match &self.said {
                Some(said) => StatusLine::new(look).note(said.clone(), NoteTone::Plain),
                None => StatusLine::new(look).note("Live", NoteTone::Live).live(true),
            }
            .facts([
                Fact::labeled("host", "api.example.org").code(),
                Fact::labeled("?", "Shortcuts"),
                Fact::labeled("Ctrl K", "Commands"),
            ]),
        }
    }

    fn page(&mut self, f: &mut Frame, page: Rect, cx: &mut Cx) {
        let look = cx.look;
        let at = |n: &str| FocusId::new(n);
        let shown = self.shown().len();
        let header = match self.page {
            Page::Runs => PageHeader::new(look, "Runs")
                .count(
                    TableWords::default()
                        .rows
                        .say(shown)
                        .replace(&shown.to_string(), &count(shown)),
                )
                .actions([
                    Button::new(look, "Filter")
                        .variant(ButtonVariant::Secondary)
                        .hovered(self.filter_at.zip(cx.hover).is_some_and(|(r, p)| r.contains(p)))
                        .opens_menu(self.menu.open)
                        .focused(cx.focus.ring(at(FILTER))),
                    Button::new(look, "New")
                        .hovered(self.new_at.zip(cx.hover).is_some_and(|(r, p)| r.contains(p)))
                        .focused(cx.focus.ring(at(NEW))),
                ]),
            Page::Jobs => PageHeader::new(look, "Jobs").count("5 jobs"),
        };
        let hh = header.height(page.width);
        let hr = Rect { height: hh, ..page };
        let layout = header.layout(hr);
        (self.filter_at, self.new_at) = (None, None);
        if self.page == Page::Runs {
            self.filter_at = layout.actions.first().map(|(_, r)| *r);
            self.new_at = layout.actions.get(1).map(|(_, r)| *r);
            if let Some(r) = self.filter_at {
                cx.focus.stop(at(FILTER), r);
            }
            if let Some(r) = self.new_at {
                cx.focus.stop(at(NEW), r);
            }
        }
        f.render_widget(&header, hr);
        let gap = 1 + u16::from(look.density() == Density::Regular);
        let area = Rect {
            y: page.y + hh + gap - 1,
            height: page.height.saturating_sub(hh + gap - 1),
            ..page
        };
        self.table_at = area;
        let focused = cx.focus.stop(at(TABLE), area) && cx.active;
        let hover = cx.hover.filter(|_| !self.menu.open);
        match self.page {
            Page::Runs => {
                let (columns, rows) = (columns(), self.rows());
                let table = Table::new(look, &columns, &rows)
                    .selectable(true)
                    .row_actions(true)
                    .focused(focused)
                    .total(rows.len());
                let hovered = hover
                    .and_then(|p| table.hit(area, p, &self.table))
                    .and_then(|h| match h {
                        TableHit::Row(p) | TableHit::SelectBox(p) | TableHit::Card(p) => self.table.row_at(&rows, p),
                        _ => None,
                    });
                f.render_stateful_widget(&table.hovered(hovered), area, &mut self.table);
            }
            Page::Jobs => {
                let (jc, jr) = self.job_rows();
                let table = Table::new(look, &jc, &jr).focused(focused).editing(true);
                let hovered = hover
                    .and_then(|p| table.hit(area, p, &self.jobs))
                    .and_then(|h| match h {
                        TableHit::Row(p) | TableHit::Card(p) => self.jobs.row_at(&jr, p),
                        _ => None,
                    });
                f.render_stateful_widget(&table.hovered(hovered), area, &mut self.jobs);
                if let Some(at) = self.jobs.cursor_position() {
                    f.set_cursor_position(at);
                }
            }
        }
    }

    fn commands(&self) -> Vec<CommandGroup> {
        vec![
            CommandGroup::new(
                "Go to",
                [
                    Command::new("go-runs", "Runs").icon("▶").keys(Keys::Then('g', 'r')),
                    Command::new("go-jobs", "Jobs").icon("◆").keys(Keys::Then('g', 'j')),
                ],
            ),
            CommandGroup::new(
                "Runs",
                [
                    Command::new("new", "New run").icon("+"),
                    Command::new(
                        "failed",
                        if self.failed_only {
                            "Show every run"
                        } else {
                            "Show failed runs only"
                        },
                    )
                    .icon("!"),
                ],
            ),
        ]
    }

    fn shortcuts(&self) -> Vec<Binding<String>> {
        vec![
            Binding::new(Keys::Then('g', 'r'), "go-runs".into(), "Go to runs"),
            Binding::new(Keys::Then('g', 'j'), "go-jobs".into(), "Go to jobs"),
        ]
    }

    fn command(&mut self, id: &str) -> Ran {
        match id {
            "go-runs" => self.go_to(Page::Runs),
            "go-jobs" => self.go_to(Page::Jobs),
            "new" => {
                self.go_to(Page::Runs);
                self.dialog = Some(DialogFocus::Confirm);
            }
            "failed" => self.menu_event(MenuEvent::Action(1, None)),
            _ => {}
        }
        Ran::Done
    }

    fn entry(&mut self, group: usize, entry: usize) {
        match (group, entry) {
            (0, 0) => self.go_to(Page::Runs),
            (0, 1) => self.go_to(Page::Jobs),
            _ => {}
        }
    }

    fn search(&mut self, query: &str) {
        self.query = query.to_lowercase();
        self.table.selected.clear();
    }

    /// The side panel, while a run is opened (Enter on it): its detail.
    fn panel_width(&self) -> Option<u16> {
        (self.page == Page::Runs && self.table.opened.is_some()).then_some(40)
    }

    fn panel(&mut self, f: &mut Frame, area: Rect, cx: &mut Cx) {
        let Some(run) = self.table.opened.and_then(|r| self.shown().get(r).copied()) else {
            return;
        };
        let pane = run_pane(cx.look, run);
        let h = pane.height(area.width).min(area.height);
        self.panel_at = Rect { height: h, ..area };
        f.render_widget(&pane, self.panel_at);
    }

    /// While a cell is edited, typed characters are its, not the shortcuts'.
    fn typing(&self) -> bool {
        self.jobs.edit.is_some()
    }

    fn layer_open(&self) -> bool {
        self.menu.open || self.dialog.is_some() || self.row_menu.is_some()
    }

    fn layer_scrim(&self) -> bool {
        self.dialog.is_some()
    }

    fn layer(&mut self, f: &mut Frame, cx: &mut Cx) {
        let look = cx.look;
        if let Some((_, st, at)) = &self.row_menu {
            row_menu(look).render(cx.window, *at, f.buffer_mut(), st);
        }
        if let (true, Some(anchor)) = (self.menu.open, self.filter_at) {
            self.filter_menu(look)
                .render(cx.window, anchor, f.buffer_mut(), &self.menu);
        }
        self.dialog_at = None;
        if self.dialog.is_some() {
            let d = self.new_run(look, cx.now, sokkel_tui::cells::Width::of(cx.window));
            let words = "The run starts from the schedule's last state.";
            let area = d.area(cx.window, sokkel_tui::text::wrap(words, 40).len() as u16);
            (&d).render(area, f.buffer_mut());
            let l = d.layout(area);
            let style = TextRole::Body.style(look).fg(look.token("popover-foreground"));
            for (k, line) in sokkel_tui::text::wrap(words, l.body.width as usize)
                .into_iter()
                .enumerate()
            {
                f.buffer_mut()
                    .set_stringn(l.body.x, l.body.y + k as u16, line, l.body.width as usize, style);
            }
            self.dialog_at = Some((area, l));
        }
    }

    fn key(&mut self, key: KeyPress, cx: &mut Cx) -> Flow {
        let look = cx.look;
        // Escape closes a run's detail first.
        if key.is(Key::Escape) && self.page == Page::Runs && self.table.opened.is_some() && self.dialog.is_none() {
            self.table.opened = None;
            return Flow::Handled;
        }
        if let Some((row, mut st, at)) = self.row_menu.take() {
            let event = row_menu(look).handle(&mut st, key);
            self.row_menu = Some((row, st, at));
            self.row_menu_event(event);
            return Flow::Handled;
        }
        // m on the active run opens its actions, as a press on its menu button does.
        if key.key == Key::Char('m') && self.page == Page::Runs && cx.focus.is(FocusId::new(TABLE)) {
            let (c, r) = (columns(), self.rows());
            let table = Table::new(look, &c, &r).selectable(true).row_actions(true);
            if let (Some(row), Some(at)) = (
                self.table.row_at(&r, self.table.active),
                table.actions_area(self.table_at, &self.table, self.table.active),
            ) {
                self.row_menu = Some((
                    row,
                    MenuState {
                        open: true,
                        highlight: 0,
                        sub: None,
                    },
                    at,
                ));
            }
            return Flow::Handled;
        }
        let at = |n: &str| FocusId::new(n);
        if let Some(focus) = self.dialog {
            if matches!(key.key, Key::Tab | Key::BackTab | Key::Left | Key::Right) {
                self.dialog = Some(if focus == DialogFocus::Confirm {
                    DialogFocus::Cancel
                } else {
                    DialogFocus::Confirm
                });
            } else {
                let event = self
                    .new_run(look, cx.now, sokkel_tui::cells::Width::of(cx.window))
                    .handle(key);
                self.dialog_event(event, cx.now);
            }
            return Flow::Handled;
        }
        if self.menu.open || cx.focus.is(at(FILTER)) {
            let mut st = self.menu;
            let event = self.filter_menu(look).handle(&mut st, key);
            self.menu = st;
            self.menu_event(event);
            return if event == MenuEvent::Ignored {
                Flow::Ignored
            } else {
                Flow::Handled
            };
        }
        if cx.focus.is(at(NEW)) && key.is_press() {
            self.command("new");
            return Flow::Handled;
        }
        if cx.focus.is(at(TABLE)) {
            let event = if self.page == Page::Runs {
                let (c, r) = (columns(), self.rows());
                self.table
                    .handle(&Table::new(look, &c, &r).selectable(true).row_actions(true), key)
            } else {
                let (c, r) = self.job_rows();
                let event = self.jobs.handle(&Table::new(look, &c, &r).editing(true), key);
                self.edited(event);
                event
            };
            return if event == TableEvent::Ignored {
                Flow::Ignored
            } else {
                Flow::Handled
            };
        }
        Flow::Ignored
    }

    fn pointer(&mut self, p: Pointer, cx: &mut Cx) {
        let look = cx.look;
        // The detail pane's verbs.
        if p.kind == PointerKind::Press
            && self.panel_at.contains(p.at)
            && let Some(run) = self.table.opened.and_then(|r| self.shown().get(r).copied())
        {
            let job = run.job.clone();
            if let Some(k) = run_pane(look, run).press(self.panel_at, p.at) {
                self.said = Some(format!("{} {job}", ["Retried", "Cancelled"][k]));
            }
            return;
        }
        if let Some((row, mut st, at)) = self.row_menu.take() {
            let event = row_menu(look).pointer(&mut st, cx.window, at, p);
            self.row_menu = Some((row, st, at));
            self.row_menu_event(event);
            return;
        }
        if self.dialog.is_some() {
            if let Some((area, _)) = self.dialog_at.clone() {
                let event = self
                    .new_run(look, cx.now, sokkel_tui::cells::Width::of(cx.window))
                    .press(area, p.at);
                self.dialog_event(event, cx.now);
            }
            return;
        }
        if let Some(anchor) = self.filter_at {
            let mut st = self.menu;
            let event = self.filter_menu(look).pointer(&mut st, cx.window, anchor, p);
            self.menu = st;
            if event != MenuEvent::Ignored || self.menu.open {
                self.menu_event(event);
                return;
            }
        }
        let inside = |r: Option<Rect>| r.is_some_and(|r| r.contains(p.at));
        match p.kind {
            PointerKind::Press if inside(self.new_at) => {
                self.command("new");
            }
            PointerKind::Press | PointerKind::ScrollUp | PointerKind::ScrollDown if self.table_at.contains(p.at) => {
                let by: isize = if p.kind == PointerKind::ScrollUp { -3 } else { 3 };
                if self.page == Page::Runs {
                    let (c, r) = (columns(), self.rows());
                    let table = Table::new(look, &c, &r).selectable(true).row_actions(true);
                    if p.kind == PointerKind::Press {
                        let hit = table.hit(self.table_at, p.at, &self.table);
                        let event = hit.map(|h| table.press(&mut self.table, h));
                        // While a run's detail shows, a press on a row shows that run's.
                        if self.table.opened.is_some() && event == Some(TableEvent::Moved) {
                            self.table.opened = self.table.row_at(&r, self.table.active);
                        }
                        if let Some(TableEvent::Actions(row)) = event
                            && let Some(at) = table.actions_area(self.table_at, &self.table, self.table.active)
                        {
                            self.row_menu = Some((
                                row,
                                MenuState {
                                    open: true,
                                    highlight: 0,
                                    sub: None,
                                },
                                at,
                            ));
                        }
                    } else {
                        table.scroll(&mut self.table, by);
                    }
                } else {
                    let (c, r) = self.job_rows();
                    let table = Table::new(look, &c, &r).editing(true);
                    if p.kind == PointerKind::Press {
                        if let Some(hit) = table.hit(self.table_at, p.at, &self.jobs) {
                            table.press(&mut self.jobs, hit);
                        }
                    } else {
                        table.scroll(&mut self.jobs, by);
                    }
                }
            }
            _ => {}
        }
    }

    /// A busy confirm settles after a moment and the run is added.
    fn palette_search(&mut self, query: &str) {
        self.asked = Some((query.to_string(), self.now));
    }

    fn palette_answer(&self) -> Option<SearchAnswer> {
        self.answer.clone()
    }

    /// A run chosen in the palette: its detail in the side panel.
    fn palette_open(&mut self, id: &str) {
        let Some(i) = id.strip_prefix("run:").and_then(|i| i.parse::<usize>().ok()) else {
            return;
        };
        self.page = Page::Runs;
        let job = self.runs.get(i).map(|r| (r.job.clone(), r.started.clone()));
        self.table.opened = self
            .shown()
            .iter()
            .position(|r| Some((r.job.clone(), r.started.clone())) == job);
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        // The search answers 400 ms after it was asked: the runs whose job, project or person
        // holds the query.
        if let Some((q, t)) = self.asked.clone()
            && now.saturating_sub(t) >= 400
        {
            self.asked = None;
            let ql = q.to_lowercase();
            let hits: Vec<Command> = self
                .runs
                .iter()
                .enumerate()
                .filter(|(_, r)| {
                    [Some(&r.job), Some(&r.project), r.by.as_ref()]
                        .into_iter()
                        .flatten()
                        .any(|w| w.to_lowercase().contains(&ql))
                })
                .take(6)
                .map(|(i, r)| {
                    Command::new(format!("run:{i}"), format!("{} at {}", r.job, r.started))
                        .icon("▶")
                        .detail(format!("{} in {}", r.state, r.project))
                })
                .collect();
            self.answer = Some(SearchAnswer {
                query: q,
                groups: vec![CommandGroup::new("Runs", hits)],
                failed: false,
            });
        }
        if let Some(t) = self.busy_since
            && now.saturating_sub(t) >= 1500
        {
            self.busy_since = None;
            self.dialog = None;
            self.runs
                .insert(0, run("nightly-export", "queued", Some("ada"), "harbor", "now", None));
        }
    }
}

fn entry(label: &str, icon: &str, count: Option<String>, active: bool) -> RailEntry {
    let mut e = RailEntry::new(label).icon(icon);
    if let Some(c) = count {
        e = e.count(c);
    }
    if active { e.active() } else { e }
}

fn count(n: usize) -> String {
    sokkel_tui::table::count(n)
}

/// A run's detail, for the side panel: what ran and where, then its verbs.
fn run_pane<'a>(look: &'a Look, run: &Run) -> DetailPane<'a> {
    DetailPane::new(look)
        .sections([
            PaneSection::new("Run")
                .summary(format!("{} {} in {}.", run.job, run.state, run.project))
                .facts([
                    ("State", Some(run.state.clone())),
                    ("Started by", run.by.clone()),
                    ("Started", Some(run.started.clone())),
                    ("Took", run.took.as_ref().map(|(t, _)| t.clone())),
                ]),
            PaneSection::new("Host").facts([
                ("Host", Some(format!("{}.example.org", run.project))),
                ("Command", Some(format!("tidevann run {}", run.job))),
            ]),
        ])
        .actions([
            Button::new(look, "Retry").variant(ButtonVariant::Secondary),
            Button::new(look, "Cancel run").variant(ButtonVariant::Outline),
        ])
}

/// A run's actions, from its menu button.
fn row_menu(look: &Look) -> DropdownMenu<'_> {
    DropdownMenu::new(
        look,
        [
            Item::new("Retry").icon("↻").into(),
            Item::new("Cancel run").destructive().into(),
            Item::new("Open job").into(),
        ],
    )
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("Job").width(ColumnWidth::Fill(3)).sortable(),
        Column::new("State").sortable(),
        Column::new("Started by").sortable(),
        Column::new("Project").width(ColumnWidth::Fill(2)).sortable(),
        Column::new("Started").sortable(),
        Column::new("Took").align(Align::End).sortable(),
    ]
}

fn jobs() -> (Vec<Column>, Vec<Row>) {
    let columns = vec![
        Column::new("Job").width(ColumnWidth::Fill(2)).sortable(),
        Column::new("Project").sortable(),
        Column::new("Schedule").width(ColumnWidth::Fill(1)).editable(),
        Column::new("Last run").sortable(),
    ];
    let job = |name: &str, project: &str, schedule: &str, last: &str| {
        Row::new([
            Cell::text(name),
            Cell::text(project),
            Cell::text(schedule),
            Cell::status(last),
        ])
        // Its detail, opened under it with Enter, Right or a press.
        .detail([
            ("Host", format!("{project}.example.org")),
            (
                "Owner",
                ["ada", "grace", "linus", "margaret"][name.len() % 4].to_string(),
            ),
            ("Command", format!("tidevann run {name}")),
        ])
    };
    // A job's last runs, its children in the tree.
    let run = |n: &str, when: &str, state: &str| {
        Row::new([Cell::text(n), Cell::missing(), Cell::text(when), Cell::status(state)]).child()
    };
    let rows = vec![
        job("nightly-export", "harbor", "every night at 02:00", "running"),
        run("run 412", "today 02:00", "running"),
        run("run 411", "yesterday 02:00", "succeeded"),
        job("sync-inventory", "juniper", "every hour", "failed").tone(RowTone::Critical),
        run("run 98", "today 11:58", "failed").tone(RowTone::Critical),
        run("run 97", "today 10:58", "succeeded"),
        job("rebuild-index", "larkspur", "every night at 01:40", "succeeded"),
        job("send-digest", "harbor", "every morning at 07:00", "queued"),
        job("import-orders", "juniper", "every 15 minutes", "succeeded"),
    ];
    (columns, rows)
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // A specimen: one frame after the keys given.
    if let Some(at) = args.iter().position(|a| a == "--frame") {
        let scheme = if args.iter().any(|a| a == "light") {
            Scheme::Light
        } else {
            Scheme::Dark
        };
        let look = Look::new("tidevann", scheme).expect("tidevann");
        let size = (
            args[at + 1].parse().expect("columns"),
            args[at + 2].parse().expect("rows"),
        );
        let keys: Vec<KeyPress> = args[at + 4..].iter().filter_map(|k| KeyPress::named(k)).collect();
        let buf = app::shot(Runs::new(), look.clone(), size, |_| {}, &keys);
        let out = &args[at + 3];
        std::fs::write(
            format!("{out}.svg"),
            snapshot::svg(&buf, look.token("background"), look.token("foreground")),
        )?;
        std::fs::write(format!("{out}.txt"), snapshot::text(&buf))?;
        return Ok(());
    }
    app::run_shell(Runs::new(), app::look_from_args("tidevann"))
}
