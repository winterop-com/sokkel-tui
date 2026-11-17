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
