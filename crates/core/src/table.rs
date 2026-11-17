//! Table, from sokkel-design `parts/table`: the one table (principle 14).
//!
//! Every listing is this table: a frame, a head row that sorts by keys, rows drawn only while in
//! view, selection, tones, the quiet dash for a missing value (principle 10), and the range in its
//! foot. Below 69 columns of its own box (576 px) its rows stack as cards.
//!
//! In cells (the part's `cells` note) every row, the head and the foot are one row at both
//! densities; cells are two columns apart with a column of padding at each end; the rule under the
//! head is a row of tees in `table-head-line`. The select column is four columns. The active row is
//! marked by `focus-mark` in `ring` in its start column (open-questions T4, recommendation a). The
//! foot's range sits in the frame's bottom line, at its end.
//!
//! Built so far: the table variant with sorting, selection, tones, loading, empty, a failed read
//! and stacked cards. Not yet: the bar (search, tools), the list variant, groups, trees, detail
//! rows, row actions, paging, editing, resizing, reordering and the grid view.

use crate::cells::{PX_PER_COL, by_density, cols};
use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::page_state::PageState;
use crate::text::{self, Plural, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Widget};
use std::cmp::Ordering;
use std::collections::BTreeSet;

/// How a column takes its width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnWidth {
    /// A width in px, as the web gives a grid track: round(px / 8.4) columns.
    Px(f64),
    /// As wide as its widest drawn cell, at most `maxWidth` (320 px, 38 columns).
    Content,
    /// A share of the room left, by weight.
    Fill(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    End,
}

/// One column: its header, width, alignment and whether it sorts.
#[derive(Debug, Clone)]
pub struct Column {
    pub header: String,
    pub width: ColumnWidth,
    pub align: Align,
    pub sortable: bool,
}

impl Column {
    pub fn new(header: impl Into<String>) -> Column {
        Column {
            header: header.into(),
            width: ColumnWidth::Content,
            align: Align::Start,
            sortable: false,
        }
    }

    pub fn width(mut self, width: ColumnWidth) -> Self {
        self.width = width;
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

/// What a cell shows.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Text(String),
    /// A run state: its dot in `status-<state>` and its word in `status-<state>-ink`, so the state
    /// is said by its word, not its color alone (principle 22).
    Status(String),
    /// Not there: the quiet dash, never a blank.
    Missing,
}

/// A cell: what it shows, and what it sorts by when that is not its words.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub value: CellValue,
    pub sort: Option<f64>,
}

impl Cell {
    pub fn text(s: impl Into<String>) -> Cell {
        Cell {
            value: CellValue::Text(s.into()),
            sort: None,
        }
    }

    pub fn status(state: impl Into<String>) -> Cell {
        Cell {
            value: CellValue::Status(state.into()),
            sort: None,
        }
    }

    pub fn missing() -> Cell {
        Cell {
            value: CellValue::Missing,
            sort: None,
        }
    }

    /// `Some` text, else the dash.
    pub fn maybe(s: Option<impl Into<String>>) -> Cell {
        s.map_or_else(Cell::missing, Cell::text)
    }

    /// Sort by a number rather than the words (`sortValue`).
    pub fn sort_by(mut self, value: f64) -> Cell {
        self.sort = Some(value);
        self
    }

    fn words(&self) -> &str {
        match &self.value {
            CellValue::Text(s) | CellValue::Status(s) => s,
            CellValue::Missing => "",
        }
    }

    fn width(&self, look: &Look) -> usize {
        match &self.value {
            CellValue::Text(s) => text::width(s),
            CellValue::Status(s) => 2 + text::width(s),
            CellValue::Missing => text::width(look.glyphs().missing),
        }
    }
}

/// A row's tone (`rowTone`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowTone {
    #[default]
    Plain,
    Critical,
    Warning,
    /// The cells' content quiet.
    Muted,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub tone: RowTone,
}

impl Row {
    pub fn new(cells: impl IntoIterator<Item = Cell>) -> Row {
        Row {
            cells: cells.into_iter().collect(),
            tone: RowTone::Plain,
        }
    }

    pub fn tone(mut self, tone: RowTone) -> Self {
        self.tone = tone;
        self
    }
}

/// A column's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDir {
    Ascending,
    Descending,
}

/// The words a table says, with the spec's English defaults (`WORDS` in sokkel-ui).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableWords {
    pub rows: Plural,
    pub range: String,
    pub selected: String,
    pub empty: String,
    pub ascending: String,
    pub descending: String,
    pub unsorted: String,
    pub select_row: String,
    pub select_all: String,
    pub clear: String,
    pub loaded: String,
}

impl Default for TableWords {
    fn default() -> Self {
        TableWords {
            rows: Plural::new("{count} row", "{count} rows"),
            range: "{from}–{to} of {total}".into(),
            selected: "{count} selected".into(),
            empty: "Nothing here.".into(),
            ascending: "Ascending".into(),
            descending: "Descending".into(),
            unsorted: "As given".into(),
            select_row: "Select row".into(),
            select_all: "Select all rows".into(),
            clear: "Clear".into(),
            loaded: "{count} loaded".into(),
        }
    }
}

/// A count as sokkel-ui writes it today (`en-US`, until the locale lands): `1,200`.
pub fn count(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// What a key did to the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableEvent {
    Ignored,
    /// The active row moved.
    Moved,
    /// The order changed.
    Sorted,
    /// The selection changed.
    Selected,
    /// A row was picked (Enter, or Space where the table does not select): its index among the rows.
    Opened(usize),
}

/// What lives between frames: the active row, the scroll, the order, the selection.
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// The active row, as a place in the drawn order.
    pub active: usize,
    /// The first place drawn.
    pub offset: usize,
    pub sort: Option<(usize, SortDir)>,
    /// Selected rows, by their index among the rows given.
    pub selected: BTreeSet<usize>,
    /// The head the keys chose in the head row, while they are there.
    pub head: Option<usize>,
    /// The row picked for a detail beside the list.
    pub opened: Option<usize>,
    /// How many rows the last frame showed: a view's worth for PageUp and PageDown.
    view: usize,
    /// Whether the last frame stacked the rows as cards.
    stacked: bool,
}

impl TableState {
    pub fn sorted_by(mut self, column: usize, dir: SortDir) -> Self {
        self.sort = Some((column, dir));
        self
    }

    /// The rows in the order drawn: as given, or by the sorted column.
    pub fn order(&self, rows: &[Row]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..rows.len()).collect();
        if let Some((c, dir)) = self.sort {
            order.sort_by(|&a, &b| {
                let (x, y) = (rows[a].cells.get(c), rows[b].cells.get(c));
                let gone = |v: Option<&Cell>| v.is_none_or(|v| v.value == CellValue::Missing);
                // A missing value sorts last, whichever way.
                match (gone(x), gone(y)) {
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => {
                        let o = compare(x, y);
                        if dir == SortDir::Descending { o.reverse() } else { o }
                    }
                }
            });
        }
        order
    }

    /// The row index of the active row.
    pub fn active_row(&self, rows: &[Row]) -> Option<usize> {
        self.order(rows).get(self.active).copied()
    }

    /// Answer a key while the table has focus.
    pub fn handle(&mut self, table: &Table, key: KeyPress) -> TableEvent {
        let n = table.rows.len();
        if key.alt || key.ctrl || n == 0 && self.head.is_none() {
            return TableEvent::Ignored;
        }
        let sortable: Vec<usize> = (0..table.columns.len())
            .filter(|&c| table.columns[c].sortable)
            .collect();
        if let Some(h) = self.head {
            let at = sortable.iter().position(|&c| c == h).unwrap_or(0);
            return match key.key {
                Key::Left => {
                    self.head = Some(sortable[at.saturating_sub(1)]);
                    TableEvent::Moved
                }
                Key::Right => {
                    self.head = Some(sortable[(at + 1).min(sortable.len() - 1)]);
                    TableEvent::Moved
                }
                Key::Enter | Key::Char(' ') => {
                    self.sort = match self.sort {
                        Some((c, SortDir::Ascending)) if c == h => Some((h, SortDir::Descending)),
                        Some((c, SortDir::Descending)) if c == h => None,
                        _ => Some((h, SortDir::Ascending)),
                    };
                    TableEvent::Sorted
                }
                Key::Down => {
                    self.head = None;
                    self.active = 0;
                    TableEvent::Moved
                }
                _ => TableEvent::Ignored,
            };
        }
        let last = n.saturating_sub(1);
        let page = self.view.max(1);
        let moved = |s: &mut TableState, to: usize| {
            s.active = to.min(last);
            TableEvent::Moved
        };
        match key.key {
            Key::Down => moved(self, self.active + 1),
            Key::Up if self.active == 0 && !self.stacked && !sortable.is_empty() => {
                // Into the head row, on the sorted head, else the first sortable one.
                self.head = Some(match self.sort {
                    Some((c, _)) if sortable.contains(&c) => c,
                    _ => sortable[0],
                });
                TableEvent::Moved
            }
            Key::Up => moved(self, self.active.saturating_sub(1)),
            Key::PageDown => moved(self, self.active + page),
            Key::PageUp => moved(self, self.active.saturating_sub(page)),
            Key::Home => moved(self, 0),
            Key::End => moved(self, last),
            Key::Char(' ') if table.selectable => {
                if let Some(r) = self.active_row(table.rows)
                    && !self.selected.remove(&r)
                {
                    self.selected.insert(r);
                }
                TableEvent::Selected
            }
            Key::Char(' ') | Key::Enter => match self.active_row(table.rows) {
                Some(r) => {
                    self.opened = Some(r);
                    TableEvent::Opened(r)
                }
                None => TableEvent::Ignored,
            },
            _ => TableEvent::Ignored,
        }
    }
}

fn compare(a: Option<&Cell>, b: Option<&Cell>) -> Ordering {
    let key = |c: Option<&Cell>| c.map(|c| (c.sort, c.words().to_string()));
    match (key(a), key(b)) {
        (Some((Some(x), _)), Some((Some(y), _))) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        (Some((_, x)), Some((_, y))) => x.cmp(&y),
        _ => Ordering::Equal,
    }
}

/// The one table. Build it each frame and draw it with its [`TableState`].
#[derive(Debug, Clone)]
pub struct Table<'a> {
    look: &'a Look,
    columns: &'a [Column],
    rows: &'a [Row],
    selectable: bool,
    focused: bool,
    hovered: Option<usize>,
    total: Option<usize>,
    loading: bool,
    failed: Option<PageState<'a>>,
    words: TableWords,
}

impl<'a> Table<'a> {
    pub fn new(look: &'a Look, columns: &'a [Column], rows: &'a [Row]) -> Table<'a> {
        Table {
            look,
            columns,
            rows,
            selectable: false,
            focused: false,
            hovered: None,
            total: None,
            loading: false,
            failed: None,
            words: TableWords::default(),
        }
    }

    /// Rows select with a box.
    pub fn selectable(mut self, on: bool) -> Self {
        self.selectable = on;
        self
    }

    /// The table has keyboard focus: the active row is marked.
    pub fn focused(mut self, on: bool) -> Self {
        self.focused = on;
        self
    }

    /// The row under the pointer, by its index among the rows.
    pub fn hovered(mut self, row: Option<usize>) -> Self {
        self.hovered = row;
        self
    }

    /// How many rows there are in all, where the rows given are one page of them.
    pub fn total(mut self, total: usize) -> Self {
        self.total = Some(total);
        self
    }

    /// The first rows are loading: placeholder rows under the heads.
    pub fn loading(mut self, on: bool) -> Self {
        self.loading = on;
        self
    }

    /// The read failed: the state drawn unframed in place of the rows.
    pub fn failed(mut self, state: PageState<'a>) -> Self {
        self.failed = Some(state.framed(false).on("card"));
        self
    }

    pub fn words(mut self, words: TableWords) -> Self {
        self.words = words;
        self
    }

    /// Whether a box of `width` columns stacks the rows as cards: below 576 px (69 columns).
    pub fn stacks(width: u16) -> bool {
        (width as f64) * PX_PER_COL < 576.0
    }

    /// The range its foot says: `1–12 of 1,200`.
    pub fn range(&self) -> String {
        let n = self.rows.len();
        let total = self.total.unwrap_or(n);
        self.words
            .range
            .replace("{from}", &count(if n == 0 { 0 } else { 1 }))
            .replace("{to}", &count(n))
            .replace("{total}", &count(total))
    }

    fn select_w(&self) -> u16 {
        if self.selectable { 4 } else { 0 }
    }

    /// Each column's width in columns, across `room`.
    pub fn widths(&self, room: u16) -> Vec<u16> {
        let look = self.look;
        let gap = 2u16;
        let n = self.columns.len() as u16;
        let room = room.saturating_sub(2 + self.select_w() + gap * n.saturating_sub(1));
        let head_w = |c: &Column| text::width(&c.header) as u16 + if c.sortable { 2 } else { 0 };
        let mut w: Vec<u16> = self
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| match c.width {
                ColumnWidth::Px(px) => cols(px),
                ColumnWidth::Content => {
                    let widest = self
                        .rows
                        .iter()
                        .filter_map(|r| r.cells.get(i))
                        .map(|cell| cell.width(look) as u16)
                        .max()
                        .unwrap_or(0);
                    widest.max(head_w(c)).min(cols(320.0))
                }
                ColumnWidth::Fill(_) => head_w(c),
            })
            .collect();
        let used: u16 = w.iter().sum();
        if used <= room {
            // The room left goes to the fill columns by weight, else to the last column.
            let spare = room - used;
            let weights: Vec<u16> = self
                .columns
                .iter()
                .map(|c| if let ColumnWidth::Fill(k) = c.width { k } else { 0 })
                .collect();
            let total: u16 = weights.iter().sum();
            if total > 0 {
                let mut given = 0;
                for (i, k) in weights.iter().enumerate() {
                    let add = (spare * k).checked_div(total).unwrap_or(0);
                    w[i] += add;
                    given += add;
                }
                if let Some(i) = weights.iter().rposition(|&k| k > 0) {
                    w[i] += spare - given;
                }
            }
        } else {
            // Too narrow: the widest columns give way first, never below their head.
            let mut over = used - room;
            while over > 0 {
                let Some(i) = (0..w.len())
                    .filter(|&i| w[i] > head_w(&self.columns[i]).max(4))
                    .max_by_key(|&i| w[i])
                else {
                    break;
                };
                w[i] -= 1;
                over -= 1;
            }
        }
        w
    }

    fn row_key(&self, row: usize, state: &TableState) -> &'static str {
        let r = &self.rows[row];
        if state.selected.contains(&row) {
            "selected"
        } else if state.opened == Some(row) {
            "opened"
        } else if self.hovered == Some(row) {
            "hover"
        } else {
            match r.tone {
                RowTone::Critical => "toneCritical",
                RowTone::Warning => "toneWarning",
                RowTone::Muted => "muted",
                RowTone::Plain => "rest",
            }
        }
    }

    /// The pieces of one cell, cut to `w` columns.
    fn cell_spans(&self, cell: &Cell, w: u16, ground: Rgba, ink: Rgba) -> Vec<(String, Style)> {
        let look = self.look;
        let t = |c: Rgba| look.terminal(c);
        let glyphs = look.glyphs();
        let body = TextRole::Body.style(look).bg(t(ground));
        match &cell.value {
            CellValue::Text(s) => vec![(text::truncate(s, w as usize, glyphs.ellipsis), body.fg(t(ink)))],
            CellValue::Missing => vec![(glyphs.missing.to_string(), body.fg(look.token("faint")))],
            CellValue::Status(state) => {
                let dot = format!("status-{state}");
                let word = format!("status-{state}-ink");
                let has = |n: &str| look.has_color(n);
                let dot_ink = if has(&dot) { look.color(&dot) } else { ink };
                let word_ink = if has(&word) { look.color(&word) } else { ink };
                vec![
                    (format!("{} ", glyphs.status_dot), body.fg(t(dot_ink))),
                    (
                        text::truncate(state, (w as usize).saturating_sub(2), glyphs.ellipsis),
                        body.fg(t(word_ink)),
                    ),
                ]
            }
        }
    }
}

/// What a pointer at a position hits in a drawn table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableHit {
    /// A row, by its place in the drawn order.
    Row(usize),
    /// A row's select box, by its place.
    SelectBox(usize),
    /// The head's box: select every row, or none.
    HeadBox,
    /// A column's head, by its index.
    Head(usize),
    /// A stacked card, by its place.
    Card(usize),
}

impl TableState {
    /// The row at a place in the drawn order, by its index among the rows.
    pub fn row_at(&self, rows: &[Row], place: usize) -> Option<usize> {
        self.order(rows).get(place).copied()
    }
}

impl Table<'_> {
    /// What a pointer at `at` hits in the table drawn in `area` with `state`.
    pub fn hit(&self, area: Rect, at: Position, state: &TableState) -> Option<TableHit> {
        if !area.contains(at) || self.rows.is_empty() || self.loading || self.failed.is_some() {
            return None;
        }
        let n = self.rows.len();
        if Table::stacks(area.width) {
            let look = self.look;
            let pad_y = by_density(look, 1, 0);
            let gap = by_density(look, 1, 0);
            let card_h = 2 + 2 * pad_y + self.columns.len().max(1) as u16;
            let k = ((at.y - area.y) / (card_h + gap)) as usize;
            let within = (at.y - area.y) % (card_h + gap);
            let place = state.offset + k;
            if within >= card_h || place >= n {
                return None;
            }
            let box_row = within == 1 + pad_y;
            if self.selectable && box_row && (area.x + 2..area.x + 6).contains(&at.x) {
                return Some(TableHit::SelectBox(place));
            }
            return Some(TableHit::Card(place));
        }
        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        if !inner.contains(at) {
            return None;
        }
        let in_box = self.selectable && (inner.x + 1..inner.x + 5).contains(&at.x);
        if at.y == inner.y {
            if in_box {
                return Some(TableHit::HeadBox);
            }
            let widths = self.widths(inner.width);
            let mut x = inner.x + 1 + self.select_w();
            for (i, w) in widths.iter().enumerate() {
                if (x..x + w).contains(&at.x) {
                    return Some(TableHit::Head(i));
                }
                x += w + 2;
            }
            return None;
        }
        let body_y = inner.y + 2;
        if at.y < body_y {
            return None;
        }
        let place = state.offset + (at.y - body_y) as usize;
        if place >= n {
            return None;
        }
        Some(if in_box {
            TableHit::SelectBox(place)
        } else {
            TableHit::Row(place)
        })
    }

    /// A press on what [`Table::hit`] found: a row becomes active (and is picked where the table
    /// does not select), a box turns its row's selection over, the head's box selects every row or
    /// none, a sortable head turns its order.
    pub fn press(&self, state: &mut TableState, hit: TableHit) -> TableEvent {
        state.head = None;
        match hit {
            TableHit::Row(p) | TableHit::Card(p) => {
                state.active = p;
                match (self.selectable, state.row_at(self.rows, p)) {
                    (false, Some(r)) => {
                        state.opened = Some(r);
                        TableEvent::Opened(r)
                    }
                    _ => TableEvent::Moved,
                }
            }
            TableHit::SelectBox(p) => {
                state.active = p;
                if let Some(r) = state.row_at(self.rows, p)
                    && !state.selected.remove(&r)
                {
                    state.selected.insert(r);
                }
                TableEvent::Selected
            }
            TableHit::HeadBox => {
                if state.selected.len() >= self.rows.len() {
                    state.selected.clear();
                } else {
                    state.selected = (0..self.rows.len()).collect();
                }
                TableEvent::Selected
            }
            TableHit::Head(c) if self.columns.get(c).is_some_and(|c| c.sortable) => {
                state.sort = match state.sort {
                    Some((s, SortDir::Ascending)) if s == c => Some((c, SortDir::Descending)),
                    Some((s, SortDir::Descending)) if s == c => None,
                    _ => Some((c, SortDir::Ascending)),
                };
                TableEvent::Sorted
            }
            TableHit::Head(_) => TableEvent::Ignored,
        }
    }

    /// The wheel: the active row moves `by` rows, the view with it.
    pub fn scroll(&self, state: &mut TableState, by: isize) -> TableEvent {
        if self.rows.is_empty() {
            return TableEvent::Ignored;
        }
        let last = self.rows.len() as isize - 1;
        state.active = (state.active as isize + by).clamp(0, last) as usize;
        TableEvent::Moved
    }
}

impl ratatui::widgets::StatefulWidget for &Table<'_> {
    type State = TableState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut TableState) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height < 3 {
            return;
        }
        state.stacked = Table::stacks(area.width);
        if state.stacked {
            draw_cards(self, area, buf, state);
        } else {
            draw_table(self, area, buf, state);
        }
    }
}

fn frame(table: &Table, area: Rect, buf: &mut Buffer) -> Rect {
    let look = table.look;
    let t = |c: Rgba| look.terminal(c);
    let page = look.color("background");
    let ground = look.paint("table.base.rest.frame", "ground").unwrap_or(page);
    let edge = look.paint("table.base.rest.frame", "edge").unwrap_or(ground);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_set(look.border_set("radius-surface"))
        .border_style(Style::new().fg(t(edge)).bg(t(page)));
    let inner = block.inner(area);
    block.render(area, buf);
    buf.set_style(inner, Style::new().bg(t(ground)));
    inner
}

fn draw_table(table: &Table, area: Rect, buf: &mut Buffer, state: &mut TableState) {
    let look = table.look;
    let glyphs = look.glyphs();
    let t = |c: Rgba| look.terminal(c);
    let inner = frame(table, area, buf);
    let widths = table.widths(inner.width);
    let card = look
        .paint("table.base.rest.frame", "ground")
        .unwrap_or_else(|| look.color("card"));

    // The head row, and the rule under it as a row of tees.
    let head_ground = look.paint("table.base.rest.head", "ground").unwrap_or(card);
    let head_row = Rect::new(inner.x, inner.y, inner.width, 1);
    buf.set_style(head_row, Style::new().bg(t(head_ground)));
    if table.selectable {
        // The head's box: none, some or every row selected.
        let n = state.selected.len();
        let g = if n == 0 {
            glyphs.checkbox_off
        } else if n >= table.rows.len() {
            glyphs.checkbox_on
        } else {
            glyphs.checkbox_mixed
        };
        let ink = look
            .paint("table.base.rest.head", "ink")
            .unwrap_or_else(|| look.color("table-head-ink"));
        buf.set_string(inner.x + 1, inner.y, g, Style::new().fg(t(ink)).bg(t(head_ground)));
    }
    let mut x = inner.x + 1 + table.select_w();
    for (i, c) in table.columns.iter().enumerate() {
        let w = widths[i];
        let sorted = state.sort.filter(|(s, _)| *s == i).map(|(_, d)| d);
        let key = if sorted.is_some() { "sorted" } else { "rest" };
        let ink = look
            .paint(&format!("table.base.{key}.head"), "ink")
            .unwrap_or_else(|| look.color("table-head-ink"));
        let style = TextRole::Head.style(look).fg(t(ink)).bg(t(head_ground));
        let name = TextRole::Head.transform(look, &c.header);
        let glyph = match sorted {
            Some(SortDir::Ascending) => Some(glyphs.sort_ascending),
            Some(SortDir::Descending) => Some(glyphs.sort_descending),
            None if state.head == Some(i) => Some(glyphs.sortable),
            None => None,
        };
        let room = w.saturating_sub(if glyph.is_some() { 2 } else { 0 }) as usize;
        let name = text::truncate(&name, room, glyphs.ellipsis);
        let name_w = text::width(&name) as u16;
        let nx = if c.align == Align::End {
            x + w.saturating_sub(name_w + if glyph.is_some() { 2 } else { 0 })
        } else {
            x
        };
        buf.set_stringn(nx, inner.y, &name, w as usize, style);
        if let Some(g) = glyph {
            buf.set_string(nx + name_w + 1, inner.y, g, style);
        }
        // The head the keys chose: a ring beside its name, as a focused button.
        if table.focused && state.head == Some(i) {
            let ring = look.color("ring");
            let end = nx + name_w + if glyph.is_some() { 2 } else { 0 };
            if nx > inner.x {
                buf[(nx - 1, inner.y)].set_symbol(glyphs.ring_start).set_fg(t(ring));
            }
            if end < inner.right() {
                buf[(end, inner.y)].set_symbol(glyphs.ring_end).set_fg(t(ring));
            }
        }
        x += w + 2;
    }
    let rule_y = inner.y + 1;
    if rule_y < inner.bottom() {
        let bx = look.box_glyphs("radius-surface");
        let line = look.paint("table.base.rest.head", "rule").unwrap_or(card);
        let page = look.color("background");
        let st = Style::new().fg(t(line)).bg(t(card));
        buf[(area.x, rule_y)].set_symbol(bx.tee_start).set_style(st.bg(t(page)));
        for x in inner.x..inner.right() {
            buf[(x, rule_y)].set_symbol(bx.across).set_style(st);
        }
        buf[(area.right() - 1, rule_y)]
            .set_symbol(bx.tee_end)
            .set_style(st.bg(t(page)));
    }
    let body = Rect {
        y: inner.y + 2,
        height: inner.height.saturating_sub(2),
        ..inner
    };
    state.view = body.height as usize;
    if body.height == 0 {
        return;
    }

    if let Some(failed) = &table.failed {
        // Padded `density.panel-pad` across and 12 down: one blank row at regular.
        let pad_y = by_density(look, 1, 0);
        let r = Rect {
            y: body.y + pad_y,
            height: body.height.saturating_sub(pad_y),
            ..body
        };
        failed.render(r, buf);
        return;
    }
    if table.loading {
        // A placeholder bar per cell at row height, filling the view.
        let bar = look.paint("table.base.rest.placeholder", "ground").unwrap_or(card);
        for y in body.y..body.bottom() {
            let mut x = body.x + 1 + table.select_w();
            for w in &widths {
                let len = (*w * 6 / 10).max(1);
                buf.set_style(Rect::new(x, y, len, 1), Style::new().bg(t(bar)));
                x += w + 2;
            }
        }
        return;
    }
    if table.rows.is_empty() {
        let ink = look
            .paint("table.base.rest.empty", "ink")
            .unwrap_or_else(|| look.color("muted-foreground"));
        buf.set_stringn(
            body.x + 1,
            body.y,
            &table.words.empty,
            body.width.saturating_sub(2) as usize,
            TextRole::Body.style(look).fg(t(ink)),
        );
        return;
    }

    // The rows in view: the active one kept in it.
    let order = state.order(table.rows);
    state.active = state.active.min(order.len() - 1);
    let view = body.height as usize;
    if state.active < state.offset {
        state.offset = state.active;
    } else if state.active >= state.offset + view {
        state.offset = state.active + 1 - view;
    }
    state.offset = state.offset.min(order.len().saturating_sub(view));
    for (k, &row) in order.iter().enumerate().skip(state.offset).take(view) {
        let y = body.y + (k - state.offset) as u16;
        let key = format!("table.base.{}.row", table.row_key(row, state));
        let mut ground = look.paint(&key, "ground").unwrap_or(card);
        if key == "table.base.rest.row" && k % 2 == 1 {
            ground = look.color("table-zebra");
        }
        let mut ink = look.paint(&key, "ink").unwrap_or_else(|| look.color("foreground"));
        if table.rows[row].tone == RowTone::Muted && key == "table.base.muted.row" {
            ink = look.color("muted-foreground");
        }
        buf.set_style(Rect::new(body.x, y, body.width, 1), Style::new().bg(t(ground)));
        if table.focused && state.head.is_none() && k == state.active {
            buf[(body.x, y)]
                .set_symbol(glyphs.focus_mark)
                .set_style(Style::new().fg(look.token("ring")).bg(t(ground)));
        }
        let mut x = body.x + 1;
        if table.selectable {
            let g = if state.selected.contains(&row) {
                glyphs.checkbox_on
            } else {
                glyphs.checkbox_off
            };
            buf.set_string(x, y, g, Style::new().fg(t(ink)).bg(t(ground)));
            x += 4;
        }
        for (i, c) in table.columns.iter().enumerate() {
            let w = widths[i];
            let cell = table.rows[row].cells.get(i).cloned().unwrap_or_else(Cell::missing);
            let spans = table.cell_spans(&cell, w, ground, ink);
            let cw: u16 = spans.iter().map(|(s, _)| text::width(s) as u16).sum();
            let mut cx = if c.align == Align::End {
                x + w.saturating_sub(cw)
            } else {
                x
            };
            for (s, st) in spans {
                buf.set_stringn(cx, y, &s, (x + w).saturating_sub(cx) as usize, st);
                cx += text::width(&s) as u16;
            }
            x += w + 2;
        }
    }

    // The foot: the range at the end of the frame's bottom line.
    let range = format!(" {} ", table.range());
    let rw = text::width(&range) as u16;
    if rw + 4 < area.width {
        let ink = look
            .paint("table.base.rest.empty", "ink")
            .unwrap_or_else(|| look.color("muted-foreground"));
        let page = look.color("background");
        buf.set_string(
            area.right() - 2 - rw,
            area.bottom() - 1,
            &range,
            TextRole::Help.style(look).fg(t(ink)).bg(t(page)),
        );
    }
}

/// Below its stack width: each row a card, its first column the title and every other a line of
/// its name and value.
fn draw_cards(table: &Table, area: Rect, buf: &mut Buffer, state: &mut TableState) {
    let look = table.look;
    let glyphs = look.glyphs();
    let t = |c: Rgba| look.terminal(c);
    let page = look.color("background");
    let pad_y = by_density(look, 1, 0);
    let gap = by_density(look, 1, 0);
    let lines = table.columns.len().max(1) as u16;
    let card_h = 2 + 2 * pad_y + lines;
    if table.rows.is_empty() || table.loading || table.failed.is_some() {
        // The states draw as the table draws them, in a frame.
        draw_table(table, area, buf, state);
        return;
    }
    let order = state.order(table.rows);
    state.active = state.active.min(order.len() - 1);
    let per_view = ((area.height + gap) / (card_h + gap)).max(1) as usize;
    state.view = per_view;
    if state.active < state.offset {
        state.offset = state.active;
    } else if state.active >= state.offset + per_view {
        state.offset = state.active + 1 - per_view;
    }
    let name_w = table
        .columns
        .iter()
        .skip(1)
        .map(|c| text::width(&c.header))
        .max()
        .unwrap_or(0)
        .min(cols(192.0) as usize) as u16;
    for (k, &row) in order.iter().enumerate().skip(state.offset).take(per_view) {
        let y = area.y + (k - state.offset) as u16 * (card_h + gap);
        if y + card_h > area.bottom() {
            break;
        }
        let rect = Rect::new(area.x, y, area.width, card_h);
        let r = &table.rows[row];
        let st = if state.selected.contains(&row) {
            "selected"
        } else if state.opened == Some(row) {
            "opened"
        } else if table.hovered == Some(row) {
            "hover"
        } else {
            match r.tone {
                RowTone::Critical => "toneCritical",
                RowTone::Warning => "toneWarning",
                _ => "rest",
            }
        };
        let key = format!("table.base.{st}.card");
        let ground = look.paint(&key, "ground").unwrap_or_else(|| look.color("card"));
        let mut edge = look.paint(&key, "edge").unwrap_or(ground);
        // A ring is the card's edge in cells: the active card's in `ring`, a selected one's in its paint.
        if let Some(ring) = look.ring(&key, "ring") {
            edge = ring;
        }
        if table.focused && k == state.active {
            edge = look.color("ring");
        }
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(look.border_set("radius-surface"))
            .border_style(Style::new().fg(t(edge)).bg(t(page)));
        let inner = block.inner(rect);
        block.render(rect, buf);
        buf.set_style(inner, Style::new().bg(t(ground)));
        let ink = look.color("foreground");
        let quiet = look.color("muted-foreground");
        let x = inner.x + 1;
        let w = inner.width.saturating_sub(2);
        let mut ly = inner.y + pad_y;
        let mut tx = x;
        if table.selectable {
            let g = if state.selected.contains(&row) {
                glyphs.checkbox_on
            } else {
                glyphs.checkbox_off
            };
            buf.set_string(tx, ly, g, Style::new().fg(t(ink)).bg(t(ground)));
            tx += 4;
        }
        let title = r.cells.first().cloned().unwrap_or_else(Cell::missing);
        let title_w = w.saturating_sub(tx - x);
        for (s, style) in table.cell_spans(&title, title_w, ground, ink) {
            buf.set_stringn(
                tx,
                ly,
                &s,
                (x + w).saturating_sub(tx) as usize,
                style.patch(TextRole::Label.style(look)),
            );
            tx += text::width(&s) as u16;
        }
        ly += 1;
        for (i, c) in table.columns.iter().enumerate().skip(1) {
            let name = text::truncate(&c.header, name_w as usize, glyphs.ellipsis);
            buf.set_string(x, ly, &name, TextRole::Help.style(look).fg(t(quiet)).bg(t(ground)));
            let cell = r.cells.get(i).cloned().unwrap_or_else(Cell::missing);
            let mut vx = x + name_w + 2;
            for (s, style) in table.cell_spans(&cell, w.saturating_sub(name_w + 2), ground, ink) {
                buf.set_stringn(vx, ly, &s, (x + w).saturating_sub(vx) as usize, style);
                vx += text::width(&s) as u16;
            }
            ly += 1;
        }
    }
}
