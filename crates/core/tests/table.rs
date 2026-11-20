use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use sokkel_tui::table::count;
use sokkel_tui::{
    Align, Cell, Column, ColumnWidth, Key, Look, Outcome, PageState, Row, RowTone, Scheme, SortDir, Table, TableEvent,
    TableState, snapshot,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("Job").width(ColumnWidth::Fill(1)).sortable(),
        Column::new("By").sortable(),
        Column::new("Took").align(Align::End).sortable(),
    ]
}

fn rows() -> Vec<Row> {
    vec![
        Row::new([Cell::text("nightly-export"), Cell::text("ada"), Cell::missing()]),
        Row::new([
            Cell::text("sync-inventory"),
            Cell::text("grace"),
            Cell::text("2m 41s").sort_by(161.0),
        ])
        .tone(RowTone::Critical),
        Row::new([
            Cell::text("rebuild-index"),
            Cell::maybe(None::<&str>),
            Cell::text("48s").sort_by(48.0),
        ]),
    ]
}

fn draw(table: &Table, w: u16, h: u16, state: &mut TableState) -> (Buffer, Vec<String>) {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    table.render(buf.area, &mut buf, state);
    let lines = snapshot::text(&buf).lines().map(str::to_string).collect();
    (buf, lines)
}

#[test]
fn a_table_is_its_frame_head_rule_rows_and_range() {
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r);
    let (buf, lines) = draw(&table, 80, 7, &mut TableState::default());
    assert!(lines[0].starts_with('╭'));
    assert!(lines[1].starts_with("│ Job"), "{lines:?}");
    assert!(lines[2].starts_with('├') && lines[2].ends_with('┤'));
    assert!(lines[3].contains("nightly-export"));
    assert!(
        lines[3].trim_end().ends_with("— │"),
        "a missing value is the quiet dash"
    );
    assert!(lines[6].contains("1–3 of 3"));
    // The critical row wears its tone.
    assert_eq!(buf[(5, 4)].bg, look.fill("table.base.toneCritical.row", "ground"));
}

#[test]
fn sorting_keeps_a_missing_value_last_both_ways() {
    let r = rows();
    let up = TableState::default().sorted_by(2, SortDir::Ascending).order(&r);
    let down = TableState::default().sorted_by(2, SortDir::Descending).order(&r);
    assert_eq!(up, vec![2, 1, 0]);
    assert_eq!(down, vec![1, 2, 0]);
    let by_name = TableState::default().sorted_by(0, SortDir::Ascending).order(&r);
    assert_eq!(by_name, vec![0, 2, 1]);
}

#[test]
fn keys_move_the_active_row_and_sort_from_the_head_row() {
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r).focused(true);
    let mut s = TableState::default();
    draw(&table, 80, 8, &mut s);
    assert_eq!(s.handle(&table, Key::Down.into()), TableEvent::Moved);
    assert_eq!(s.active, 1);
    s.handle(&table, Key::End.into());
    assert_eq!(s.active, 2);
    s.handle(&table, Key::Down.into());
    assert_eq!(s.active, 2, "stops at the last");
    s.handle(&table, Key::Home.into());
    // Up from the first row enters the head row on the first sortable head.
    s.handle(&table, Key::Up.into());
    assert_eq!(s.head, Some(0));
    s.handle(&table, Key::Right.into());
    s.handle(&table, Key::Right.into());
    s.handle(&table, Key::Right.into());
    assert_eq!(s.head, Some(2), "stops at the last head");
    assert_eq!(s.handle(&table, Key::Enter.into()), TableEvent::Sorted);
    assert_eq!(s.sort, Some((2, SortDir::Ascending)));
    s.handle(&table, Key::Char(' ').into());
    assert_eq!(s.sort, Some((2, SortDir::Descending)));
    s.handle(&table, Key::Enter.into());
    assert_eq!(s.sort, None, "then as given");
    s.handle(&table, Key::Down.into());
    assert_eq!((s.head, s.active), (None, 0));
    // Enter picks the active row.
    assert_eq!(s.handle(&table, Key::Enter.into()), TableEvent::Opened(0));
}

#[test]
fn space_selects_where_the_table_selects() {
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r).selectable(true);
    let mut s = TableState::default();
    let (_, lines) = draw(&table, 80, 8, &mut s);
    assert!(lines[1].starts_with("│ ▢   Job"), "{lines:?}");
    assert_eq!(s.handle(&table, Key::Char(' ').into()), TableEvent::Selected);
    assert!(s.selected.contains(&0));
    let (_, lines) = draw(&table, 80, 8, &mut s);
    assert!(lines[1].starts_with("│ ⊟"), "some rows selected");
    assert!(lines[3].starts_with("│ ▣"));
}

#[test]
fn only_the_rows_in_view_are_drawn_and_the_active_one_stays_in_it() {
    let look = look();
    let c = columns();
    let r: Vec<Row> = (0..500)
        .map(|i| Row::new([Cell::text(format!("run {i}")), Cell::text("ada"), Cell::missing()]))
        .collect();
    let table = Table::new(&look, &c, &r).total(1200);
    let mut s = TableState::default();
    s.active = 300;
    let (_, lines) = draw(&table, 80, 8, &mut s);
    assert!(lines[6].contains("run 300"), "{lines:?}");
    assert!(lines[7].contains("1–500 of 1,200"));
    assert_eq!(s.offset, 297, "four rows in view, the active one the last");
    s.handle(&table, Key::PageUp.into());
    assert_eq!(s.active, 296, "a view's worth");
}

#[test]
fn its_states_stand_under_the_heads() {
    let look = look();
    let c = columns();
    let none: Vec<Row> = Vec::new();
    let (_, lines) = draw(&Table::new(&look, &c, &none), 60, 6, &mut TableState::default());
    assert!(lines[3].contains("Nothing here."));
    let failed = PageState::failed(&look, Some(Outcome::Status(404)));
    let (_, lines) = draw(
        &Table::new(&look, &c, &none).failed(failed),
        80,
        8,
        &mut TableState::default(),
    );
    assert!(lines[1].contains("Job"));
    assert!(lines.iter().any(|l| l.contains("! Nothing is there")), "{lines:?}");
}

#[test]
fn below_its_stack_width_rows_are_cards() {
    let look = look();
    let (c, r) = (columns(), rows());
    assert!(Table::stacks(68));
    assert!(!Table::stacks(69));
    let table = Table::new(&look, &c, &r);
    let (_, lines) = draw(&table, 40, 14, &mut TableState::default());
    assert!(lines[2].contains("nightly-export"), "{lines:?}");
    assert!(lines[3].contains("By") && lines[3].contains("ada"));
    assert!(lines[4].contains("Took") && lines[4].contains('—'));
}

#[test]
fn counts_are_written_as_sokkel_ui_writes_them() {
    assert_eq!(count(0), "0");
    assert_eq!(count(999), "999");
    assert_eq!(count(1200), "1,200");
    assert_eq!(count(50000), "50,000");
    assert_eq!(count(1234567), "1,234,567");
}

#[test]
fn the_terminal_theme_sets_its_heads_upper_case() {
    let look = Look::new("terminal", Scheme::Dark).unwrap();
    let (c, r) = (columns(), rows());
    let (_, lines) = draw(&Table::new(&look, &c, &r), 80, 7, &mut TableState::default());
    assert!(lines[1].starts_with("│ JOB"), "{lines:?}");
    assert!(lines[0].starts_with('┌'), "square corners: its surface radius is 0");
}

#[test]
fn a_pointer_hits_rows_boxes_and_heads() {
    use ratatui::layout::Position;
    use sokkel_tui::TableHit;
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r).selectable(true);
    let area = Rect::new(0, 0, 80, 8);
    let mut s = TableState::default();
    draw(&table, 80, 8, &mut s);
    // The frame's border, the head at row 1, the rule at 2, rows from 3.
    assert_eq!(table.hit(area, Position::new(20, 3), &s), Some(TableHit::Row(0)));
    assert_eq!(table.hit(area, Position::new(2, 4), &s), Some(TableHit::SelectBox(1)));
    assert_eq!(table.hit(area, Position::new(2, 1), &s), Some(TableHit::HeadBox));
    assert_eq!(table.hit(area, Position::new(6, 1), &s), Some(TableHit::Head(0)));
    assert_eq!(table.hit(area, Position::new(20, 2), &s), None, "the rule");
    assert_eq!(table.hit(area, Position::new(20, 6), &s), None, "past the last row");

    assert_eq!(table.press(&mut s, TableHit::Row(2)), TableEvent::Moved);
    assert_eq!(s.active, 2);
    table.press(&mut s, TableHit::SelectBox(1));
    assert!(s.selected.contains(&1));
    table.press(&mut s, TableHit::HeadBox);
    assert_eq!(s.selected.len(), 3, "every row");
    table.press(&mut s, TableHit::HeadBox);
    assert!(s.selected.is_empty(), "then none");
    table.press(&mut s, TableHit::Head(0));
    assert_eq!(s.sort, Some((0, SortDir::Ascending)));
    table.scroll(&mut s, 5);
    assert_eq!(s.active, 2, "the wheel stops at the last row");

    // A table that does not select picks the row pressed.
    let plain = Table::new(&look, &c, &r);
    let mut s = TableState::default();
    assert_eq!(plain.press(&mut s, TableHit::Row(1)), TableEvent::Opened(1));
}

#[test]
fn the_bar_holds_the_search_and_the_count_then_the_selection() {
    use ratatui::layout::Position;
    use sokkel_tui::TableHit;
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r).selectable(true).bar("", "Search runs", false);
    let mut s = TableState::default();
    let (_, lines) = draw(&table, 80, 9, &mut s);
    assert!(lines[1].contains("⌕ Search runs"), "{lines:?}");
    assert!(lines[1].trim_end().ends_with("3 rows │"), "{lines:?}");
    assert!(lines[2].contains("Job"), "the head under the bar");
    let area = Rect::new(0, 0, 80, 9);
    assert_eq!(table.hit(area, Position::new(5, 1), &s), Some(TableHit::Search));
    assert_eq!(
        table.hit(area, Position::new(20, 4), &s),
        Some(TableHit::Row(0)),
        "rows a row lower"
    );
    s.selected.insert(1);
    let (_, lines) = draw(&table, 80, 9, &mut s);
    assert!(
        lines[1].contains("1 selected") && lines[1].contains("Clear"),
        "{lines:?}"
    );
    assert_eq!(table.hit(area, Position::new(74, 1), &s), Some(TableHit::Clear));
    table.press(&mut s, TableHit::Clear);
    assert!(s.selected.is_empty());
}

#[test]
fn a_list_pads_wider_and_stacks_when_its_heads_no_longer_fit() {
    use sokkel_tui::TableVariant;
    let look = look();
    let (c, r) = (columns(), rows());
    let list = Table::new(&look, &c, &r).variant(TableVariant::List);
    let (_, lines) = draw(&list, 60, 7, &mut TableState::default());
    assert!(
        lines[1].starts_with("│  Job"),
        "two columns of padding at regular: {lines:?}"
    );
    assert!(!list.stacked(60) && list.stacked(14));
    let compact = look.clone().with_density(sokkel_tui::Density::Compact);
    let list = Table::new(&compact, &c, &r).variant(TableVariant::List);
    let (_, lines) = draw(&list, 60, 7, &mut TableState::default());
    assert!(lines[1].starts_with("│ Job"), "one at compact");
}

#[test]
fn unframed_it_draws_no_box_and_its_rule_runs_across() {
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r).framed(false);
    // 80 columns: a table stacks below 69, whatever its frame.
    let (_, lines) = draw(&table, 80, 6, &mut TableState::default());
    assert!(lines[0].starts_with(" Job"), "{lines:?}");
    assert!(lines[1].starts_with('─') && lines[1].ends_with('─'));
    assert!(lines[2].contains("nightly-export"));
}

#[test]
fn a_toned_state_says_its_own_word_in_its_tones_colors() {
    use sokkel_tui::StatusTone;
    let look = look();
    let c = vec![Column::new("Host"), Column::new("State")];
    let r = vec![Row::new([
        Cell::text("juniper.example.org"),
        Cell::toned("fail", StatusTone::Critical),
    ])];
    let table = Table::new(&look, &c, &r);
    let (buf, lines) = draw(&table, 80, 6, &mut TableState::default());
    let y = lines.iter().position(|l| l.contains("fail")).unwrap() as u16;
    let line = &lines[y as usize];
    let x = line[..line.find("fail").unwrap()].chars().count() as u16;
    assert_eq!(buf[(x, y)].fg, look.token("critical-ink"));
    assert!(
        StatusTone::Critical.rank() > StatusTone::Warning.rank()
            && StatusTone::Warning.rank() > StatusTone::Good.rank()
    );
}

#[test]
fn grouped_rows_stand_under_their_groups_heads() {
    use ratatui::layout::Position;
    use sokkel_tui::TableHit;
    let look = look();
    let c = vec![Column::new("Job"), Column::new("Project")];
    let row = |job: &str, project: &str| Row::new([Cell::text(job), Cell::text(project)]);
    let r = vec![
        row("nightly-export", "harbor"),
        row("sync-inventory", "juniper"),
        row("send-digest", "harbor"),
    ];
    let table = Table::new(&look, &c, &r);
    let mut state = TableState::default().grouped_by(1);
    let (_, lines) = draw(&table, 80, 10, &mut state);
    let at = |w: &str| lines.iter().position(|l| l.contains(w)).unwrap();
    assert!(
        at("harbor 2") < at("nightly-export") && at("send-digest") < at("juniper 1"),
        "{lines:?}"
    );
    assert!(at("juniper 1") < at("sync-inventory"));
    // A press on a group's head is no row; one on a row is its place in the drawn order.
    let area = Rect::new(0, 0, 80, 10);
    assert_eq!(table.hit(area, Position::new(5, at("harbor 2") as u16), &state), None);
    assert_eq!(
        table.hit(area, Position::new(5, at("send-digest") as u16), &state),
        Some(TableHit::Row(1))
    );
    assert_eq!(state.row_at(&r, 1), Some(2), "send-digest, second in harbor");
}

#[test]
fn a_rows_menu_button_asks_for_its_actions() {
    use ratatui::layout::Position;
    use sokkel_tui::{TableEvent, TableHit};
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r).row_actions(true).focused(true);
    let mut s = TableState::default();
    let (_, lines) = draw(&table, 80, 9, &mut s);
    let area = Rect::new(0, 0, 80, 9);
    let at = table.actions_area(area, &s, 1).expect("in view");
    assert!(lines[at.y as usize].contains('…'), "{lines:?}");
    let hit = table.hit(area, Position::new(at.x + 1, at.y), &s);
    assert_eq!(hit, Some(TableHit::Actions(1)));
    assert_eq!(
        table.press(&mut s, hit.unwrap()),
        TableEvent::Actions(s.row_at(&r, 1).unwrap())
    );
    assert_eq!(s.active, 1);
}

#[test]
fn the_pager_asks_for_the_pages_beside_this_one() {
    use ratatui::layout::Position;
    use sokkel_tui::{TableEvent, TableHit};
    let look = look();
    let (c, r) = (columns(), rows());
    let table = Table::new(&look, &c, &r)
        .paging(2, 3)
        .starting_at(r.len())
        .total(3 * r.len());
    let mut s = TableState::default();
    let (_, lines) = draw(&table, 80, 10, &mut s);
    let foot = lines.last().unwrap();
    assert!(foot.contains("Page 2 of 3"), "{foot}");
    let n = r.len();
    assert!(foot.contains(&format!("{}–{} of {}", n + 1, 2 * n, 3 * n)), "{foot}");
    let area = Rect::new(0, 0, 80, 10);
    let x = |g: char| foot.chars().position(|c| c == g).unwrap() as u16;
    let prev = table.hit(area, Position::new(x('‹'), 9), &s);
    assert_eq!(prev, Some(TableHit::PagePrev));
    assert_eq!(table.press(&mut s, prev.unwrap()), TableEvent::Page(1));
    let next = table.hit(area, Position::new(x('›'), 9), &s).unwrap();
    assert_eq!(table.press(&mut s, next), TableEvent::Page(3));
    // On the last page the next arrow asks for nothing.
    let last = Table::new(&look, &c, &r).paging(3, 3);
    assert_eq!(last.press(&mut s, TableHit::PageNext), TableEvent::Ignored);
}

#[test]
fn a_rows_detail_opens_under_it() {
    use ratatui::layout::Position;
    use sokkel_tui::{Key, KeyPress, TableEvent, TableHit};
    let look = look();
    let c = vec![Column::new("Job"), Column::new("Project")];
    let r = vec![
        Row::new([Cell::text("nightly-export"), Cell::text("harbor")]).detail([("Host", "harbor.example.org")]),
        Row::new([Cell::text("send-digest"), Cell::text("juniper")]),
    ];
    let table = Table::new(&look, &c, &r).focused(true);
    let mut s = TableState::default();
    assert_eq!(s.handle(&table, KeyPress::new(Key::Right)), TableEvent::Detail(0, true));
    let (_, lines) = draw(&table, 80, 9, &mut s);
    let at = |w: &str| lines.iter().position(|l| l.contains(w)).unwrap();
    assert_eq!(at("harbor.example.org"), at("nightly-export") + 1, "{lines:?}");
    assert_eq!(at("send-digest"), at("harbor.example.org") + 1);
    // A press on the detail is no row; Left closes it.
    let area = Rect::new(0, 0, 80, 9);
    assert_eq!(
        table.hit(area, Position::new(4, at("harbor.example.org") as u16), &s),
        None
    );
    assert_eq!(s.handle(&table, KeyPress::new(Key::Left)), TableEvent::Detail(0, false));
    // A press on the row opens it again.
    let hit = table
        .hit(area, Position::new(4, at("nightly-export") as u16), &s)
        .unwrap();
    assert_eq!(hit, TableHit::Row(0));
    assert_eq!(table.press(&mut s, hit), TableEvent::Detail(0, true));
}

#[test]
fn a_trees_children_show_under_an_open_parent_and_keys_pass_closed_ones() {
    use sokkel_tui::{Key, KeyPress, TableEvent};
    let look = look();
    let c = vec![Column::new("Job").sortable(), Column::new("State")];
    let row = |job: &str, state: &str| Row::new([Cell::text(job), Cell::status(state)]);
    let r = vec![
        row("sync-inventory", "failed"),
        row("run 98", "failed").child(),
        row("nightly-export", "running"),
        row("run 412", "running").child(),
    ];
    let table = Table::new(&look, &c, &r).focused(true);
    let mut s = TableState::default();
    let (_, lines) = draw(&table, 80, 10, &mut s);
    assert!(
        !lines.iter().any(|l| l.contains("run 98")),
        "closed, its children hide: {lines:?}"
    );
    // Down passes the closed parent's child.
    s.handle(&table, KeyPress::new(Key::Down));
    assert_eq!(s.active_row(&r), Some(2));
    // Right opens it; Down goes into its child; Left goes back to the parent.
    assert_eq!(s.handle(&table, KeyPress::new(Key::Right)), TableEvent::Detail(2, true));
    s.handle(&table, KeyPress::new(Key::Down));
    assert_eq!(s.active_row(&r), Some(3));
    let (_, lines) = draw(&table, 80, 10, &mut s);
    let at = |w: &str| lines.iter().position(|l| l.contains(w)).unwrap();
    assert_eq!(at("run 412"), at("nightly-export") + 1);
    assert!(lines[at("run 412")].contains("    run 412"), "set in under its parent");
    assert_eq!(s.handle(&table, KeyPress::new(Key::Left)), TableEvent::Moved);
    assert_eq!(s.active_row(&r), Some(2));
    // Sorted, the children stay with their parent.
    s.sort = Some((0, sokkel_tui::SortDir::Ascending));
    assert_eq!(s.order(&r), vec![2, 3, 0, 1]);
}

#[test]
fn an_editable_cell_opens_saves_and_moves_on() {
    use sokkel_tui::{Key, KeyPress, TableEvent};
    let look = look();
    let c = vec![
        Column::new("Job"),
        Column::new("Schedule").editable(),
        Column::new("Note").editable(),
    ];
    let row = |job: &str, s: &str| Row::new([Cell::text(job), Cell::text(s), Cell::missing()]);
    let r = vec![row("sync-inventory", "every hour"), row("send-digest", "every morning")];
    let table = Table::new(&look, &c, &r).editing(true).focused(true);
    let mut s = TableState::default();
    let key = |k| KeyPress::new(k);
    // The Job column does not edit: a typed character is left to the rows.
    s.handle(&table, key(Key::Char('x')));
    assert!(s.edit.is_none());
    s.handle(&table, key(Key::Right));
    assert_eq!(s.cell, 1);
    // Enter opens it holding its value; typing goes to it; Enter saves and moves down.
    s.handle(&table, key(Key::Enter));
    assert_eq!(s.edit.as_ref().unwrap().value(), "every hour");
    s.handle(&table, key(Key::Char('!')));
    let (_, lines) = draw(&table, 80, 8, &mut s);
    assert!(lines.iter().any(|l| l.contains("every hour!")), "{lines:?}");
    assert!(s.cursor_position().is_some(), "the cursor in the cell");
    assert_eq!(
        s.handle(&table, key(Key::Enter)),
        TableEvent::Edited { row: 0, column: 1 }
    );
    assert_eq!(s.saved.as_deref(), Some("every hour!"));
    assert_eq!(s.active_row(&r), Some(1), "down a row");
    // A typed character starts afresh; Tab saves and moves on; Escape drops an edit.
    s.handle(&table, key(Key::Char('w')));
    assert_eq!(s.edit.as_ref().unwrap().value(), "w");
    assert_eq!(
        s.handle(&table, key(Key::Tab)),
        TableEvent::Edited { row: 1, column: 1 }
    );
    assert_eq!(s.cell, 2);
    s.handle(&table, key(Key::Char('z')));
    s.handle(&table, key(Key::Escape));
    assert!(s.edit.is_none());
}

#[test]
fn a_resizable_columns_edge_follows_a_drag_and_the_keys() {
    use ratatui::layout::Position;
    use sokkel_tui::{Key, KeyPress, Pointer, PointerKind, TableHit};
    let look = look();
    let c = vec![
        Column::new("Job").width(ColumnWidth::Fill(1)).resizable().sortable(),
        Column::new("State").sortable(),
    ];
    let r = vec![Row::new([Cell::text("nightly-export"), Cell::status("running")])];
    let table = Table::new(&look, &c, &r).focused(true);
    let mut s = TableState::default();
    let area = Rect::new(0, 0, 80, 6);
    let (_, lines) = draw(&table, 80, 6, &mut s);
    let head_y = lines.iter().position(|l| l.contains("Job")).unwrap() as u16;
    let state_x = lines[head_y as usize].find("State").unwrap() as u16;
    // The gap before State is Job's edge.
    let edge = (0..state_x)
        .rev()
        .find(|&x| table.hit(area, Position::new(x, head_y), &s) == Some(TableHit::Edge(0)))
        .expect("an edge before State");
    assert!(state_x - edge <= 3, "right before State");
    let hit = table.hit(area, Position::new(edge, head_y), &s);
    assert_eq!(hit, Some(TableHit::Edge(0)));
    table.press(&mut s, hit.unwrap());
    let at = |kind, x| Pointer {
        kind,
        at: Position::new(x, head_y),
    };
    table.drag(area, &mut s, at(PointerKind::Drag, 30));
    let w = s.sizes[&0];
    let (_, lines) = draw(&table, 80, 6, &mut s);
    assert!(
        lines[head_y as usize].find("State").unwrap() < state_x as usize,
        "narrower: {lines:?}"
    );
    table.drag(area, &mut s, at(PointerKind::Release, 30));
    assert_eq!(s.resizing, None);
    // A drag after the release moves nothing.
    table.drag(area, &mut s, at(PointerKind::Drag, 60));
    assert_eq!(s.sizes[&0], w);
    // With its head chosen: > widens it 2, = puts it back.
    s.head = Some(0);
    s.handle(&table, KeyPress::new(Key::Char('>')));
    assert_eq!(s.sizes[&0], w + 2);
    s.handle(&table, KeyPress::new(Key::Char('=')));
    assert!(!s.sizes.contains_key(&0));
}

#[test]
fn a_head_wider_than_its_column_wraps_between_its_words() {
    use ratatui::layout::Position;
    use sokkel_tui::TableHit;
    let look = look();
    let c = vec![
        Column::new("Job"),
        Column::new("Times started this week"),
        Column::new("Runs failed this week"),
        Column::new("Hosts used this week"),
    ];
    let r = vec![Row::new([
        Cell::text("nightly-export"),
        Cell::text("4"),
        Cell::text("1"),
        Cell::text("2"),
    ])];
    let table = Table::new(&look, &c, &r);
    // With room, each head stands whole on one row.
    let mut s = TableState::default();
    let (_, lines) = draw(&table, 100, 8, &mut s);
    let head = lines.iter().position(|l| l.contains("Job")).unwrap();
    assert!(lines[head].contains("Times started this week"), "{lines:?}");
    // Short of room, the long heads give way, never past their longest word, and wrap onto two
    // lines; the rows move a row lower.
    let w = table.widths(70);
    assert!(w[1] < 23 && w[1] >= "started".len() as u16, "{w:?}");
    let (_, lines) = draw(&table, 72, 8, &mut s);
    let head = lines.iter().position(|l| l.contains("Job")).unwrap();
    assert!(
        lines[head].contains("Times") && lines[head + 1].contains("this"),
        "{lines:?}"
    );
    assert!(
        lines[head + 3].contains("nightly-export"),
        "the rule under both lines, then the rows"
    );
    // A press on either line is the head's.
    let x = lines[head + 1].find("this").unwrap() as u16;
    let area = Rect::new(0, 0, 72, 8);
    assert!(matches!(
        table.hit(area, Position::new(x, head as u16 + 1), &s),
        Some(TableHit::Head(_))
    ));
}

#[test]
fn a_table_stacks_by_its_own_stack_below_and_its_columns_least_width() {
    let look = look();
    let columns = [Column::new("Job"), Column::new("State"), Column::new("Host")];
    let rows: Vec<Row> = Vec::new();
    let table = || Table::new(&look, &columns, &rows);
    // At its defaults (576 px and 48rem) it stacks below 69 columns, as sokkel-ui's does.
    assert!(table().stacked(55));
    assert!(!table().stacked(69));
    // Its columns' own width as its least: it stays a table while they fit.
    let least = table().least_width();
    assert!(least < 55, "{least}");
    assert!(!table().min_width(0).stacked(55));
    assert!(table().min_width(0).stacked(least - 1));
    // Its own stackBelow, and none at all.
    assert!(!table().stack_below(Some(48)).stacked(55));
    assert!(table().stack_below(Some(48)).stacked(47));
    assert!(!table().stack_below(None).stacked(10));
}
