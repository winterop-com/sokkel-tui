//! 03: the one table. Forty runs: sorting by the heads, selecting, tones, the quiet dash for what
//! is not there, only the rows in view drawn, and cards below 69 columns of its own box (narrow the
//! terminal to see them).
//!
//! Usage: cargo run --example 03_table [-- light]
//! Keys: Up, Down, PgUp, PgDn, Home, End move; Space selects; Up from the first row enters the
//! head row, where Left and Right choose a head and Enter turns its order; Enter on a row picks it.
//! `/` searches the jobs in the bar (Enter or Escape leaves the search); `l` switches between the
//! table and the list variant. The mouse: a press on a row, a box, a head, the search or Clear; the
//! wheel; the pager's arrows at the foot. Ctrl+Q quits.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use sokkel_tui::app;
use sokkel_tui::{
    Align, Cell, CellValue, Column, ColumnWidth, Fact, Key, KeyPress, Look, NoteTone, Pointer, PointerKind, Row,
    RowTone, SortDir, StatusLine, Table, TableEvent, TableHit, TableState, TableVariant,
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

/// A page of forty runs, counted from 1: each page the runs before the last.
fn rows(page: usize) -> Vec<Row> {
    let base = (page - 1) * 40;
    (base..base + 40)
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
                Cell::text(format!("{:02}:{:02}", (23 - (i / 4) % 24), mins)),
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
    query: String,
    searching: bool,
    list: bool,
    all: Vec<Row>,
    /// The page shown, counted from 1, of the 1,200 runs forty a page.
    page: usize,
}

impl App {
    fn table(&self) -> Table<'_> {
        Table::new(&self.look, &self.columns, &self.rows)
            .selectable(true)
            .focused(!self.searching)
            .bar(self.query.clone(), "Search the jobs", self.searching)
            .paging(self.page, 30)
            .starting_at(if self.query.is_empty() { (self.page - 1) * 40 } else { 0 })
            .variant(if self.list {
                TableVariant::List
            } else {
                TableVariant::Table
            })
    }

    /// The rows the search leaves; a new search starts the selection over.
    fn narrow(&mut self) {
        let q = self.query.to_lowercase();
        self.rows = self
            .all
            .iter()
            .filter(|r| match &r.cells[0].value {
                CellValue::Text(job) => job.contains(&q),
                _ => true,
            })
            .cloned()
            .collect();
        self.state.selected.clear();
        self.state.active = 0;
    }

    fn picked(&mut self, event: TableEvent) {
        match event {
            TableEvent::Opened(r) => self.picked = Some(format!("Picked run {} ({})", r + 1, JOBS[r % 5])),
            // The pager asks for a page: its forty runs, the search kept.
            TableEvent::Page(p) => {
                self.page = p;
                self.all = rows(p);
                self.narrow();
                self.picked = Some(format!("Page {p}"));
            }
            _ => {}
        }
    }
}

impl app::Program for App {
    fn look(&self) -> &Look {
        &self.look
    }

    fn key(&mut self, key: KeyPress) -> bool {
        if self.searching {
            match key.key {
                Key::Char(c) if !key.ctrl => {
                    self.query.push(c);
                    self.narrow();
                }
                Key::Backspace => {
                    self.query.pop();
                    self.narrow();
                }
                Key::Enter | Key::Escape | Key::Tab => self.searching = false,
                _ => {}
            }
            return false;
        }
        match key.key {
            Key::Char('/') => {
                self.searching = true;
                return false;
            }
            Key::Char('l') => {
                self.list = !self.list;
                return false;
            }
            _ => {}
        }
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
                let hit = table.hit(self.area, p.at, &state);
                let event = hit.map(|h| table.press(&mut state, h));
                self.searching = hit == Some(TableHit::Search);
                if let Some(event) = event {
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
        let mut state = std::mem::take(&mut self.state);
        // The whole set is 1,200 runs, forty of them here; a search counts what it found.
        let total = if self.query.is_empty() { 1200 } else { self.rows.len() };
        let table = self.table().hovered(hovered).total(total);
        f.render_stateful_widget(&table, self.area, &mut state);
        self.state = state;

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
            Fact::labeled("l", if self.list { "list" } else { "table" }),
        ]);
        f.render_widget(
            &foot,
            Rect::new(area.x, area.bottom().saturating_sub(foot_h), area.width, foot_h),
        );
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        look: app::look_from_args("tidevann"),
        columns: columns(),
        rows: rows(1),
        all: rows(1),
        page: 1,
        query: String::new(),
        searching: false,
        list: false,
        state: TableState::default(),
        area: Rect::default(),
        hover: None,
        picked: None,
    };
    app::run(&mut app)
}
