//! 04: what opens over a page. A Filter menu with check rows, a label and a submenu; a select
//! choosing the project; and a Delete button whose confirm dialog stays busy a moment, Cancel saying
//! Stop waiting once it has held `busy-release`. Focus goes back to what opened each.
//!
//! Usage: cargo run --example 04_overlays [-- light]
//! Keys: Tab moves between Filter, the project and Delete; Enter, Space or Down opens; Up and Down
//! move in what is open, Right opens the submenu, Enter chooses, Escape closes. The mouse works on
//! every row and verb, and a press outside closes. Ctrl+Q quits.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use sokkel_tui::app;
use sokkel_tui::{
    Button, ButtonVariant, Confirm, Dialog, DialogEvent, DialogFocus, DialogLayout, DialogWidth, DropdownMenu, Fact,
    Item, Key, KeyPress, Look, MenuEvent, MenuState, NoteTone, PageHeader, Pointer, PointerKind, PopupRow, Select,
    SelectEvent, SelectOption, SelectState, StatusLine, TextRole,
};

const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Filter,
    Project,
    Delete,
}

struct App {
    look: Look,
    focus: Stop,
    failed_only: bool,
    mine_only: bool,
    since: usize,
    menu: MenuState,
    project: usize,
    select: SelectState,
    dialog: bool,
    dialog_focus: DialogFocus,
    busy_since: Option<u64>,
    said: String,
    now: u64,
    at: [Rect; 3],
    dialog_at: Option<(Rect, DialogLayout)>,
    /// The window the last frame drew in: where the popups open.
    win: Rect,
}

impl App {
    fn menu(&self) -> DropdownMenu<'_> {
        DropdownMenu::new(
            &self.look,
            [
                PopupRow::Label("Show".into()),
                Item::new("Failed runs only").check(self.failed_only).into(),
                Item::new("Started by me").check(self.mine_only).into(),
                PopupRow::Separator,
                Item::new("Since").into(),
            ],
        )
        .submenu(
            4,
            ["Today", "This week", "Ever"]
                .iter()
                .enumerate()
                .map(|(k, w)| Item::new(*w).check(k == self.since).into()),
        )
    }

    fn select(&self) -> Select<'_> {
        Select::new(&self.look, PROJECTS.map(|p| SelectOption::new(p).lead("●")))
            .value(Some(self.project))
            .focused(self.focus == Stop::Project && !self.select.open)
    }

    fn dialog(&self) -> Dialog<'_> {
        let busy = self.busy_since.map(|t| self.now - t);
        Dialog::new(&self.look, "Delete nightly-export?")
            .description("Its 1,200 runs go with it.")
            .width(DialogWidth::Sm)
            .confirm(
                Confirm::new("Delete")
                    .destructive()
                    .busy(busy.is_some(), Some("Deleting")),
            )
            .busy_for(busy.unwrap_or(0))
            .focus(self.dialog_focus)
    }

    fn menu_event(&mut self, event: MenuEvent) {
        match event {
            MenuEvent::Action(1, None) => self.failed_only = !self.failed_only,
            MenuEvent::Action(2, None) => self.mine_only = !self.mine_only,
            MenuEvent::Action(4, Some(k)) => {
                self.since = k;
                self.said = format!("Since: {}", ["today", "this week", "ever"][k]);
            }
            _ => {}
        }
    }

    fn dialog_event(&mut self, event: DialogEvent) {
        match event {
            DialogEvent::Confirm => self.busy_since = Some(self.now),
            DialogEvent::Close => self.dialog = false,
            DialogEvent::StopWaiting => {
                self.busy_since = None;
                self.dialog = false;
                self.said = "Stopped waiting; the delete goes on without us.".into();
            }
            _ => {}
        }
    }
}

impl app::Program for App {
    fn look(&self) -> &Look {
        &self.look
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        // A slow delete: it settles after eight seconds, past busy-release.
        if let Some(t) = self.busy_since
            && now - t > 8000
        {
            self.busy_since = None;
            self.dialog = false;
            self.said = "Deleted nightly-export.".into();
        }
    }

    fn key(&mut self, key: KeyPress) -> bool {
        if self.dialog {
            if key.is(Key::Tab) || key.is(Key::BackTab) {
                self.dialog_focus = match self.dialog_focus {
                    DialogFocus::Confirm => DialogFocus::Cancel,
                    _ => DialogFocus::Confirm,
                };
                return false;
            }
            let event = self.dialog().handle(key);
            self.dialog_event(event);
            return false;
        }
        if self.menu.open {
            let mut st = self.menu;
            let event = self.menu().handle(&mut st, key);
            self.menu = st;
            self.menu_event(event);
            return false;
        }
        if self.select.open {
            let mut st = self.select;
            if let SelectEvent::Chose(k) = self.select().handle(&mut st, key) {
                self.project = k;
            }
            self.select = st;
            return false;
        }
        match key.key {
            Key::Escape => return true,
            Key::Tab => {
                self.focus = match self.focus {
                    Stop::Filter => Stop::Project,
                    Stop::Project => Stop::Delete,
                    Stop::Delete => Stop::Filter,
                }
            }
            Key::BackTab => {
                self.focus = match self.focus {
                    Stop::Filter => Stop::Delete,
                    Stop::Project => Stop::Filter,
                    Stop::Delete => Stop::Project,
                }
            }
            _ => match self.focus {
                Stop::Filter => {
                    let mut st = self.menu;
                    self.menu().handle(&mut st, key);
                    self.menu = st;
                }
                Stop::Project => {
                    let mut st = self.select;
                    if let SelectEvent::Chose(k) = self.select().handle(&mut st, key) {
                        self.project = k;
                    }
                    self.select = st;
                }
                Stop::Delete if key.is_press() => {
                    self.dialog = true;
                    self.dialog_focus = DialogFocus::Confirm;
                }
                Stop::Delete => {}
            },
        }
        false
    }

    fn pointer(&mut self, p: Pointer) {
        // An open dialog takes every press; a press outside closes it unless it is busy.
        if self.dialog {
            if let Some((area, _)) = &self.dialog_at {
                let event = self.dialog().press(*area, p.at);
                self.dialog_event(event);
            }
            return;
        }
        // The menu and the select answer what lands on them or their triggers.
        let mut st = self.menu;
        let event = self.menu().pointer(&mut st, self.win, self.at[0], p);
        self.menu = st;
        if event != MenuEvent::Ignored {
            self.focus = Stop::Filter;
            self.menu_event(event);
            return;
        }
        let mut st = self.select;
        let event = self.select().pointer(&mut st, self.win, self.at[1], p);
        self.select = st;
        match event {
            SelectEvent::Ignored => {}
            SelectEvent::Chose(k) => self.project = k,
            _ => self.focus = Stop::Project,
        }
        if event != SelectEvent::Ignored {
            return;
        }
        if p.kind == PointerKind::Press && self.at[2].contains(p.at) {
            self.focus = Stop::Delete;
            self.dialog = true;
            self.dialog_focus = DialogFocus::Confirm;
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let look = &self.look;
        let area = f.area();
        f.buffer_mut().set_style(
            area,
            Style::new().bg(look.token("background")).fg(look.token("foreground")),
        );
        let foot_h = StatusLine::height(look);
        let page = Rect::new(
            area.x + 2,
            area.y + 1,
            area.width.saturating_sub(4),
            area.height.saturating_sub(foot_h + 1),
        );
        let header = PageHeader::new(look, "Runs")
            .count(format!("in {}", PROJECTS[self.project]))
            .actions([
                Button::new(look, "Filter")
                    .variant(ButtonVariant::Secondary)
                    .opens_menu(self.menu.open)
                    .focused(self.focus == Stop::Filter && !self.menu.open && !self.dialog),
                Button::new(look, "Delete job")
                    .variant(ButtonVariant::Outline)
                    .destructive_hover(true)
                    .focused(self.focus == Stop::Delete && !self.dialog),
            ]);
        let hr = Rect {
            height: header.height(page.width),
            ..page
        };
        let l = header.layout(hr);
        f.render_widget(&header, hr);
        self.at[0] = l.actions[0].1;
        self.at[2] = l.actions[1].1;

        // The project select, and what the filter shows.
        let y = hr.bottom() + 1;
        let label = TextRole::Label.style(look).fg(look.token("foreground"));
        f.buffer_mut().set_string(page.x, y, "Project", label);
        self.at[1] = Rect::new(page.x, y + 1, 20, 1);
        self.select().render_trigger(self.at[1], f.buffer_mut(), &self.select);
        let quiet = TextRole::Help.style(look).fg(look.token("muted-foreground"));
        let shows = format!(
            "Showing {} runs{}, {}.",
            if self.failed_only { "failed" } else { "all" },
            if self.mine_only { " started by me" } else { "" },
            ["today", "this week", "ever"][self.since]
        );
        f.buffer_mut().set_string(page.x, y + 3, shows, quiet);

        // What is open, last.
        self.win = area;
        self.menu().render(area, self.at[0], f.buffer_mut(), &self.menu);
        self.select()
            .render_list(area, self.at[1], f.buffer_mut(), &self.select);
        self.dialog_at = None;
        if self.dialog {
            let d = self.dialog();
            let r = d.area(area, 1);
            (&d).render(r, f.buffer_mut());
            let l = d.layout(r);
            f.buffer_mut().set_string(
                l.body.x,
                l.body.y,
                "Runs that are going stop first.",
                TextRole::Body.style(look).fg(look.token("popover-foreground")),
            );
            self.dialog_at = Some((r, l));
        }
        let note = if self.said.is_empty() {
            "Tab moves, Enter opens, Escape closes"
        } else {
            &self.said
        };
        let foot = StatusLine::new(look)
            .note(note, NoteTone::Plain)
            .facts([Fact::labeled("Ctrl Q", "quit")]);
        f.render_widget(
            &foot,
            Rect::new(area.x, area.bottom().saturating_sub(foot_h), area.width, foot_h),
        );
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        look: app::look_from_args("tidevann"),
        focus: Stop::Filter,
        failed_only: false,
        mine_only: false,
        since: 1,
        menu: MenuState::default(),
        project: 0,
        select: SelectState::default(),
        dialog: false,
        dialog_focus: DialogFocus::Confirm,
        busy_since: None,
        said: String::new(),
        now: 0,
        at: [Rect::default(); 3],
        dialog_at: None,
        win: Rect::default(),
    };
    app::run(&mut app)
}
