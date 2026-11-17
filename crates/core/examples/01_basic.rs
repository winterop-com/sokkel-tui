//! 01: the basics. A look, focus moved by Tab, two text fields and two buttons, and the status
//! line saying what the focused control would put in a tooltip.
//!
//! A "New job" form: type a name, a schedule in cron's five fields, and a note (a text area). While the schedule is not
//! five fields its field shows the issue mark, and Create says why it cannot act yet; once it can,
//! Enter on it is busy a moment and the job is made.
//!
//! Usage: cargo run --example 01_basic [-- light]
//! Keys: Tab and Shift+Tab move focus; type in a field; Enter or Space presses a button;
//! Ctrl+Q or Esc quits. A press of the pointer focuses what is under it, and presses a button.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{StatefulWidget, Widget};
use sokkel_tui::app;
use sokkel_tui::{
    Button, ButtonVariant, Fact, FieldFocus, FieldState, Focus, FocusId, Issue, Key, KeyPress, Look, NoteTone, Pointer,
    PointerKind, StatusLine, TextField, TextRole, Tip,
};

const NAME: &str = "name";
const SCHEDULE: &str = "schedule";
const NOTE: &str = "note";
const CANCEL: &str = "cancel";
const CREATE: &str = "create";

struct App {
    look: Look,
    focus: Focus,
    name: FieldState,
    schedule: FieldState,
    note: FieldState,
    busy_since: Option<u64>,
    made: Option<String>,
    now: u64,
    quit: bool,
}

impl App {
    /// Why Create cannot act yet, in the app's words.
    fn reason(&self) -> Option<&'static str> {
        if self.name.value().trim().is_empty() {
            Some("Give the job a name first")
        } else if self.schedule_issue().is_some() {
            Some("Fix the schedule first")
        } else {
            None
        }
    }

    fn schedule_issue(&self) -> Option<Issue> {
        let v = self.schedule.value().trim();
        let fields = v.split_whitespace().count();
        (!v.is_empty() && fields != 5).then(|| {
            Issue::critical(format!("A schedule is five fields; this is {fields}."))
                .detail("minute, hour, day, month, weekday")
        })
    }

    fn create(&self) -> Button<'_> {
        Button::new(&self.look, "Create")
            .reason(self.reason())
            .busy(
                self.busy_since.is_some(),
                self.now - self.busy_since.unwrap_or(self.now),
            )
            .focused(self.focus.ring(FocusId::new(CREATE)))
    }

    fn key(&mut self, key: KeyPress) {
        if (key.ctrl && key.key == Key::Char('q')) || key.is(Key::Escape) {
            self.quit = true;
            return;
        }
        if self.focus.handle(key) {
            return;
        }
        let at = |n: &str| FocusId::new(n);
        if self.focus.is(at(NAME)) {
            let field = TextField::new(&self.look, "Name");
            self.name.focus = FieldFocus::Control;
            self.name.handle(&field, key);
        } else if self.focus.is(at(SCHEDULE)) {
            let field = TextField::new(&self.look, "Schedule");
            self.schedule.focus = FieldFocus::Control;
            self.schedule.handle(&field, key);
        } else if self.focus.is(at(NOTE)) {
            let field = TextField::new(&self.look, "Note").multiline(3, 5);
            self.note.focus = FieldFocus::Control;
            self.note.handle(&field, key);
        } else if self.focus.is(at(CANCEL)) && key.is_press() {
            self.quit = true;
        } else if self.focus.is(at(CREATE)) && self.create().presses(key) {
            self.busy_since = Some(self.now);
        }
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        if let Some(t) = self.busy_since
            && now - t > 1200
        {
            self.busy_since = None;
            self.made = Some(self.name.value().to_string());
            self.name.set_value("");
            self.schedule.set_value("");
            self.note.set_value("");
            self.focus.set(FocusId::new(NAME));
        }
    }

    fn draw(&mut self, f: &mut ratatui::Frame) {
        let look = &self.look;
        let area = f.area();
        f.buffer_mut().set_style(
            area,
            Style::new().bg(look.token("background")).fg(look.token("foreground")),
        );
        let at = |n: &str| FocusId::new(n);
        let w = area.width.saturating_sub(4).min(60);
        let x = area.x + 2;
        let mut y = area.y + 1;
        f.buffer_mut().set_string(
            x,
            y,
            "New job",
            TextRole::PageTitle.style(look).fg(look.token("foreground")),
        );
        y += 2;

        self.focus.begin();
        // The fields: each a stop that takes typed text.
        self.name.focus = if self.focus.is(at(NAME)) {
            FieldFocus::Control
        } else {
            FieldFocus::None
        };
        self.schedule.focus = if self.focus.is(at(SCHEDULE)) {
            FieldFocus::Control
        } else {
            FieldFocus::None
        };
        let name = TextField::new(look, "Name")
            .placeholder("nightly-export")
            .help("Lowercase, with dashes.");
        let h = name.height(w);
        let r = Rect::new(x, y, w, h);
        self.focus.text_stop(at(NAME), r);
        (&name).render(r, f.buffer_mut(), &mut self.name);
        y += h + 1;
        let schedule = TextField::new(look, "Schedule")
            .hint("cron")
            .placeholder("0 2 * * *")
            .issues(self.schedule_issue())
            .fallback("Empty: by hand only.");
        let h = schedule.height(w);
        let r = Rect::new(x, y, w, h);
        self.focus.text_stop(at(SCHEDULE), r);
        (&schedule).render(r, f.buffer_mut(), &mut self.schedule);
        y += h + 1;

        // A text area: Enter starts a line; it grows to five.
        self.note.focus = if self.focus.is(at(NOTE)) {
            FieldFocus::Control
        } else {
            FieldFocus::None
        };
        let note = TextField::new(look, "Note")
            .multiline(3, 5)
            .help("Optional. Enter starts a line.");
        let h = note.height_with(w, &self.note);
        let r = Rect::new(x, y, w, h);
        self.focus.text_stop(at(NOTE), r);
        (&note).render(r, f.buffer_mut(), &mut self.note);
        y += h + 1;

        // The buttons: Cancel, then Create, the primary last, a column apart.
        let cancel = Button::new(look, "Cancel")
            .variant(ButtonVariant::Ghost)
            .focused(self.focus.ring(at(CANCEL)));
        let create_w = self.create().width();
        let cr = Rect::new(x + w - create_w - cancel.width() - 1, y, cancel.width(), 1);
        let kr = Rect::new(x + w - create_w, y, create_w, 1);
        self.focus.stop(at(CANCEL), cr);
        self.focus.stop(at(CREATE), kr);
        let create = self.create();
        cancel.render(cr, f.buffer_mut());
        (&create).render(kr, f.buffer_mut());
        let create_tip = create.tip();
        self.focus.end();

        if let Some(made) = &self.made {
            let style = TextRole::Body.style(look).fg(look.token("status-succeeded-ink"));
            f.buffer_mut().set_string(x, y + 2, format!("Made {made}."), style);
        }
        // The cursor, where a field has focus.
        if let Some(p) = self
            .name
            .cursor_position()
            .or(self.schedule.cursor_position())
            .or(self.note.cursor_position())
        {
            f.set_cursor_position(p);
        }

        // The status line: the focused control's tip, else the keys.
        let tip = if self.focus.is(at(CREATE)) {
            create_tip
        } else if self.focus.is(at(SCHEDULE)) {
            schedule.tip(&self.schedule).map(|words| Tip { words, shortcut: None })
        } else {
            None
        };
        let foot = StatusLine::new(look)
            .note("Tab moves focus  Esc quits", NoteTone::Plain)
            .tip(tip)
            .facts([Fact::labeled("theme", look.theme()).code()]);
        let fh = StatusLine::height(look);
        f.render_widget(
            &foot,
            Rect::new(area.x, area.bottom().saturating_sub(fh), area.width, fh),
        );
    }
}

impl app::Program for App {
    fn look(&self) -> &Look {
        &self.look
    }

    fn draw(&mut self, f: &mut ratatui::Frame) {
        App::draw(self, f);
    }

    fn key(&mut self, key: KeyPress) -> bool {
        App::key(self, key);
        self.quit
    }

    fn pointer(&mut self, p: Pointer) {
        if p.kind != PointerKind::Press {
            return;
        }
        match self.focus.press(p.at) {
            Some(id) if id == FocusId::new(CANCEL) => self.quit = true,
            Some(id) if id == FocusId::new(CREATE) => App::key(self, KeyPress::new(Key::Enter)),
            _ => {}
        }
    }

    fn tick(&mut self, now: u64) {
        App::tick(self, now);
    }

    fn done(&self) -> bool {
        self.quit
    }
}

fn main() -> std::io::Result<()> {
    let mut focus = Focus::new();
    focus.set(FocusId::new(NAME));
    let mut app = App {
        look: app::look_from_args("tidevann"),
        focus,
        name: FieldState::default(),
        schedule: FieldState::default(),
        note: FieldState::default(),
        busy_since: None,
        made: None,
        now: 0,
        quit: false,
    };
    app::run(&mut app)
}
