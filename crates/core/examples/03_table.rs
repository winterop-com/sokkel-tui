//! 03: the one table. Forty runs: sorting by the heads, selecting, tones, the quiet dash for what
//! is not there, only the rows in view drawn, and cards below 69 columns of its own box (narrow the
//! terminal to see them).
//!
//! Usage: cargo run --example 03_table [-- light]
//! Keys: Up, Down, PgUp, PgDn, Home, End move; Space selects; Up from the first row enters the
//! head row, where Left and Right choose a head and Enter turns its order; Enter on a row picks it.
//! The mouse: a press on a row, a box or a head; the wheel. Ctrl+Q quits.

#[path = "common/mod.rs"]
mod common;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use sokkel_tui::{
    Align, Cell, Column, ColumnWidth, Fact, Key, KeyPress, Look, NoteTone, Pointer, PointerKind, Row, RowTone, SortDir,
    StatusLine, Table, TableEvent, TableHit, TableState,
};

const JOBS: [&str; 5] = [
    "nightly-export",
    "sync-inventory",
    "rebuild-index",
    "send-digest",
    "import-orders",
];
const STATES: [&str; 6] = ["succeeded", "succeeded", "running", "failed", "queued", "cancelled"];
const PEOPLE: [Option<&str>; 5] = [Some("ada"), Some("grace"), Some("linus"), None, Some("margaret")];
const PROJECTS: [&str; 3] = ["harbor", "juniper", "larkspur"];

fn columns() -> Vec<Column> {
    vec![
        Column::new("Job").width(ColumnWidth::Fill(3)).sortable(),
        Column::new("State").sortable(),
        Column::new("Started by").sortable(),
        Column::new("Project").sortable(),
        Column::new("Started").sortable(),
        Column::new("Took").align(Align::End).sortable(),
    ]
}

fn rows() -> Vec<Row> {
    (0..40)
        .map(|i| {
            let state = STATES[(i * 7) % STATES.len()];
            let mins = (i * 13 + 5) % 60;
            let took = (state == "succeeded" || state == "failed").then(|| {
                let secs = 30 + i * 37 % 900;
                Cell::text(format!("{}m {:02}s", secs / 60, secs % 60)).sort_by(secs as f64)
            });
            let tone = if state == "failed" {
                RowTone::Critical
            } else {
                RowTone::Plain
            };
            Row::new([
                Cell::text(JOBS[i % 5]),
                Cell::status(state),
                Cell::maybe(PEOPLE[(i * 3) % 5]),
                Cell::text(PROJECTS[i % 3]),
                Cell::text(format!("{:02}:{:02}", 12 - (i / 4), mins)),
                took.unwrap_or_else(Cell::missing),
            ])
            .tone(tone)
        })
        .collect()
}

struct App {
    look: Look,
    columns: Vec<Column>,
    rows: Vec<Row>,
    state: TableState,
    area: Rect,
    hover: Option<ratatui::layout::Position>,
    picked: Option<String>,
}

impl App {
    fn table(&self) -> Table<'_> {
        Table::new(&self.look, &self.columns, &self.rows)
            .selectable(true)
            .focused(true)
    }

    fn picked(&mut self, event: TableEvent) {
        if let TableEvent::Opened(r) = event {
            self.picked = Some(format!("Picked run {} ({})", r + 1, JOBS[r % 5]));
        }
    }
}

impl common::App for App {
    fn look(&self) -> &Look {
        &self.look
    }

    fn key(&mut self, key: KeyPress) -> bool {
        if key.is(Key::Escape) && self.state.head.is_none() {
            return true;
        }
        // Enter on a row picks it, where Space selects.
        if key.is(Key::Enter) && self.state.head.is_none() {
            let r = self.state.active_row(&self.rows);
            self.picked = r.map(|r| format!("Picked run {} ({})", r + 1, JOBS[r % 5]));
            return false;
        }
        let mut state = std::mem::take(&mut self.state);
        let event = state.handle(&self.table(), key);
        self.state = state;
        self.picked(event);
        false
    }

    fn pointer(&mut self, p: Pointer) {
        let mut state = std::mem::take(&mut self.state);
        let table = self.table();
        match p.kind {
            PointerKind::Press => {
                if let Some(hit) = table.hit(self.area, p.at, &state) {
                    let event = table.press(&mut state, hit);
                    self.picked(event);
                }
            }
            PointerKind::ScrollUp => {
                table.scroll(&mut state, -3);
            }
            PointerKind::ScrollDown => {
                table.scroll(&mut state, 3);
            }
            PointerKind::Move => self.hover = Some(p.at),
        }
        self.state = state;
    }

    fn draw(&mut self, f: &mut Frame) {
        let look = &self.look;
        let area = f.area();
        f.buffer_mut().set_style(
            area,
            Style::new().bg(look.token("background")).fg(look.token("foreground")),
        );
        let foot_h = StatusLine::height(look);
        self.area = Rect::new(
            area.x + 1,
            area.y,
            area.width.saturating_sub(2),
            area.height.saturating_sub(foot_h),
        );
        let hovered = self
            .hover
            .and_then(|at| self.table().hit(self.area, at, &self.state))
            .and_then(|h| match h {
                TableHit::Row(p) | TableHit::Card(p) | TableHit::SelectBox(p) => self.state.row_at(&self.rows, p),
                _ => None,
            });
        let table = Table::new(look, &self.columns, &self.rows)
            .selectable(true)
            .focused(true)
            .hovered(hovered)
            .total(1200);
        f.render_stateful_widget(&table, self.area, &mut self.state);

        let sort = match self.state.sort {
            Some((c, d)) => format!(
                "{} {}",
                self.columns[c].header,
                if d == SortDir::Ascending {
                    "ascending"
                } else {
                    "descending"
                }
            ),
            None => "as given".into(),
        };
        let note = self
            .picked
            .clone()
            .unwrap_or_else(|| "Space selects, Enter picks".into());
        let foot = StatusLine::new(look).note(note, NoteTone::Plain).facts([
            Fact::labeled("sort", sort),
            Fact::labeled("selected", self.state.selected.len().to_string()),
            Fact::labeled("row", (self.state.active + 1).to_string()),
        ]);
        f.render_widget(
            &foot,
            Rect::new(area.x, area.bottom().saturating_sub(foot_h), area.width, foot_h),
        );
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        look: common::look(),
        columns: columns(),
        rows: rows(),
        state: TableState::default(),
        area: Rect::default(),
        hover: None,
        picked: None,
    };
    common::run(&mut app)
}
