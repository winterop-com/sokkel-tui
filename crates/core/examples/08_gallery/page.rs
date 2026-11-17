//! The page sheet: PageState in each read, PageHeader at its widths, Section in its variants and
//! tones.

use ratatui::{Frame, layout::Rect, style::Style};
use sokkel_tui::{
    Button, ButtonVariant, Look, Outcome, PageHeader, PageState, Section, SectionTone, SectionVariant, StateTone,
    TextRole,
};

fn heading(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    let style = TextRole::SectionTitle.style(look).fg(look.token("foreground"));
    if y < f.area().bottom() {
        f.buffer_mut().set_string(x, y, words, style);
    }
}

fn note(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    let style = TextRole::Help.style(look).fg(look.token("muted-foreground"));
    if y < f.area().bottom() {
        f.buffer_mut().set_string(x, y, words, style);
    }
}

/// Lays parts down in one column, or two where the width allows.
struct Columns {
    x0: u16,
    w: u16,
    two: bool,
    y: [u16; 2],
}

impl Columns {
    fn new(area: Rect, y: u16) -> Columns {
        let two = area.width >= 100;
        let w = if two { (area.width - 8) / 2 } else { area.width - 4 };
        Columns {
            x0: area.x + 2,
            w,
            two,
            y: [y, y],
        }
    }

    /// The area for the next part of `h` rows under a note line.
    fn take(&mut self, h: u16) -> (Rect, u16) {
        let c = if self.two {
            if self.y[0] <= self.y[1] { 0 } else { 1 }
        } else {
            0
        };
        let x = self.x0 + c as u16 * (self.w + 4);
        let note_y = self.y[c];
        self.y[c] += h + 2;
        (Rect::new(x, note_y + 1, self.w, h), note_y)
    }

    fn bottom(&self) -> u16 {
        self.y[0].max(self.y[1])
    }
}

pub fn draw(f: &mut Frame, look: &Look) {
    let area = f.area();
    f.buffer_mut().set_style(
        area,
        Style::new().bg(look.token("background")).fg(look.token("foreground")),
    );
    let x0 = area.x + 2;
    let mut y = area.y + 1;

    // Page headers: one wide, one with items folding, one outside a shell with a trail and back.
    heading(f, look, x0, y, "PageHeader");
    y += 2;
    let w = area.width - 4;
    let headers: Vec<(&str, PageHeader)> = vec![
        (
            "title, count, actions",
            PageHeader::new(look, "Runs").count("1,200 runs").actions([
                Button::new(look, "Filter").variant(ButtonVariant::Secondary),
                Button::new(look, "New"),
            ]),
        ),
        (
            "described, items folding into More",
            PageHeader::new(look, "nightly-export")
                .code("job/nightly-export")
                .description("Exports the day's orders to the harbor warehouse every night at two.")
                .meta(["harbor", "ada", "every day at 02:00"])
                .items([
                    Button::new(look, "Run now"),
                    Button::new(look, "Pause"),
                    Button::new(look, "Duplicate"),
                    Button::new(look, "Delete").destructive_hover(true),
                ])
                .primary(Button::new(look, "Edit")),
        ),
        (
            "outside a shell: trail and back",
            PageHeader::new(look, "Run 4812")
                .back(true)
                .trail(["Tidevann", "Runs"], false)
                .count("failed"),
        ),
    ];
    for (name, h) in &headers {
        note(f, look, x0, y, name);
        let hh = h.height(w);
        f.render_widget(h, Rect::new(x0, y + 1, w, hh).intersection(area));
        y += hh + 2;
    }

    // Page states and sections, in columns.
    heading(f, look, x0, y, "PageState");
    y += 2;
    let mut cols = Columns::new(area, y);
    let states: Vec<(&str, PageState)> = vec![
        ("loading", PageState::loading(look, 300)),
        (
            "failed, by the server's answer",
            PageState::failed(look, Some(Outcome::Status(422)))
                .diagnostic("The schedule `0 25 * * *` is not a schedule.")
                .problems([
                    "The hour is 0 to 23.",
                    "send-digest runs at the same minute: `0 2 * * *`.",
                ])
                .retry(true),
        ),
        (
            "not answering",
            PageState::failed(look, Some(Outcome::Unreachable))
                .diagnostic("api.example.org did not answer in 30 seconds.")
                .retry(false),
        ),
        ("missing (404)", PageState::failed(look, Some(Outcome::Status(404)))),
        (
            "warning: part of the page",
            PageState::failed(look, Some(Outcome::Status(500)))
                .tone(StateTone::Warning)
                .diagnostic("The hosts could not be read; the runs are current."),
        ),
        (
            "unframed",
            PageState::failed(look, Some(Outcome::Status(503)))
                .framed(false)
                .diagnostic("The runs are being moved; try again in a minute."),
        ),
        (
            "empty",
            PageState::empty(look, "No runs yet. A job's first run shows here."),
        ),
    ];
    for (name, s) in &states {
        let (r, ny) = cols.take(s.height(cols.w));
        note(f, look, r.x, ny, name);
        f.render_widget(s, r.intersection(area));
    }
    y = cols.bottom() + 1;

    heading(f, look, x0, y, "Section");
    y += 2;
    let mut cols = Columns::new(area, y);
    let sections: Vec<(&str, Section, u16)> = vec![
        (
            "boxed, meta, actions",
            Section::new(look, "Recent runs")
                .meta("12")
                .actions([Button::new(look, "All runs").variant(ButtonVariant::Ghost)]),
            2,
        ),
        (
            "described",
            Section::new(look, "Schedule").description("When the job runs and who is told when it fails."),
            2,
        ),
        (
            "critical",
            Section::new(look, "Failing hosts")
                .tone(SectionTone::Critical)
                .meta("2"),
            2,
        ),
        ("graph", Section::new(look, "Runs by hour").tone(SectionTone::Graph), 3),
        (
            "flat",
            Section::new(look, "Settings").variant(SectionVariant::Flat).meta("4"),
            2,
        ),
        ("fill", Section::new(look, "Last runs").meta("12").fill(12, 3), 4),
    ];
    for (name, s, body) in &sections {
        let (r, ny) = cols.take(s.height(cols.w, *body));
        note(f, look, r.x, ny, name);
        f.render_widget(s, r.intersection(area));
        let b = s.body(r);
        let words = TextRole::Body.style(look).fg(look.token("muted-foreground"));
        if *name == "fill" {
            // Twelve runs in four rows' room, scrolled to the fourth.
            let words = TextRole::Body
                .style(look)
                .fg(look.token("muted-foreground"))
                .bg(look.token("card"));
            for k in 0..b.height {
                let run = ["nightly-export", "sync-inventory", "rebuild-index", "send-digest"][(k as usize + 3) % 4];
                if b.y + k < area.bottom() {
                    f.buffer_mut().set_stringn(
                        b.x,
                        b.y + k,
                        format!("{run}  {:02}:04", 12 - k - 3),
                        b.width as usize,
                        words,
                    );
                }
            }
        } else if *name != "graph" && b.y < area.bottom() {
            f.buffer_mut().set_stringn(
                b.x,
                b.y,
                "nightly-export  succeeded  6m 19s",
                b.width as usize,
                words.bg(look.token(if *name == "flat" { "background" } else { "card" })),
            );
        }
    }
}
