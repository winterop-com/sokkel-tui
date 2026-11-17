//! The table sheet: the one table selecting, sorting by keys, loading, empty, failed, and stacked
//! as cards below its stack width.

use ratatui::{Frame, layout::Rect, style::Style};
use sokkel_tui::{
    Align, Cell, Column, ColumnWidth, Look, Outcome, PageState, Row, RowTone, SortDir, Table, TableState, TextRole,
};

fn note(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    if y < f.area().bottom() {
        let style = TextRole::Help.style(look).fg(look.token("muted-foreground"));
        f.buffer_mut().set_string(x, y, words, style);
    }
}

fn heading(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    if y < f.area().bottom() {
        let style = TextRole::SectionTitle.style(look).fg(look.token("foreground"));
        f.buffer_mut().set_string(x, y, words, style);
    }
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("Job").width(ColumnWidth::Fill(1)).sortable(),
        Column::new("State").sortable(),
        Column::new("Started by").sortable(),
        Column::new("Took").align(Align::End).sortable(),
    ]
}

fn rows() -> Vec<Row> {
    let run = |job: &str, state: &str, by: Option<&str>, took: Option<(&str, f64)>| {
        Row::new([
            Cell::text(job),
            Cell::status(state),
            Cell::maybe(by),
            took.map_or_else(Cell::missing, |(s, v)| Cell::text(s).sort_by(v)),
        ])
    };
    vec![
        run("nightly-export", "running", Some("ada"), None),
        run("sync-inventory", "failed", Some("grace"), Some(("2m 41s", 161.0))).tone(RowTone::Critical),
        run("rebuild-index", "succeeded", Some("linus"), Some(("14m 02s", 842.0))),
        run("send-digest", "queued", None, None),
        run("import-orders", "succeeded", Some("margaret"), Some(("48s", 48.0))),
        run(
            "nightly-export",
            "completed-with-errors",
            Some("ada"),
            Some(("6m 19s", 379.0)),
        )
        .tone(RowTone::Warning),
    ]
}

pub fn draw(f: &mut Frame, look: &Look) {
    let area = f.area();
    f.buffer_mut().set_style(
        area,
        Style::new().bg(look.token("background")).fg(look.token("foreground")),
    );
    let x0 = area.x + 2;
    let w = area.width - 4;
    let mut y = area.y + 1;
    heading(f, look, x0, y, "Table");
    y += 2;
    let columns = columns();
    let rows = rows();

    let place = |f: &mut Frame, name: &str, h: u16, table: &Table, state: &mut TableState, y: &mut u16| {
        note(f, look, x0, *y, name);
        let r = Rect::new(x0, *y + 1, w, h).intersection(area);
        f.render_stateful_widget(table, r, state);
        *y += h + 2;
    };

    // Selecting, the active row marked, sorted by how long the runs took.
    let table = Table::new(look, &columns, &rows).selectable(true).focused(true);
    let mut state = TableState::default().sorted_by(3, SortDir::Descending);
    state.selected.extend([2, 4]);
    state.active = 1;
    place(
        f,
        "selectable, sorted by Took, two selected, the active row marked",
        10,
        &table,
        &mut state,
        &mut y,
    );

    // The head row chosen by keys: Up from the first row.
    let table = Table::new(look, &columns, &rows).focused(true).hovered(Some(4));
    let mut state = TableState::default();
    state.head = Some(2);
    place(
        f,
        "the head row by keys (Started by chosen), a row hovered",
        10,
        &table,
        &mut state,
        &mut y,
    );

    let empty: Vec<Row> = Vec::new();
    let table = Table::new(look, &columns, &empty).loading(true);
    place(f, "loading", 6, &table, &mut TableState::default(), &mut y);
    let table = Table::new(look, &columns, &empty);
    place(f, "empty", 5, &table, &mut TableState::default(), &mut y);
    let failed = PageState::failed(look, Some(Outcome::Unreachable))
        .diagnostic("api.example.org did not answer in 30 seconds.")
        .retry(false);
    let table = Table::new(look, &columns, &empty).failed(failed);
    place(f, "failed", 10, &table, &mut TableState::default(), &mut y);

    // Below its stack width (69 columns of its own box): cards.
    let narrow = 48.min(w);
    note(f, look, x0, y, "stacked: its own box 48 columns wide");
    let table = Table::new(look, &columns, &rows).selectable(true).focused(true);
    let mut state = TableState::default();
    state.selected.insert(1);
    let h = area.bottom().saturating_sub(y + 1);
    f.render_stateful_widget(&table, Rect::new(x0, y + 1, narrow, h).intersection(area), &mut state);
}
