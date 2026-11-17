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

use crate::button::{Button, ButtonVariant};
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
use std::collections::{BTreeMap, BTreeSet};

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
    /// Its cells edit as text in a table with [`Table::editing`] (`edit`).
    pub editable: bool,
    /// Its head's end edge resizes it (`resizable`).
    pub resizable: bool,
}

impl Column {
    pub fn new(header: impl Into<String>) -> Column {
        Column {
            header: header.into(),
            width: ColumnWidth::Content,
            align: Align::Start,
            sortable: false,
            editable: false,
            resizable: false,
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

    /// Its head's end edge resizes it: dragged, or with `<` and `>` while its head is chosen in
    /// the head row (`=` puts its width back).
    pub fn resizable(mut self) -> Self {
        self.resizable = true;
        self
    }

    /// Its cells edit as text, in a table with [`Table::editing`].
    pub fn editable(mut self) -> Self {
        self.editable = true;
        self
    }
}

/// What a cell shows.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Text(String),
    /// A state in words the app chooses (a check's ok, warn, fail), its dot and word in a tone's
    /// colors: for states the contract's run vocabulary does not hold.
    Toned(String, StatusTone),
    /// A run state: its dot in `status-<state>` and its word in `status-<state>-ink`, so the state
    /// is said by its word, not its color alone (principle 22).
    Status(String),
    /// Not there: the quiet dash, never a blank.
    Missing,
}

/// The tone of a state the app names itself: its dot in the tone's color token and its word in
/// the tone's ink (`good` and `good-ink`, `warning`, `critical`, `info`; quiet in
/// `muted-foreground`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusTone {
    Good,
    Warning,
    Critical,
    Info,
    Quiet,
}

impl StatusTone {
    /// The dot's color and the word's ink, as tokens.
    pub fn tokens(self) -> (&'static str, &'static str) {
        match self {
            StatusTone::Good => ("good", "good-ink"),
            StatusTone::Warning => ("warning", "warning-ink"),
            StatusTone::Critical => ("critical", "critical-ink"),
            StatusTone::Info => ("info", "info-ink"),
            StatusTone::Quiet => ("muted-foreground", "muted-foreground"),
        }
    }

    /// How bad it is, for the worst of several: critical over warning over info over good.
    pub fn rank(self) -> u8 {
        match self {
            StatusTone::Critical => 4,
            StatusTone::Warning => 3,
            StatusTone::Info => 2,
            StatusTone::Good => 1,
            StatusTone::Quiet => 0,
        }
    }
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

    /// A state the app names (ok, warn, fail), its dot and word in `tone`'s colors.
    pub fn toned(state: impl Into<String>, tone: StatusTone) -> Cell {
        Cell {
            value: CellValue::Toned(state.into(), tone),
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

    /// The words the cell shows (a missing value's are empty).
    pub fn words(&self) -> &str {
        match &self.value {
            CellValue::Text(s) | CellValue::Status(s) | CellValue::Toned(s, _) => s,
            CellValue::Missing => "",
        }
    }

    fn width(&self, look: &Look) -> usize {
        match &self.value {
            CellValue::Text(s) => text::width(s),
            CellValue::Status(s) | CellValue::Toned(s, _) => 2 + text::width(s),
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
    /// The row's detail (`detail`): labeled lines it opens under itself.
    pub detail: Vec<(String, String)>,
    /// Its depth in a tree (`childRows`): a row deeper than the one before it is that row's child.
    pub depth: u8,
}

impl Row {
    pub fn new(cells: impl IntoIterator<Item = Cell>) -> Row {
        Row {
            cells: cells.into_iter().collect(),
            tone: RowTone::Plain,
            detail: Vec::new(),
            depth: 0,
        }
    }

    /// A child of the row before it at a lesser depth (`childRows`): shown while that row is open,
    /// its first cell set in under its parent's.
    pub fn child(self) -> Self {
        self.depth(1)
    }

    /// Its depth in the tree: 0 a root, 1 a child, 2 a grandchild.
    pub fn depth(mut self, depth: u8) -> Self {
        self.depth = depth;
        self
    }

    /// A detail the row opens under itself: Enter, Right or a press opens it, Left closes it.
    pub fn detail(mut self, lines: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        self.detail = lines.into_iter().map(|(l, v)| (l.into(), v.into())).collect();
        self
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
    pub page: String,
    pub load_more: String,
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
            page: "Page {page} of {pages}".into(),
            load_more: "Load more".into(),
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
    /// A cell's edit was saved (Enter or Tab): the row's index among the rows and the column; the
    /// value typed is in [`TableState::saved`]. The app keeps it, or turns it down and says why.
    Edited {
        row: usize,
        column: usize,
    },
    /// A row's actions were asked for (its menu button pressed): its index among the rows. The
    /// app opens its menu at [`Table::actions_area`].
    Actions(usize),
    /// A row's detail opened (true) or closed: its index among the rows.
    Detail(usize, bool),
    /// The pager asked for this page (counted from 1): the app loads it and gives its rows.
    Page(usize),
    /// Load more was pressed: the app gives the rows after these.
    LoadMore,
}

/// One line of the table's body: a group's head (its name and count), or a row by its place.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BodyLine {
    Head(String, usize),
    Row(usize),
    /// A line of an open row's detail: the row's place and the line.
    Detail(usize, usize),
}

/// Whether a row has children: the row after it is deeper.
fn has_children(rows: &[Row], row: usize) -> bool {
    rows.get(row + 1).is_some_and(|n| n.depth > rows[row].depth)
}

/// The row's parent: the nearest row before it at a lesser depth.
fn parent_of(rows: &[Row], row: usize) -> Option<usize> {
    let d = rows.get(row)?.depth;
    (d > 0).then(|| (0..row).rev().find(|&j| rows[j].depth < d)).flatten()
}

/// Whether a row shows: each of its ancestors open.
fn shown(rows: &[Row], row: usize, expanded: &BTreeSet<usize>) -> bool {
    let mut at = row;
    while let Some(p) = parent_of(rows, at) {
        if !expanded.contains(&p) {
            return false;
        }
        at = p;
    }
    true
}

/// The words a row's cell in `column` gives its group.
fn group_key(rows: &[Row], row: usize, column: usize) -> String {
    rows[row]
        .cells
        .get(column)
        .map_or(String::new(), |c| c.words().to_string())
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
    /// The rows whose detail is open, by their index among the rows.
    pub expanded: BTreeSet<usize>,
    /// The active cell's column, in a table that edits.
    pub cell: usize,
    /// The cell being edited, while it is: its text and cursor.
    pub edit: Option<crate::text_field::FieldState>,
    /// Columns a resize left at a width of their own, in columns, by the column.
    pub sizes: BTreeMap<usize, u16>,
    /// The column whose edge a press took, while it is dragged.
    pub resizing: Option<usize>,
    /// The value the last [`TableEvent::Edited`] saved.
    pub saved: Option<String>,
    /// Where the terminal's cursor goes while a cell is edited.
    edit_cursor: Option<ratatui::layout::Position>,
    /// Rows grouped by this column's value (`groupBy`), each group under its head, the groups in
    /// the order they first come, sorted within.
    pub group: Option<usize>,
    /// How many rows the last frame showed: a view's worth for PageUp and PageDown.
    view: usize,
    /// Whether the last frame stacked the rows as cards.
    stacked: bool,
    /// The last frame's room for the columns.
    room: u16,
}

impl TableState {
    pub fn sorted_by(mut self, column: usize, dir: SortDir) -> Self {
        self.sort = Some((column, dir));
        self
    }

    /// Rows grouped by `column`'s value, each group under a head naming it with its count.
    pub fn grouped_by(mut self, column: usize) -> Self {
        self.group = Some(column);
        self
    }

    /// The body's lines in drawn order: a group's head before its rows while grouped, else the
    /// rows alone, each by its place.
    fn lines(&self, rows: &[Row]) -> Vec<BodyLine> {
        let order = self.order(rows);
        let detail = |place: usize| {
            let row = order[place];
            let n = if self.expanded.contains(&row) {
                rows[row].detail.len()
            } else {
                0
            };
            (0..n).map(move |i| BodyLine::Detail(place, i))
        };
        let seen = |p: &usize| shown(rows, order[*p], &self.expanded);
        let Some(g) = self.group else {
            return (0..order.len())
                .filter(seen)
                .flat_map(|p| std::iter::once(BodyLine::Row(p)).chain(detail(p)))
                .collect();
        };
        let mut out = Vec::new();
        let mut last: Option<String> = None;
        for (place, &row) in order.iter().enumerate() {
            if !seen(&place) {
                out.extend(detail(place));
                continue;
            }
            let key = group_key(rows, row, g);
            if last.as_ref() != Some(&key) {
                let n = order.iter().filter(|&&r| group_key(rows, r, g) == key).count();
                out.push(BodyLine::Head(key.clone(), n));
                last = Some(key);
            }
            out.push(BodyLine::Row(place));
            out.extend(detail(place));
        }
        out
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
        // Grouped: the groups in the order they first come among the rows, each sorted within.
        if let Some(g) = self.group {
            let mut firsts: Vec<String> = Vec::new();
            for r in 0..rows.len() {
                let k = group_key(rows, r, g);
                if !firsts.contains(&k) {
                    firsts.push(k);
                }
            }
            order.sort_by_key(|&r| firsts.iter().position(|k| *k == group_key(rows, r, g)).unwrap_or(0));
        }
        // A tree: the roots in that order, each followed by its descendants as given.
        if rows.iter().any(|r| r.depth > 0) {
            let mut tree = Vec::with_capacity(order.len());
            for &r in order.iter().filter(|&&r| rows[r].depth == 0) {
                tree.push(r);
                tree.extend((r + 1..rows.len()).take_while(|&j| rows[j].depth > 0));
            }
            order = tree;
        }
        order
    }

    /// A key in a table that edits: while a cell is open it goes to the cell; else Left and Right
    /// move the active cell, and Enter or a typed character opens an editable one. `None` leaves
    /// the key to the rows.
    fn edit_key(&mut self, table: &Table, key: KeyPress) -> Option<TableEvent> {
        let n = table.columns.len();
        if let Some(mut field) = self.edit.take() {
            let row = self.active_row(table.rows)?;
            let column = self.cell;
            let value = field.value().to_string();
            let saved = TableEvent::Edited { row, column };
            return Some(match key.key {
                Key::Escape => TableEvent::Moved,
                Key::Enter => {
                    let event = saved;
                    self.saved = Some(value);
                    // Down a row, as a sheet moves.
                    let vis = self.visible(table.rows);
                    let at = vis.iter().position(|&p| p == self.active).unwrap_or(0);
                    self.active = vis.get(at + 1).copied().unwrap_or(self.active);
                    event
                }
                Key::Tab => {
                    let event = saved;
                    self.saved = Some(value);
                    // On to the next editable column.
                    if let Some(c) = (column + 1..n).find(|&c| table.columns[c].editable) {
                        self.cell = c;
                    }
                    event
                }
                _ => {
                    let input = crate::text_field::TextField::new(table.look, "");
                    field.focus = crate::text_field::FieldFocus::Control;
                    field.handle(&input, key);
                    self.edit = Some(field);
                    TableEvent::Moved
                }
            });
        }
        match key.key {
            Key::Left if self.cell > 0 => {
                self.cell -= 1;
                Some(TableEvent::Moved)
            }
            Key::Right if self.cell + 1 < n => {
                self.cell += 1;
                Some(TableEvent::Moved)
            }
            Key::Left | Key::Right => Some(TableEvent::Ignored),
            Key::Enter | Key::Char(_) if table.columns.get(self.cell).is_some_and(|c| c.editable) => {
                let row = self.active_row(table.rows)?;
                // Enter opens it holding its value; a typed character starts it afresh.
                let start = match key.key {
                    Key::Char(c) => c.to_string(),
                    _ => table.rows[row]
                        .cells
                        .get(self.cell)
                        .map_or(String::new(), |c| c.words().to_string()),
                };
                let mut field = crate::text_field::FieldState::new(start);
                field.focus = crate::text_field::FieldFocus::Control;
                self.edit = Some(field);
                Some(TableEvent::Moved)
            }
            _ => None,
        }
    }

    /// Where the terminal's cursor goes: in the cell being edited.
    pub fn cursor_position(&self) -> Option<ratatui::layout::Position> {
        self.edit.as_ref().and(self.edit_cursor)
    }

    /// The places drawn, in order: every row but those under a closed parent.
    fn visible(&self, rows: &[Row]) -> Vec<usize> {
        self.lines(rows)
            .into_iter()
            .filter_map(|l| if let BodyLine::Row(p) = l { Some(p) } else { None })
            .collect()
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
                // The chosen head's edge: < and > move it 2 columns (16 px), = puts it back.
                Key::Char(k @ ('<' | '>' | '=')) if table.columns[h].resizable => {
                    if k == '=' {
                        self.sizes.remove(&h);
                    } else {
                        let now =
                            self.sizes.get(&h).copied().unwrap_or_else(|| {
                                table.widths_sized(self.room, &self.sizes).get(h).copied().unwrap_or(8)
                            });
                        let min = (text::width(&table.columns[h].header) as u16 + 2).max(4);
                        let to = if k == '<' { now.saturating_sub(2) } else { now + 2 };
                        self.sizes.insert(h, to.clamp(min, cols(320.0).max(min)));
                    }
                    TableEvent::Moved
                }
                _ => TableEvent::Ignored,
            };
        }
        if table.editing
            && let Some(event) = self.edit_key(table, key)
        {
            return event;
        }
        // The keys move over the rows shown: a closed parent's children are passed over.
        let vis = self.visible(table.rows);
        let last = vis.len().saturating_sub(1);
        let at = vis.iter().position(|&p| p == self.active).unwrap_or(0);
        let page = self.view.max(1);
        let moved = |s: &mut TableState, to: usize| {
            s.active = vis.get(to.min(last)).copied().unwrap_or(0);
            TableEvent::Moved
        };
        match key.key {
            Key::Down => moved(self, at + 1),
            Key::Up if at == 0 && !self.stacked && !sortable.is_empty() => {
                // Into the head row, on the sorted head, else the first sortable one.
                self.head = Some(match self.sort {
                    Some((c, _)) if sortable.contains(&c) => c,
                    _ => sortable[0],
                });
                TableEvent::Moved
            }
            Key::Up => moved(self, at.saturating_sub(1)),
            Key::PageDown => moved(self, at + page),
            Key::PageUp => moved(self, at.saturating_sub(page)),
            Key::Home => moved(self, 0),
            Key::End => moved(self, last),
            Key::Right | Key::Left | Key::Enter
                if self
                    .active_row(table.rows)
                    .is_some_and(|r| table.opens(r) || key.key == Key::Left && parent_of(table.rows, r).is_some()) =>
            {
                let r = self.active_row(table.rows).unwrap_or(0);
                let open = match key.key {
                    Key::Right => true,
                    Key::Left => false,
                    _ => !self.expanded.contains(&r),
                };
                if open == self.expanded.contains(&r) || !table.opens(r) {
                    // Left on a child with nothing open goes to its parent.
                    if key.key == Key::Left
                        && let Some(p) = parent_of(table.rows, r)
                    {
                        let order = self.order(table.rows);
                        self.active = order.iter().position(|&x| x == p).unwrap_or(self.active);
                        return TableEvent::Moved;
                    }
                    return TableEvent::Ignored;
                }
                if open {
                    self.expanded.insert(r);
                } else {
                    self.expanded.remove(&r);
                }
                TableEvent::Detail(r, open)
            }
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

/// The table's two variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableVariant {
    #[default]
    Table,
    /// A list: wider side padding, and cards as soon as its columns no longer fit.
    List,
}

/// The bar over the heads: a search at its start and the count at its end; while rows are
/// selected, the selection bar ("{count} selected" and Clear).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableBar {
    pub query: String,
    pub placeholder: String,
    pub focused: bool,
}

/// Where a drawn table put its rows: one reading for drawing and for hitting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Geom {
    inner: Rect,
    pad: u16,
    bar_y: Option<u16>,
    head_y: u16,
    rule_y: u16,
    body: Rect,
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
    variant: TableVariant,
    framed: bool,
    row_actions: bool,
    editing: bool,
    /// The pager: this page and how many there are (counted from 1).
    paging: Option<(usize, usize)>,
    load_more: bool,
    /// How many rows come before these in the whole (a page's start), for the range.
    before: usize,
    bar: Option<TableBar>,
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
            variant: TableVariant::Table,
            framed: true,
            row_actions: false,
            editing: false,
            paging: None,
            load_more: false,
            before: 0,
            bar: None,
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

    pub fn variant(mut self, variant: TableVariant) -> Self {
        self.variant = variant;
        self
    }

    /// A menu button at each row's end (`rowActions`): a press on it is [`TableEvent::Actions`],
    /// where the app opens the row's menu.
    pub fn row_actions(mut self, on: bool) -> Self {
        self.row_actions = on;
        self
    }

    /// Its cells edit (`editing`): Left and Right move the active cell across the columns; Enter
    /// or a typed character opens an editable one ([`Column::editable`]) in place; Enter saves
    /// and moves down, Tab saves and moves on, Escape drops the edit ([`TableEvent::Edited`]).
    pub fn editing(mut self, on: bool) -> Self {
        self.editing = on;
        self
    }

    /// A pager at the foot (`paging`): `page` of `pages`, counted from 1, its arrows asking for the
    /// page before and after ([`TableEvent::Page`]).
    pub fn paging(mut self, page: usize, pages: usize) -> Self {
        self.paging = Some((page.max(1), pages.max(1)));
        self
    }

    /// How many rows come before these in the whole (a page's start): the range counts from it,
    /// "41–80 of 1,200" on the second page of forty.
    pub fn starting_at(mut self, before: usize) -> Self {
        self.before = before;
        self
    }

    /// A Load more button at the foot (`loadMore: button`), asking for the rows after these
    /// ([`TableEvent::LoadMore`]).
    pub fn load_more(mut self, on: bool) -> Self {
        self.load_more = on;
        self
    }

    /// The foot's controls on the frame's bottom line, at its start: the pager's arrows and its
    /// words, or Load more.
    fn foot_controls(&self, area: Rect) -> (Option<Rect>, Option<Rect>, Option<Rect>, String) {
        if !self.framed || area.height < 3 {
            return (None, None, None, String::new());
        }
        let y = area.bottom() - 1;
        let x = area.x + 2;
        if let Some((page, pages)) = self.paging {
            let words = self
                .words
                .page
                .replace("{page}", &count(page))
                .replace("{pages}", &count(pages));
            let w = text::width(&words) as u16;
            let prev = Rect::new(x, y, 3, 1);
            let next = Rect::new(x + 3 + w + 2, y, 3, 1);
            return (Some(prev), Some(next), None, words);
        }
        if self.load_more {
            let w = text::width(&self.words.load_more) as u16 + 2;
            return (None, None, Some(Rect::new(x, y, w, 1)), String::new());
        }
        (None, None, None, String::new())
    }

    /// The columns the row actions' button takes at a row's end, and the gap before it.
    fn actions_w(&self) -> u16 {
        if self.row_actions { 3 + 1 } else { 0 }
    }

    /// In a box of its own (default), or unframed where something else frames it.
    pub fn framed(mut self, framed: bool) -> Self {
        self.framed = framed;
        self
    }

    /// The bar over the heads, with the app's search `query` (empty: the `placeholder`).
    pub fn bar(mut self, query: impl Into<String>, placeholder: impl Into<String>, focused: bool) -> Self {
        self.bar = Some(TableBar {
            query: query.into(),
            placeholder: placeholder.into(),
            focused,
        });
        self
    }

    /// The side padding of its rows: one column, or a list's `panel-pad` (2, 1 at compact).
    fn pad(&self) -> u16 {
        match self.variant {
            TableVariant::Table => 1,
            TableVariant::List => by_density(self.look, 2, 1),
        }
    }

    fn geom(&self, area: Rect) -> Geom {
        let inner = if self.framed {
            Rect::new(
                area.x + 1,
                area.y + 1,
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
            )
        } else {
            area
        };
        let bar_y = self.bar.as_ref().map(|_| inner.y);
        let head_y = inner.y + u16::from(bar_y.is_some());
        let body_y = head_y + 2;
        Geom {
            inner,
            pad: self.pad(),
            bar_y,
            head_y,
            rule_y: head_y + 1,
            body: Rect {
                y: body_y,
                height: inner.bottom().saturating_sub(body_y),
                ..inner
            },
        }
    }

    /// Whether it draws its rows as cards in a box `width` columns wide: a table below 69 columns
    /// (576 px); a list as soon as its columns' heads no longer fit.
    pub fn stacked(&self, width: u16) -> bool {
        match self.variant {
            TableVariant::Table => Table::stacks(width),
            TableVariant::List => {
                let inner = width.saturating_sub(if self.framed { 2 } else { 0 });
                let need: u16 = self
                    .columns
                    .iter()
                    .map(|c| text::width(&c.header) as u16 + if c.sortable { 2 } else { 0 })
                    .sum::<u16>()
                    + 2 * self.columns.len().saturating_sub(1) as u16
                    + 2 * self.pad()
                    + self.select_w();
                need > inner
            }
        }
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
            .replace("{from}", &count(if n == 0 { 0 } else { self.before + 1 }))
            .replace("{to}", &count(self.before + n))
            .replace("{total}", &count(total))
    }

    /// The columns before the first cell: the select box, and the disclosure chevron while a row
    /// has a detail.
    fn select_w(&self) -> u16 {
        (if self.selectable { 4 } else { 0 }) + self.lead_w()
    }

    /// The disclosure chevron's columns, while some row has a detail.
    fn lead_w(&self) -> u16 {
        if (0..self.rows.len()).any(|r| self.opens(r)) {
            2
        } else {
            0
        }
    }

    /// Whether a row opens: it has a detail, or children.
    fn opens(&self, row: usize) -> bool {
        !self.rows[row].detail.is_empty() || has_children(self.rows, row)
    }

    /// Each column's width in columns, across `room`.
    pub fn widths(&self, room: u16) -> Vec<u16> {
        self.widths_sized(room, &BTreeMap::new())
    }

    /// Each column's width across `room`, the columns in `sizes` at the width a resize left them.
    pub fn widths_sized(&self, room: u16, sizes: &BTreeMap<usize, u16>) -> Vec<u16> {
        let look = self.look;
        let gap = 2u16;
        let n = self.columns.len() as u16;
        let room = room.saturating_sub(2 * self.pad() + self.select_w() + self.actions_w() + gap * n.saturating_sub(1));
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
        for (&i, &size) in sizes {
            if let Some(x) = w.get_mut(i) {
                *x = size;
            }
        }
        let used: u16 = w.iter().sum();
        if used <= room {
            // The room left goes to the fill columns by weight, else to the last column.
            let spare = room - used;
            let weights: Vec<u16> = self
                .columns
                .iter()
                .enumerate()
                .map(|(i, c)| match c.width {
                    ColumnWidth::Fill(k) if !sizes.contains_key(&i) => k,
                    _ => 0,
                })
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
                    .filter(|&i| !sizes.contains_key(&i) && w[i] > head_w(&self.columns[i]).max(4))
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
            CellValue::Toned(state, tone) => {
                let (dot, word) = tone.tokens();
                vec![
                    (format!("{} ", glyphs.status_dot), body.fg(t(look.color(dot)))),
                    (
                        text::truncate(state, (w as usize).saturating_sub(2), glyphs.ellipsis),
                        body.fg(t(look.color(word))),
                    ),
                ]
            }
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
    /// The bar's search.
    Search,
    /// The selection bar's Clear.
    Clear,
    /// A row's menu button, by its place.
    Actions(usize),
    /// A cell, by its row's place and its column, in a table with [`Table::editing`].
    Cell(usize, usize),
    /// A resizable column's edge in the head row, by the column.
    Edge(usize),
    /// The pager's previous and next arrows, and Load more, in the foot.
    PagePrev,
    PageNext,
    LoadMore,
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
        if self.stacked(area.width) {
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
        let (prev, next, more, _) = self.foot_controls(area);
        let on = |r: Option<Rect>| r.is_some_and(|r| r.contains(at));
        if on(prev) {
            return Some(TableHit::PagePrev);
        }
        if on(next) {
            return Some(TableHit::PageNext);
        }
        if on(more) {
            return Some(TableHit::LoadMore);
        }
        let g = self.geom(area);
        let inner = g.inner;
        if !inner.contains(at) {
            return None;
        }
        if g.bar_y == Some(at.y) {
            if !state.selected.is_empty() {
                let clear = text::width(&self.words.clear) as u16 + 2;
                return (at.x + clear + 1 >= inner.right()).then_some(TableHit::Clear);
            }
            let search_w = (inner.width / 2).min(32);
            return (at.x < inner.x + g.pad + search_w).then_some(TableHit::Search);
        }
        let in_box = self.selectable && (inner.x + g.pad..inner.x + g.pad + 4).contains(&at.x);
        if at.y == g.head_y {
            if in_box {
                return Some(TableHit::HeadBox);
            }
            let widths = self.widths_sized(inner.width, &state.sizes);
            let mut x = inner.x + g.pad + self.select_w();
            for (i, w) in widths.iter().enumerate() {
                if (x..x + w).contains(&at.x) {
                    return Some(TableHit::Head(i));
                }
                // The gap after a resizable head is its edge.
                if self.columns[i].resizable && (x + w..x + w + 2).contains(&at.x) {
                    return Some(TableHit::Edge(i));
                }
                x += w + 2;
            }
            return None;
        }
        let body_y = g.body.y;
        if at.y < body_y {
            return None;
        }
        let line = state.offset + (at.y - body_y) as usize;
        let Some(BodyLine::Row(place)) = state.lines(self.rows).get(line).cloned() else {
            return None;
        };
        if place >= n {
            return None;
        }
        if self.row_actions && at.x + 3 + g.pad > inner.right() {
            return Some(TableHit::Actions(place));
        }
        // In a table that edits, a press lands on a cell.
        if self.editing && !in_box {
            let widths = self.widths_sized(inner.width, &state.sizes);
            let mut x = inner.x + g.pad + self.select_w();
            for (c, w) in widths.iter().enumerate() {
                if (x..x + w + 2).contains(&at.x) {
                    return Some(TableHit::Cell(place, c));
                }
                x += w + 2;
            }
        }
        Some(if in_box {
            TableHit::SelectBox(place)
        } else {
            TableHit::Row(place)
        })
    }

    /// A drag or release after a press took a column's edge ([`TableHit::Edge`]): the column
    /// follows the pointer, between its head's width and 320 px, and lets go on release.
    pub fn drag(&self, area: Rect, state: &mut TableState, p: crate::keys::Pointer) -> TableEvent {
        use crate::keys::PointerKind;
        let Some(c) = state.resizing else {
            return TableEvent::Ignored;
        };
        match p.kind {
            PointerKind::Drag => {
                let g = self.geom(area);
                let widths = self.widths_sized(g.inner.width, &state.sizes);
                let x0 = g.inner.x + g.pad + self.select_w() + widths[..c].iter().map(|w| w + 2).sum::<u16>();
                let min = (text::width(&self.columns[c].header) as u16 + 2).max(4);
                let to = p.at.x.saturating_sub(x0).clamp(min, cols(320.0).max(min));
                state.sizes.insert(c, to);
                TableEvent::Moved
            }
            PointerKind::Release => {
                state.resizing = None;
                TableEvent::Moved
            }
            _ => TableEvent::Ignored,
        }
    }

    /// Where a row's menu button stands, for a row at `place` drawn in `area` (its menu opens
    /// there), while it is in view.
    pub fn actions_area(&self, area: Rect, state: &TableState, place: usize) -> Option<Rect> {
        if !self.row_actions || state.stacked {
            return None;
        }
        let g = self.geom(area);
        let line = state.lines(self.rows).iter().position(|l| *l == BodyLine::Row(place))?;
        let y = g.body.y + line.checked_sub(state.offset)? as u16;
        (y < g.body.bottom()).then(|| Rect::new(g.inner.right().saturating_sub(g.pad + 3), y, 3, 1))
    }

    /// A press on what [`Table::hit`] found: a row becomes active (and is picked where the table
    /// does not select), a box turns its row's selection over, the head's box selects every row or
    /// none, a sortable head turns its order.
    pub fn press(&self, state: &mut TableState, hit: TableHit) -> TableEvent {
        state.head = None;
        match hit {
            TableHit::Row(p) | TableHit::Card(p) => {
                state.active = p;
                // A row with a detail opens or closes it.
                if let Some(r) = state.row_at(self.rows, p)
                    && self.opens(r)
                {
                    let open = !state.expanded.remove(&r);
                    if open {
                        state.expanded.insert(r);
                    }
                    return TableEvent::Detail(r, open);
                }
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
            TableHit::Actions(p) => {
                state.active = p;
                match state.row_at(self.rows, p) {
                    Some(r) => TableEvent::Actions(r),
                    None => TableEvent::Ignored,
                }
            }
            // A press on the active cell opens it, where it edits; elsewhere it becomes active.
            TableHit::Cell(p, c) => {
                let again = state.active == p && state.cell == c && state.edit.is_none();
                state.active = p;
                state.cell = c;
                state.edit = None;
                if again
                    && self.columns.get(c).is_some_and(|col| col.editable)
                    && let Some(r) = state.row_at(self.rows, p)
                {
                    let mut field = crate::text_field::FieldState::new(
                        self.rows[r]
                            .cells
                            .get(c)
                            .map_or(String::new(), |cell| cell.words().to_string()),
                    );
                    field.focus = crate::text_field::FieldFocus::Control;
                    state.edit = Some(field);
                }
                TableEvent::Moved
            }
            TableHit::Edge(c) => {
                state.resizing = Some(c);
                TableEvent::Moved
            }
            TableHit::PagePrev => match self.paging {
                Some((page, _)) if page > 1 => TableEvent::Page(page - 1),
                _ => TableEvent::Ignored,
            },
            TableHit::PageNext => match self.paging {
                Some((page, pages)) if page < pages => TableEvent::Page(page + 1),
                _ => TableEvent::Ignored,
            },
            TableHit::LoadMore => TableEvent::LoadMore,
            TableHit::Head(_) | TableHit::Search => TableEvent::Ignored,
            TableHit::Clear => {
                state.selected.clear();
                TableEvent::Selected
            }
        }
    }

    /// The wheel: the active row moves `by` rows, the view with it.
    pub fn scroll(&self, state: &mut TableState, by: isize) -> TableEvent {
        if self.rows.is_empty() {
            return TableEvent::Ignored;
        }
        let vis = state.visible(self.rows);
        let at = vis.iter().position(|&p| p == state.active).unwrap_or(0) as isize;
        let to = (at + by).clamp(0, vis.len() as isize - 1) as usize;
        state.active = vis.get(to).copied().unwrap_or(0);
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
        state.stacked = self.stacked(area.width);
        state.room = self.geom(area).inner.width;
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
    let geo = table.geom(area);
    let inner = geo.inner;
    if table.framed {
        frame(table, area, buf);
    } else {
        let ground = look
            .paint("table.base.rest.frame", "ground")
            .unwrap_or_else(|| look.color("card"));
        buf.set_style(area, Style::new().bg(t(ground)));
    }
    let widths = table.widths_sized(inner.width, &state.sizes);
    let card = look
        .paint("table.base.rest.frame", "ground")
        .unwrap_or_else(|| look.color("card"));

    // The head row, and the rule under it as a row of tees.
    let head_ground = look.paint("table.base.rest.head", "ground").unwrap_or(card);
    if let Some(y) = geo.bar_y {
        draw_bar(table, Rect::new(inner.x, y, inner.width, 1), geo.pad, buf, state);
    }
    let head_row = Rect::new(inner.x, geo.head_y, inner.width, 1);
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
        buf.set_string(
            inner.x + geo.pad,
            geo.head_y,
            g,
            Style::new().fg(t(ink)).bg(t(head_ground)),
        );
    }
    let mut x = inner.x + geo.pad + table.select_w();
    for (i, c) in table.columns.iter().enumerate() {
        let w = widths[i];
        let sorted = state.sort.filter(|(s, _)| *s == i).map(|(_, d)| d);
        let key = if sorted.is_some() { "sorted" } else { "rest" };
        let ink = look
            .paint(&format!("table.base.{key}.head"), "ink")
            .unwrap_or_else(|| look.color("table-head-ink"));
        let style = TextRole::ColumnHead.style(look).fg(t(ink)).bg(t(head_ground));
        let name = TextRole::ColumnHead.transform(look, &c.header);
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
        buf.set_stringn(nx, geo.head_y, &name, w as usize, style);
        if let Some(g) = glyph {
            buf.set_string(nx + name_w + 1, geo.head_y, g, style);
        }
        // The head the keys chose: a ring beside its name, as a focused button.
        if table.focused && state.head == Some(i) {
            let ring = look.color("ring");
            let end = nx + name_w + if glyph.is_some() { 2 } else { 0 };
            if nx > inner.x {
                buf[(nx - 1, geo.head_y)].set_symbol(glyphs.ring_start).set_fg(t(ring));
            }
            if end < inner.right() {
                buf[(end, geo.head_y)].set_symbol(glyphs.ring_end).set_fg(t(ring));
            }
        }
        // A resizable head's edge shows its line while it is dragged or its head chosen by keys.
        let chosen = table.focused && state.head == Some(i);
        if c.resizable && (state.resizing == Some(i) || chosen) && x + w + 1 < inner.right() {
            let line = look.color("primary");
            buf[(x + w + 1, geo.head_y)]
                .set_symbol(glyphs.scroll_thumb)
                .set_fg(t(line))
                .set_bg(t(head_ground));
        }
        x += w + 2;
    }
    let rule_y = geo.rule_y;
    if rule_y < inner.bottom() {
        let bx = look.box_glyphs("radius-surface");
        let line = look.paint("table.base.rest.head", "rule").unwrap_or(card);
        let page = look.color("background");
        let st = Style::new().fg(t(line)).bg(t(card));
        if table.framed {
            buf[(area.x, rule_y)].set_symbol(bx.tee_start).set_style(st.bg(t(page)));
            buf[(area.right() - 1, rule_y)]
                .set_symbol(bx.tee_end)
                .set_style(st.bg(t(page)));
        }
        for x in inner.x..inner.right() {
            buf[(x, rule_y)].set_symbol(bx.across).set_style(st);
        }
    }
    let body = geo.body;
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
            let mut x = body.x + geo.pad + table.select_w();
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
            body.x + geo.pad,
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
    let lines = state.lines(table.rows);
    let view = body.height as usize;
    // The active row's line kept in view, and the head over the first group in view with it.
    let active_line = lines
        .iter()
        .position(|l| *l == BodyLine::Row(state.active))
        .unwrap_or(0);
    let lead = usize::from(active_line > 0 && matches!(lines.get(active_line - 1), Some(BodyLine::Head(..))));
    if active_line.saturating_sub(lead) < state.offset {
        state.offset = active_line.saturating_sub(lead);
    } else {
        // The active row and its open detail in view, the row first where both do not fit.
        let shown = lines[active_line + 1..]
            .iter()
            .take_while(|l| matches!(l, BodyLine::Detail(p, _) if *p == state.active))
            .count();
        let end = active_line + shown;
        if end >= state.offset + view {
            state.offset = (end + 1 - view).min(active_line);
        }
    }
    state.offset = state.offset.min(lines.len().saturating_sub(view));
    let mut edit_cursor = None;
    for (li, line) in lines.iter().enumerate().skip(state.offset).take(view) {
        let y = body.y + (li - state.offset) as u16;
        let k = match line {
            BodyLine::Head(name, n) => {
                // A group's head, set as the column heads are, in the group's own case, a step
                // above its rows; its count quiet after it.
                let hg = look.paint("table.base.rest.head", "ground").unwrap_or(card);
                let hi = look
                    .paint("table.base.rest.head", "ink")
                    .unwrap_or_else(|| look.color("foreground"));
                buf.set_style(Rect::new(body.x, y, body.width, 1), Style::new().bg(t(hg)));
                let room = body.width.saturating_sub(2 * geo.pad) as usize;
                let name = text::truncate(name, room.saturating_sub(6), glyphs.ellipsis);
                let style = TextRole::ColumnHead.style(look).fg(t(hi)).bg(t(hg));
                buf.set_string(body.x + geo.pad, y, &name, style);
                let quiet = TextRole::Help.style(look).fg(look.token("muted-foreground")).bg(t(hg));
                buf.set_string(body.x + geo.pad + text::width(&name) as u16 + 1, y, count(*n), quiet);
                continue;
            }
            BodyLine::Detail(p, i) => {
                // An open row's detail: labeled lines on the panel's ground, from the first cell.
                let (label, value) = &table.rows[order[*p]].detail[*i];
                let dg = look.color("panel-ground");
                buf.set_style(Rect::new(body.x, y, body.width, 1), Style::new().bg(t(dg)));
                let x = body.x + geo.pad + table.select_w();
                let lw = table.rows[order[*p]]
                    .detail
                    .iter()
                    .map(|(l, _)| text::width(l))
                    .max()
                    .unwrap_or(0) as u16;
                let quiet = TextRole::Help.style(look).fg(look.token("muted-foreground")).bg(t(dg));
                buf.set_stringn(x, y, label, body.right().saturating_sub(x) as usize, quiet);
                let vx = x + lw + 2;
                if vx < body.right() {
                    let ink = TextRole::Body.style(look).fg(look.token("foreground")).bg(t(dg));
                    let room = body.right().saturating_sub(vx + geo.pad) as usize;
                    buf.set_string(vx, y, text::truncate(value, room, glyphs.ellipsis), ink);
                }
                continue;
            }
            BodyLine::Row(p) => *p,
        };
        let row = order[k];
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
        let mut x = body.x + geo.pad;
        if table.selectable {
            let g = if state.selected.contains(&row) {
                glyphs.checkbox_on
            } else {
                glyphs.checkbox_off
            };
            buf.set_string(x, y, g, Style::new().fg(t(ink)).bg(t(ground)));
            x += 4;
        }
        // A row with a detail leads with its chevron: closed, or open while it shows.
        let indent = 2 * table.rows[row].depth as u16;
        if table.lead_w() > 0 {
            if table.opens(row) {
                let g = if state.expanded.contains(&row) {
                    glyphs.chevron_down
                } else {
                    glyphs.chevron
                };
                let quiet = look.color("muted-foreground");
                buf.set_string(x + indent, y, g, Style::new().fg(t(quiet)).bg(t(ground)));
            }
            x += table.lead_w();
        }
        for (i, c) in table.columns.iter().enumerate() {
            // A child's first cell is set in under its parent's.
            let inset = if i == 0 { indent.min(widths[i]) } else { 0 };
            let (sx, sw) = (x + inset, widths[i] - inset);
            let cell = table.rows[row].cells.get(i).cloned().unwrap_or_else(Cell::missing);
            // In a table that edits, the active cell wears the focus ring's edges in the gaps
            // beside it; open, it is a field: its ground, the text typed, the cursor.
            let here = table.editing && table.focused && k == state.active && i == state.cell;
            if here {
                let ring = Style::new().fg(look.token("ring")).bg(t(ground));
                if sx > 0 {
                    buf.set_string(sx - 1, y, glyphs.edge_start, ring);
                }
                buf.set_string(sx + sw, y, glyphs.edge_end, ring);
            }
            if let Some(field) = state.edit.as_ref().filter(|_| here) {
                let fg = look.color("field");
                buf.set_style(Rect::new(sx, y, sw, 1), Style::new().bg(t(fg)));
                let value = field.value();
                let at = text::width(&value[..field.cursor()]) as u16;
                // The window follows the cursor.
                let skip = (at + 1).saturating_sub(sw) as usize;
                let shown: String = value.chars().skip(skip).collect();
                buf.set_stringn(
                    sx,
                    y,
                    &shown,
                    sw as usize,
                    TextRole::Body.style(look).fg(look.token("foreground")).bg(t(fg)),
                );
                edit_cursor = Some(ratatui::layout::Position::new(sx + at - skip as u16, y));
                x += widths[i] + 2;
                continue;
            }
            let spans = table.cell_spans(&cell, sw, ground, ink);
            let cw: u16 = spans.iter().map(|(s, _)| text::width(s) as u16).sum();
            let mut cx = if c.align == Align::End {
                sx + sw.saturating_sub(cw)
            } else {
                sx
            };
            for (s, st) in spans {
                buf.set_stringn(cx, y, &s, (sx + sw).saturating_sub(cx) as usize, st);
                cx += text::width(&s) as u16;
            }
            x += widths[i] + 2;
        }
        // The row's menu button at its end: quiet, in the row's ink while the row is active.
        if table.row_actions {
            let bx = body.right().saturating_sub(geo.pad + 2);
            let quiet = look.color("muted-foreground");
            let ink = if k == state.active && table.focused { ink } else { quiet };
            buf.set_string(bx, y, glyphs.more, Style::new().fg(t(ink)).bg(t(ground)));
        }
    }

    state.edit_cursor = edit_cursor;

    // The foot: the range at the end of the frame's bottom line.
    let range = format!(" {} ", table.range());
    let rw = text::width(&range) as u16;
    if table.framed && rw + 4 < area.width {
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
    // The pager or Load more, at the foot's start.
    let (prev, next, more, words) = table.foot_controls(area);
    let quiet = look.color("muted-foreground");
    let page = look.color("background");
    if let (Some(p), Some(n), Some((at, pages))) = (prev, next, table.paging) {
        let ink = |on: bool| {
            if on {
                look.color("foreground")
            } else {
                look.color("faint")
            }
        };
        buf.set_string(
            p.x,
            p.y,
            format!(" {} ", glyphs.back),
            Style::new().fg(t(ink(at > 1))).bg(t(page)),
        );
        buf.set_string(
            p.right(),
            p.y,
            format!(" {words} "),
            TextRole::Help.style(look).fg(t(quiet)).bg(t(page)),
        );
        buf.set_string(
            n.x,
            n.y,
            format!(" {} ", glyphs.chevron),
            Style::new().fg(t(ink(at < pages))).bg(t(page)),
        );
    }
    if let Some(m) = more {
        Button::new(look, table.words.load_more.clone())
            .variant(ButtonVariant::Ghost)
            .size(crate::button::ButtonSize::Sm)
            .render(m, buf);
    }
}

/// The bar over the heads: the search at its start and the count at its end; while rows are
/// selected, "{count} selected" and Clear.
fn draw_bar(table: &Table, row: Rect, pad: u16, buf: &mut Buffer, state: &TableState) {
    let Some(bar) = &table.bar else { return };
    let look = table.look;
    let glyphs = look.glyphs();
    let t = |c: Rgba| look.terminal(c);
    let quiet = look.color("muted-foreground");
    let help = TextRole::Help.style(look);
    if !state.selected.is_empty() {
        let words = table.words.selected.replace("{count}", &count(state.selected.len()));
        buf.set_string(
            row.x + pad,
            row.y,
            words,
            TextRole::Label.style(look).fg(look.token("foreground")),
        );
        let clear = Button::new(look, table.words.clear.clone()).variant(ButtonVariant::Ghost);
        let w = clear.width();
        clear.render(Rect::new(row.right().saturating_sub(w + 1), row.y, w, 1), buf);
        return;
    }
    // The search, on the field's ground with its edge, focused or not.
    let w = (row.width / 2).min(32);
    let r = Rect::new(row.x + pad, row.y, w, 1);
    let key = if bar.focused {
        "text-field.base.focus.control"
    } else {
        "text-field.base.rest.control"
    };
    let ground = look.paint(key, "ground").unwrap_or_else(|| look.color("field"));
    let edge = look.paint(key, "edge").unwrap_or(ground);
    buf.set_style(r, Style::new().bg(t(ground)));
    buf[(r.x, r.y)].set_symbol(glyphs.edge_start).set_fg(t(edge));
    buf[(r.right() - 1, r.y)].set_symbol(glyphs.edge_end).set_fg(t(edge));
    buf.set_string(r.x + 1, r.y, glyphs.search, Style::new().fg(t(quiet)));
    let (words, ink) = if bar.query.is_empty() {
        (bar.placeholder.as_str(), quiet)
    } else {
        (bar.query.as_str(), look.color("foreground"))
    };
    let shown = text::truncate(words, w.saturating_sub(4) as usize, glyphs.ellipsis);
    buf.set_string(r.x + 3, r.y, &shown, TextRole::Body.style(look).fg(t(ink)));
    // The count at the end: how many rows there are, the range when unframed.
    let n = table.total.unwrap_or(table.rows.len());
    let said = table.words.rows.say(n).replace(&n.to_string(), &count(n));
    let sw = text::width(&said) as u16;
    buf.set_string(row.right().saturating_sub(sw + pad), row.y, said, help.fg(t(quiet)));
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
