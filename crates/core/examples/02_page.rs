//! 02: a page. Its header (a title with its count, a code, a description, actions), and a read
//! that lands in one of its states: loading, failed (headed by what the server answered), empty, or
//! the content, here two sections.
//!
//! Usage: cargo run --example 02_page [-- light]
//! Keys: 1 loading, 2 failed, 3 empty, 4 the content; Tab moves between Refresh and, in the failed
//! state, Try again; Enter presses it (a read takes a moment). A press does the same. Ctrl+Q quits.

#[path = "common/mod.rs"]
mod common;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use sokkel_tui::{
    Button, ButtonVariant, Fact, Key, KeyPress, Look, NoteTone, Outcome, PageHeader, PageState, Pointer, PointerKind,
    Section, SectionTone, StatusLine, TextRole,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Read {
    Loading,
    Failed,
    Empty,
    Content,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    Refresh,
    Retry,
}

struct Page {
    look: Look,
    read: Read,
    /// A read under way since, and where it lands.
    reading: Option<(u64, Read)>,
    focus: Stop,
    now: u64,
    refresh_at: Option<Rect>,
    retry_at: Option<Rect>,
}

impl Page {
    fn refresh(&mut self, lands: Read) {
        self.read = Read::Loading;
        self.reading = Some((self.now, lands));
    }

    fn press(&mut self, stop: Stop) {
        match stop {
            Stop::Refresh => self.refresh(Read::Content),
            Stop::Retry => self.refresh(Read::Content),
        }
    }
}

impl common::App for Page {
    fn look(&self) -> &Look {
        &self.look
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        if let Some((since, lands)) = self.reading
            && now - since > 900
        {
            self.read = lands;
            self.reading = None;
        }
    }

    fn key(&mut self, key: KeyPress) -> bool {
        match key.key {
            Key::Char('1') => self.read = Read::Loading,
            Key::Char('2') => self.read = Read::Failed,
            Key::Char('3') => self.read = Read::Empty,
            Key::Char('4') => self.read = Read::Content,
            Key::Tab | Key::BackTab if self.read == Read::Failed => {
                self.focus = match self.focus {
                    Stop::Refresh => Stop::Retry,
                    Stop::Retry => Stop::Refresh,
                }
            }
            Key::Enter | Key::Char(' ') => self.press(self.focus),
            Key::Escape => return true,
            _ => {}
        }
        if self.read != Read::Failed {
            self.focus = Stop::Refresh;
        }
        false
    }

    fn pointer(&mut self, p: Pointer) {
        if p.kind != PointerKind::Press {
            return;
        }
        if self.refresh_at.is_some_and(|r| r.contains(p.at)) {
            self.focus = Stop::Refresh;
            self.press(Stop::Refresh);
        } else if self.retry_at.is_some_and(|r| r.contains(p.at)) {
            self.focus = Stop::Retry;
            self.press(Stop::Retry);
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

        // The header.
        let header = PageHeader::new(look, "nightly-export")
            .code("job/nightly-export")
            .description("Exports the day's orders to the harbor warehouse every night at two.")
            .meta(["harbor", "ada", "every day at 02:00"])
            .actions([Button::new(look, "Refresh")
                .variant(ButtonVariant::Outline)
                .busy(self.reading.is_some(), self.now)
                .focused(self.focus == Stop::Refresh)]);
        let hh = header.height(page.width);
        let hr = Rect { height: hh, ..page };
        self.refresh_at = header.layout(hr).actions.first().map(|(_, r)| *r);
        f.render_widget(&header, hr);

        // The read, in its state.
        let body = Rect {
            y: page.y + hh + 1,
            height: page.height.saturating_sub(hh + 1),
            ..page
        };
        self.retry_at = None;
        let state = match self.read {
            Read::Loading => Some(PageState::loading(look, self.now)),
            Read::Failed => Some(
                PageState::failed(look, Some(Outcome::Unreachable))
                    .diagnostic("api.example.org did not answer in 30 seconds.")
                    .problems(["The last run's report is from `11:58`."])
                    .retry(self.focus == Stop::Retry),
            ),
            Read::Empty => Some(PageState::empty(look, "No runs yet. The first runs tonight at 02:00.")),
            Read::Content => None,
        };
        match state {
            Some(s) => {
                let r = Rect {
                    height: s.height(body.width).min(body.height),
                    ..body
                };
                self.retry_at = s.retry_area(r);
                f.render_widget(&s, r);
            }
            None => {
                let recent = Section::new(look, "Last runs").meta("3");
                let rows = ["12:04  running", "11:58  failed", "10:04  succeeded  6m 19s"];
                let h = recent.height(body.width, rows.len() as u16);
                let r = Rect { height: h, ..body };
                (&recent).render(r, f.buffer_mut());
                let b = recent.body(r);
                for (k, line) in rows.iter().enumerate() {
                    f.buffer_mut()
                        .set_string(b.x, b.y + k as u16, line, TextRole::Body.style(look));
                }
                let failing = Section::new(look, "Failing hosts")
                    .tone(SectionTone::Critical)
                    .meta("1");
                let r2 = Rect {
                    y: r.bottom() + 1,
                    height: failing
                        .height(body.width, 1)
                        .min(body.bottom().saturating_sub(r.bottom() + 1)),
                    ..body
                };
                (&failing).render(r2, f.buffer_mut());
                let b2 = failing.body(r2);
                f.buffer_mut().set_string(
                    b2.x,
                    b2.y,
                    "api.example.org: no answer since 11:58",
                    TextRole::Body.style(look),
                );
            }
        }

        let note = match self.read {
            Read::Loading => "Loading",
            Read::Failed => "Failed: Try again reads again",
            Read::Empty => "Empty",
            Read::Content => "The content",
        };
        let foot = StatusLine::new(look)
            .note(note, NoteTone::Plain)
            .facts([Fact::labeled("1 2 3 4", "states"), Fact::labeled("Ctrl Q", "quit")]);
        f.render_widget(
            &foot,
            Rect::new(area.x, area.bottom().saturating_sub(foot_h), area.width, foot_h),
        );
    }
}

fn main() -> std::io::Result<()> {
    let mut page = Page {
        look: common::look(),
        read: Read::Loading,
        reading: None,
        focus: Stop::Refresh,
        now: 0,
        refresh_at: None,
        retry_at: None,
    };
    page.reading = Some((0, Read::Content));
    common::run(&mut page)
}
