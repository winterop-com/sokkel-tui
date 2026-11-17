//! Tidevann's runs, run in a real terminal: the shell, the runs and jobs pages, the table sorting
//! and selecting by keys, a filter menu, a New run dialog whose confirm stays busy a moment, the
//! command palette, the shortcuts dialog and the settings dialog. Every part is the core's.
//!
//! Usage: cargo run --example runs [-- light] [-- --frame <cols> <rows> <out> [keys...]]
//!
//! Keys: Tab and Shift+Tab move focus; Ctrl+K the palette; ? the shortcuts; / the search; G then R
//! and G then J go to a page; in the table Up, Down, Home, End, PageUp, PageDown, Space selects,
//! Up from the first row sorts by the heads. Ctrl+Q or Ctrl+C quits.
//!
//! With `--frame` it draws one frame after the keys given (`tab`, `down`, `ctrl-k`, `enter`, a
//! character) into `<out>.svg` and `<out>.txt` instead of taking the terminal: a specimen.

use ratatui::Frame;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{self, Event};
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use sokkel_tui::{
    Align, Binding, Button, ButtonVariant, Cell, Charset, Column, ColumnWidth, Command, CommandGroup, CommandPalette,
    Confirm, Density, Depth, Dialog, DialogEvent, DialogFocus, DialogWidth, DropdownMenu, Fact, Focus, FocusId, Item,
    Key, KeyContext, KeyPress, Keys, Look, MenuEvent, MenuState, NoteTone, PageHeader, PaletteEvent, PaletteState,
    PopupRow, Rail, RailEntry, RailFoot, RailGroup, Row, RowTone, Scheme, Select, SelectEvent, SelectOption,
    SelectState, Setting, SettingsDialog, SettingsEvent, SettingsFocus, SettingsGroup, SettingsSection, SettingsState,
    ShellLayout, ShortcutWords, Shortcuts, SortDir, StatusLine, Table, TableEvent, TableState, TextRole, Tip, TopBar,
    TopBarFocus, snapshot,
};
use std::time::{Duration, Instant};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    None,
    Palette,
    NewRun,
    Shortcuts,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    Palette,
    Shortcuts,
    Search,
    GoRuns,
    GoJobs,
}

const SEARCH: &str = "search";
const PALETTE: &str = "palette";
const FILTER: &str = "filter";
const NEW: &str = "new";
const TABLE: &str = "table";

struct App {
    look: Look,
    page: Page,
    runs: Vec<Run>,
    failed_only: bool,
    table: TableState,
    jobs: TableState,
    focus: Focus,
    keys: Shortcuts<Act>,
    layer: Layer,
    menu: MenuState,
    palette: PaletteState,
    query: String,
    dialog_focus: DialogFocus,
    busy_since: Option<u64>,
    settings: SettingsState,
    theme_select: SelectState,
    now: u64,
    quit: bool,
}

impl App {
    fn new(look: Look) -> App {
        let mut focus = Focus::new();
        focus.set(FocusId::new(TABLE));
        App {
            look,
            page: Page::Runs,
            runs: runs(),
            failed_only: false,
            table: TableState::default().sorted_by(4, SortDir::Descending),
            jobs: TableState::default(),
            focus,
            keys: Shortcuts::new()
                .bind(Binding::new(Keys::Mod('k'), Act::Palette, "Open the command palette").under_layer())
                .bind(Binding::new(
                    Keys::Char('?'),
                    Act::Shortcuts,
                    "Show the keyboard shortcuts",
                ))
                .bind(Binding::new(Keys::Char('/'), Act::Search, "Search"))
                .bind(Binding::new(Keys::Then('g', 'r'), Act::GoRuns, "Go to runs"))
                .bind(Binding::new(Keys::Then('g', 'j'), Act::GoJobs, "Go to jobs")),
            layer: Layer::None,
            menu: MenuState::default(),
            palette: PaletteState::default(),
            query: String::new(),
            dialog_focus: DialogFocus::Confirm,
            busy_since: None,
            settings: SettingsState::default(),
            theme_select: SelectState::default(),
            now: 0,
            quit: false,
        }
    }

    fn shown_runs(&self) -> Vec<Run> {
        self.runs
            .iter()
            .filter(|r| !self.failed_only || r.state == "failed")
            .filter(|r| self.query.is_empty() || r.job.contains(self.query.as_str()))
            .cloned()
            .collect()
    }

    fn open(&mut self, layer: Layer) {
        self.layer = layer;
        self.focus.open_layer(FocusId::new("layer"), None);
    }

    fn close(&mut self) {
        self.layer = Layer::None;
        self.menu = MenuState::default();
        self.focus.close_layer();
    }

    /// Draw in another scheme or density from the next frame.
    fn rest_look(&mut self, scheme: Scheme, density: Density) {
        self.look = Look::new(self.look.theme(), scheme)
            .expect("the theme")
            .with_density(density)
            .with_depth(self.look.depth())
            .with_charset(self.look.charset());
    }

    fn groups(&self) -> Vec<CommandGroup> {
        vec![
            CommandGroup::new(
                "Go to",
                [
                    Command::new("go-runs", "Runs").icon("▶").keys(Keys::Then('g', 'r')),
                    Command::new("go-jobs", "Jobs").icon("◆").keys(Keys::Then('g', 'j')),
                    Command::new("settings", "Settings").icon(self.look.glyphs().search),
                ],
            ),
            CommandGroup::new(
                "Look",
                [
                    Command::new("scheme", "Switch light and dark").icon("◐"),
                    Command::new("density", "Switch density")
                        .icon("≡")
                        .hint(match self.look.density() {
                            Density::Regular => "regular now",
                            Density::Compact => "compact now",
                        }),
                ],
            ),
            CommandGroup::new(
                "Runs",
                [
                    Command::new("new", "New run").icon("+"),
                    Command::new("shortcuts", "Show the keyboard shortcuts")
                        .icon("?")
                        .keys(Keys::Char('?')),
                ],
            ),
        ]
    }

    fn act(&mut self, act: Act) {
        match act {
            Act::Palette => {
                if self.layer == Layer::Palette {
                    self.close();
                } else if self.layer == Layer::None {
                    self.palette = PaletteState::default();
                    self.open(Layer::Palette);
                }
            }
            Act::Shortcuts => self.open(Layer::Shortcuts),
            Act::Search => self.focus.set(FocusId::new(SEARCH)),
            Act::GoRuns => self.page = Page::Runs,
            Act::GoJobs => self.page = Page::Jobs,
        }
    }

    fn run_command(&mut self, id: &str) {
        self.close();
        match id {
            "go-runs" => self.page = Page::Runs,
            "go-jobs" => self.page = Page::Jobs,
            "settings" => {
                self.settings = SettingsState::default();
                self.open(Layer::Settings);
            }
            "scheme" => {
                let s = match self.look.scheme() {
                    Scheme::Dark => Scheme::Light,
                    Scheme::Light => Scheme::Dark,
                };
                self.rest_look(s, self.look.density());
            }
            "density" => {
                let d = match self.look.density() {
                    Density::Regular => Density::Compact,
                    Density::Compact => Density::Regular,
                };
                self.rest_look(self.look.scheme(), d);
            }
            "new" => {
                self.dialog_focus = DialogFocus::Confirm;
                self.open(Layer::NewRun);
            }
            "shortcuts" => self.open(Layer::Shortcuts),
            _ => {}
        }
    }

    /// One key, from the terminal or a script.
    fn key(&mut self, key: KeyPress) {
        if key.ctrl && matches!(key.key, Key::Char('q') | Key::Char('c')) {
            self.quit = true;
            return;
        }
        // The app's shortcuts, quiet in a field and under a layer unless they ask.
        let cx = KeyContext {
            typing: self.focus.typing(),
            under_layer: self.layer != Layer::None,
            now_ms: self.now,
        };
        if let Some(act) = self.keys.handle(key, cx) {
            self.act(act);
            return;
        }
        if self.keys.pending().is_some() {
            return;
        }
        match self.layer {
            Layer::Palette => {
                let p = CommandPalette::new(&self.look, self.groups());
                match p.handle(&mut self.palette, key) {
                    PaletteEvent::Run(id) => self.run_command(&id),
                    PaletteEvent::Close | PaletteEvent::StopWaiting => self.close(),
                    _ => {}
                }
                return;
            }
            Layer::NewRun => {
                if key.is(Key::Tab) || key.is(Key::BackTab) || key.is(Key::Left) || key.is(Key::Right) {
                    self.dialog_focus = match self.dialog_focus {
                        DialogFocus::Confirm => DialogFocus::Cancel,
                        _ => DialogFocus::Confirm,
                    };
                    return;
                }
                match self.new_run_dialog().handle(key) {
                    DialogEvent::Confirm => self.busy_since = Some(self.now),
                    DialogEvent::Close | DialogEvent::StopWaiting => {
                        self.busy_since = None;
                        self.close();
                    }
                    _ => {}
                }
                return;
            }
            Layer::Shortcuts => {
                if key.is(Key::Escape) || key.is_press() {
                    self.close();
                }
                return;
            }
            Layer::Settings => {
                self.settings_key(key);
                return;
            }
            Layer::None => {}
        }
        if self.focus.handle(key) {
            return;
        }
        let at = |name: &str| FocusId::new(name);
        if self.focus.is(at(SEARCH)) {
            match key.key {
                Key::Char(c) if !key.ctrl => self.query.push(c),
                Key::Backspace => {
                    self.query.pop();
                }
                Key::Escape | Key::Enter => self.focus.set(at(TABLE)),
                _ => {}
            }
        } else if self.focus.is(at(PALETTE)) && key.is_press() {
            self.act(Act::Palette);
        } else if self.focus.is(at(FILTER)) {
            let mut state = self.menu;
            let event = self.filter_menu().handle(&mut state, key);
            self.menu = state;
            if event == MenuEvent::Action(1, None) {
                self.failed_only = !self.failed_only;
            }
        } else if self.focus.is(at(NEW)) && key.is_press() {
            self.run_command("new");
        } else if self.focus.is(at(TABLE)) {
            let rows = self.rows();
            let columns = columns();
            if self.page == Page::Runs {
                let t = Table::new(&self.look, &columns, &rows).selectable(true);
                if let TableEvent::Opened(_) = self.table.handle(&t, key) {}
            } else {
                let (jc, jr) = jobs();
                let t = Table::new(&self.look, &jc, &jr);
                self.jobs.handle(&t, key);
            }
        }
    }

    fn settings_key(&mut self, key: KeyPress) {
        let order = [
            SettingsFocus::Search,
            SettingsFocus::Entry(0, 0),
            SettingsFocus::Control(0),
            SettingsFocus::Control(1),
            SettingsFocus::Done,
        ];
        if key.is(Key::Tab) || key.is(Key::BackTab) {
            let at = order.iter().position(|f| *f == self.settings.focus).unwrap_or(0);
            let n = order.len();
            let to = if key.is(Key::Tab) {
                (at + 1) % n
            } else {
                (at + n - 1) % n
            };
            self.settings.focus = order[to];
            return;
        }
        match self.settings.focus {
            SettingsFocus::Control(0) => {
                let mut state = self.theme_select;
                let event = self.scheme_select().handle(&mut state, key);
                self.theme_select = state;
                match event {
                    SelectEvent::Chose(0) => self.rest_look(Scheme::Light, self.look.density()),
                    SelectEvent::Chose(_) => self.rest_look(Scheme::Dark, self.look.density()),
                    _ => {}
                }
                if self.theme_select.open || !key.is(Key::Escape) {
                    return;
                }
            }
            SettingsFocus::Control(1) if key.is_press() => {
                self.run_command("density");
                self.open(Layer::Settings);
                return;
            }
            _ => {}
        }
        let dialog = settings_dialog(&self.look);
        if dialog.handle(&mut self.settings, key) == SettingsEvent::Close {
            self.close();
        }
    }

    fn scheme_select(&self) -> Select<'_> {
        let value = match self.look.scheme() {
            Scheme::Light => 0,
            Scheme::Dark => 1,
        };
        Select::new(&self.look, [SelectOption::new("Light"), SelectOption::new("Dark")])
            .value(Some(value))
            .focused(self.settings.focus == SettingsFocus::Control(0))
    }

    fn filter_menu(&self) -> DropdownMenu<'_> {
        DropdownMenu::new(
            &self.look,
            [
                PopupRow::Label("Show".into()),
                Item::new("Failed runs only").check(self.failed_only).into(),
            ],
        )
    }

    fn new_run_dialog(&self) -> Dialog<'_> {
        let busy = self.busy_since.map(|t| self.now.saturating_sub(t));
        Dialog::new(&self.look, "Start nightly-export now?")
            .description("It runs on harbor, as ada.")
            .width(DialogWidth::Sm)
            .confirm(Confirm::new("Start").busy(busy.is_some(), Some("Starting")))
            .busy_for(busy.unwrap_or(0))
            .focus(self.dialog_focus)
    }

    /// Time passes: a busy confirm settles after a moment and the run is added.
    fn tick(&mut self, now: u64) {
        self.now = now;
        if let Some(t) = self.busy_since
            && now.saturating_sub(t) >= 1500
        {
            self.busy_since = None;
            self.runs
                .insert(0, run("nightly-export", "queued", Some("ada"), "harbor", "now", None));
            self.close();
        }
    }

    fn rows(&self) -> Vec<Row> {
        self.shown_runs()
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

    fn draw(&mut self, f: &mut Frame) {
        let look = &self.look;
        let window = f.area();
        f.buffer_mut().set_style(
            window,
            Style::new().bg(look.token("background")).fg(look.token("foreground")),
        );
        let at = |name: &str| FocusId::new(name);
        self.focus.begin();
        let rail = Rail::new(look, "T", "Tidevann")
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
            });
        let shell = ShellLayout::new(look, window, rail.width());
        if let Some(r) = shell.rail {
            f.render_widget(&rail, r);
        }
        let title = match self.page {
            Page::Runs => "Runs",
            Page::Jobs => "Jobs",
        };
        let top = TopBar::new(look)
            .trail(["Tidevann", title])
            .window(shell.width)
            .search(self.query.clone());
        let top_layout = top.layout(shell.top);
        if let Some(r) = top_layout.search {
            self.focus.text_stop(at(SEARCH), r);
        }
        if let Some(r) = top_layout.palette {
            self.focus.stop(at(PALETTE), r);
        }
        let top_focus = if self.focus.ring(at(SEARCH)) {
            TopBarFocus::Search
        } else if self.focus.ring(at(PALETTE)) {
            TopBarFocus::Palette
        } else {
            TopBarFocus::None
        };
        f.render_widget(top.focus(top_focus), shell.top);

        // The page: its header, then the table.
        let page = shell.page(look);
        let shown = self.shown_runs().len();
        let header = match self.page {
            Page::Runs => PageHeader::new(look, "Runs")
                .count(format!("{} runs", count(shown)))
                .actions([
                    Button::new(look, "Filter")
                        .variant(ButtonVariant::Secondary)
                        .opens_menu(self.menu.open)
                        .focused(self.focus.ring(at(FILTER))),
                    Button::new(look, "New").focused(self.focus.ring(at(NEW))),
                ]),
            Page::Jobs => PageHeader::new(look, "Jobs").count("5 jobs"),
        };
        let hh = header.height(page.width);
        let hr = Rect { height: hh, ..page };
        let layout = header.layout(hr);
        let mut filter_at = None;
        if self.page == Page::Runs {
            if let Some((_, r)) = layout.actions.first() {
                self.focus.stop(at(FILTER), *r);
                filter_at = Some(*r);
            }
            if let Some((_, r)) = layout.actions.get(1) {
                self.focus.stop(at(NEW), *r);
            }
        }
        f.render_widget(&header, hr);
        let gap = 1 + u16::from(look.density() == Density::Regular);
        let table_area = Rect {
            y: page.y + hh + gap - 1,
            height: page.height.saturating_sub(hh + gap - 1),
            ..page
        };
        let table_focus = self.focus.is(at(TABLE)) && self.layer == Layer::None;
        self.focus.stop(at(TABLE), table_area);
        match self.page {
            Page::Runs => {
                // One set of columns at every width: the sort's index means one column, and the
                // table's own widths squeeze them.
                let columns = columns();
                let rows = self.rows();
                let table = Table::new(look, &columns, &rows)
                    .selectable(true)
                    .focused(table_focus)
                    .total(rows.len());
                f.render_stateful_widget(&table, table_area, &mut self.table);
            }
            Page::Jobs => {
                let (jc, jr) = jobs();
                let table = Table::new(look, &jc, &jr).focused(table_focus);
                f.render_stateful_widget(&table, table_area, &mut self.jobs);
            }
        }

        // The status line: the focused control's tip, else the note and the facts.
        let tip = if self.focus.ring(at(PALETTE)) {
            Some(Tip {
                words: "Open the command palette".into(),
                shortcut: Some(Keys::Mod('k').label(&ShortcutWords::default())),
            })
        } else if self.focus.ring(at(FILTER)) {
            Some(Tip {
                words: "Show some runs, not all".into(),
                shortcut: None,
            })
        } else {
            None
        };
        let pending = self.keys.pending().map(|c| format!("{} then …", c.to_uppercase()));
        let foot = StatusLine::new(look)
            .note(pending.unwrap_or_else(|| "Live".into()), NoteTone::Live)
            .live(true)
            .tip(tip)
            .facts([
                Fact::labeled("host", "api.example.org").code(),
                Fact::labeled("?", "Shortcuts"),
                Fact::labeled("Ctrl K", "Commands"),
            ]);
        f.render_widget(&foot, shell.foot);

        // The layers, last, over the frame.
        self.focus.enter_layer(FocusId::new("layer"));
        if let Some(r) = filter_at {
            self.filter_menu().render(window, r, f.buffer_mut(), &self.menu);
        }
        match self.layer {
            Layer::Palette => {
                let p = CommandPalette::new(look, self.groups()).window(shell.width);
                let area = p.area(window, &self.palette);
                p.render(area, f.buffer_mut(), &mut self.palette);
                if let Some(pos) = self.palette.cursor_position() {
                    f.set_cursor_position(pos);
                }
            }
            Layer::NewRun => {
                let d = self.new_run_dialog().window(shell.width);
                let words = "The run starts from the schedule's last state.";
                let lines = sokkel_tui::text::wrap(words, 40);
                let area = d.area(window, lines.len() as u16);
                (&d).render(area, f.buffer_mut());
                let body = d.layout(area).body;
                let style = TextRole::Body.style(look).fg(look.token("popover-foreground"));
                for (k, line) in sokkel_tui::text::wrap(words, body.width as usize)
                    .into_iter()
                    .enumerate()
                {
                    f.buffer_mut()
                        .set_stringn(body.x, body.y + k as u16, line, body.width as usize, style);
                }
            }
            Layer::Shortcuts => {
                let d = Dialog::new(look, "Keyboard shortcuts")
                    .width(DialogWidth::Sm)
                    .focus(DialogFocus::Close)
                    .window(shell.width);
                let lines = self.keys.bindings().len() as u16 + 2;
                let area = d.area(window, lines);
                (&d).render(area, f.buffer_mut());
                let body = d.layout(area).body;
                let words = ShortcutWords::default();
                let all = self
                    .keys
                    .bindings()
                    .iter()
                    .map(|b| (b.keys.label(&words), b.does.clone()))
                    .chain([
                        ("Tab".to_string(), "Move focus".to_string()),
                        ("Ctrl Q".into(), "Quit".into()),
                    ]);
                for (k, (keys, does)) in all.enumerate() {
                    let y = body.y + k as u16;
                    if y >= body.bottom() {
                        break;
                    }
                    let kst = TextRole::Badge
                        .style(look)
                        .fg(look.token("muted-foreground"))
                        .bg(look.token("muted"));
                    f.buffer_mut().set_string(body.x, y, format!(" {keys} "), kst);
                    f.buffer_mut().set_string(
                        body.x + 12,
                        y,
                        does,
                        TextRole::Body.style(look).fg(look.token("popover-foreground")),
                    );
                }
            }
            Layer::Settings => {
                let d = settings_dialog(look).window(shell.width);
                let area = d.area(window);
                d.render(area, f.buffer_mut(), &mut self.settings);
                let layout = d.layout(area, &self.settings);
                for (k, r) in &layout.controls {
                    match k {
                        0 => {
                            let sel = self.scheme_select();
                            sel.render_trigger(*r, f.buffer_mut(), &self.theme_select);
                        }
                        _ => {
                            let words = match look.density() {
                                Density::Regular => "Regular",
                                Density::Compact => "Compact",
                            };
                            Button::new(look, words)
                                .variant(ButtonVariant::Outline)
                                .focused(self.settings.focus == SettingsFocus::Control(1))
                                .render(
                                    Rect {
                                        width: r.width.min(10),
                                        ..*r
                                    },
                                    f.buffer_mut(),
                                );
                        }
                    }
                }
                if let Some((_, r)) = layout.controls.first() {
                    self.scheme_select()
                        .render_list(window, *r, f.buffer_mut(), &self.theme_select);
                }
                if let Some(pos) = self.settings.cursor_position() {
                    f.set_cursor_position(pos);
                }
            }
            Layer::None => {}
        }
        self.focus.leave_layer();
        self.focus.end();
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
        Column::new("Schedule").width(ColumnWidth::Fill(1)),
        Column::new("Last run").sortable(),
    ];
    let job = |name: &str, project: &str, schedule: &str, last: &str| {
        Row::new([
            Cell::text(name),
            Cell::text(project),
            Cell::text(schedule),
            Cell::status(last),
        ])
    };
    let rows = vec![
        job("nightly-export", "harbor", "every night at 02:00", "running"),
        job("sync-inventory", "juniper", "every hour", "failed").tone(RowTone::Critical),
        job("rebuild-index", "larkspur", "every night at 01:40", "succeeded"),
        job("send-digest", "harbor", "every morning at 07:00", "queued"),
        job("import-orders", "juniper", "every 15 minutes", "succeeded"),
    ];
    (columns, rows)
}

fn settings_dialog(look: &Look) -> SettingsDialog<'_> {
    SettingsDialog::new(
        look,
        [SettingsGroup::new(
            None,
            [SettingsSection::new(
                "general",
                "General",
                [
                    Setting::new("scheme", "Scheme", 14).description("Light or dark, on this machine."),
                    Setting::new("density", "Density", 10).description("Regular, or compact rows and padding."),
                ],
            )
            .icon("✱")
            .no_reset()],
        )],
    )
}

/// A key a script names: `tab`, `shift-tab`, `down`, `ctrl-k`, `enter`, `esc`, or a character.
fn scripted(name: &str) -> Option<KeyPress> {
    let key = |k| Some(KeyPress::new(k));
    match name {
        "tab" => key(Key::Tab),
        "shift-tab" => key(Key::BackTab),
        "up" => key(Key::Up),
        "down" => key(Key::Down),
        "left" => key(Key::Left),
        "right" => key(Key::Right),
        "home" => key(Key::Home),
        "end" => key(Key::End),
        "enter" => key(Key::Enter),
        "esc" => key(Key::Escape),
        "space" => key(Key::Char(' ')),
        "backspace" => key(Key::Backspace),
        s if s.starts_with("ctrl-") => s.chars().nth(5).map(|c| KeyPress::ctrl(Key::Char(c))),
        s if s.chars().count() == 1 => s.chars().next().map(|c| KeyPress::new(Key::Char(c))),
        _ => None,
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let scheme = if args.iter().any(|a| a == "light") {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let look = Look::new("tidevann", scheme)
        .expect("tidevann")
        .with_depth(Depth::detect())
        .with_charset(Charset::detect());

    // A specimen: one frame after the keys given.
    if let Some(at) = args.iter().position(|a| a == "--frame") {
        let cols: u16 = args[at + 1].parse().expect("columns");
        let rows: u16 = args[at + 2].parse().expect("rows");
        let out = &args[at + 3];
        let look = Look::new("tidevann", scheme).expect("tidevann");
        let mut app = App::new(look);
        let mut term = ratatui::Terminal::new(TestBackend::new(cols, rows)).expect("a test terminal");
        term.draw(|f| app.draw(f)).expect("a frame");
        for (n, name) in args[at + 4..].iter().enumerate() {
            app.tick(n as u64 * 100);
            if let Some(k) = scripted(name) {
                app.key(k);
            }
            term.draw(|f| app.draw(f)).expect("a frame");
        }
        let buf = term.backend().buffer();
        let ground = app.look.token("background");
        let ink = app.look.token("foreground");
        std::fs::write(format!("{out}.svg"), snapshot::svg(buf, ground, ink))?;
        std::fs::write(format!("{out}.txt"), snapshot::text(buf))?;
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let start = Instant::now();
    let mut app = App::new(look);
    let result = (|| -> std::io::Result<()> {
        while !app.quit {
            app.tick(start.elapsed().as_millis() as u64);
            terminal.draw(|f| app.draw(f))?;
            // One frame each motion-fast, so spinners turn.
            if event::poll(Duration::from_millis(app.look.duration_ms("motion-fast")))?
                && let Event::Key(k) = event::read()?
                && let Some(key) = KeyPress::from_crossterm(&k)
            {
                app.key(key);
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}
