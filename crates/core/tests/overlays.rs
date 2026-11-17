use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Button, Confirm, Dialog, DialogEvent, DialogFocus, DialogWidth, DropdownMenu, Item, Key, KeyPress, Look, MenuEvent,
    MenuState, PopupRow, Scheme, Select, SelectEvent, SelectOption, SelectState, snapshot,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn menu(look: &Look) -> DropdownMenu<'_> {
    DropdownMenu::new(
        look,
        [
            PopupRow::Label("Run".into()),
            Item::new("Run now").shortcut("Ctrl R").into(),
            Item::new("Run with").into(),
            Item::new("Pause").disabled(true).into(),
            PopupRow::Separator,
            Item::new("Notify on failure").check(true).into(),
            Item::new("Delete").destructive().into(),
        ],
    )
    .submenu(2, [Item::new("harbor").into(), Item::new("juniper").into()])
}

fn press(k: Key) -> KeyPress {
    KeyPress::new(k)
}

#[test]
fn a_menu_opens_on_its_first_row_and_skips_what_is_not_a_stop() {
    let look = look();
    let m = menu(&look);
    let mut s = MenuState::default();
    assert_eq!(m.handle(&mut s, press(Key::Down)), MenuEvent::Opened);
    assert_eq!(s.highlight, 1, "the label is no stop");
    m.handle(&mut s, press(Key::Down));
    m.handle(&mut s, press(Key::Down));
    assert_eq!(s.highlight, 5, "past the disabled row and the separator");
    m.handle(&mut s, press(Key::End));
    m.handle(&mut s, press(Key::Down));
    assert_eq!(s.highlight, 6, "it stops at the last");
    m.handle(&mut s, press(Key::Char('r')));
    assert_eq!(s.highlight, 1, "typeahead: the next row it starts");
    let mut up = MenuState::default();
    m.handle(&mut up, press(Key::Up));
    assert_eq!(up.highlight, 6, "Up opens on the last row");
}

#[test]
fn a_menu_acts_closes_and_keeps_a_check_row_open() {
    let look = look();
    let m = menu(&look);
    let mut s = MenuState {
        open: true,
        highlight: 5,
        sub: None,
    };
    assert_eq!(m.handle(&mut s, press(Key::Enter)), MenuEvent::Action(5, None));
    assert!(s.open, "a check row turns over, the menu staying open");
    s.highlight = 1;
    assert_eq!(m.handle(&mut s, press(Key::Char(' '))), MenuEvent::Action(1, None));
    assert!(!s.open);
    let mut s = MenuState {
        open: true,
        highlight: 1,
        sub: None,
    };
    assert_eq!(m.handle(&mut s, press(Key::Escape)), MenuEvent::Closed);
    assert!(!s.open);
}

#[test]
fn a_submenu_opens_beside_its_row_and_closes_back_to_it() {
    let look = look();
    let m = menu(&look);
    let mut s = MenuState {
        open: true,
        highlight: 2,
        sub: None,
    };
    m.handle(&mut s, press(Key::Right));
    assert_eq!(s.sub, Some((2, 0)));
    m.handle(&mut s, press(Key::Down));
    assert_eq!(m.handle(&mut s, press(Key::Enter)), MenuEvent::Action(2, Some(1)));
    assert!(!s.open);
    let mut s = MenuState {
        open: true,
        highlight: 2,
        sub: Some((2, 0)),
    };
    m.handle(&mut s, press(Key::Left));
    assert_eq!((s.sub, s.highlight), (None, 2));

    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 14));
    let s = MenuState {
        open: true,
        highlight: 2,
        sub: Some((2, 1)),
    };
    m.render(buf.area, Rect::new(1, 0, 9, 1), &mut buf, &s);
    let text = snapshot::text(&buf);
    assert!(text.contains("Run now") && text.contains("Ctrl R"), "{text}");
    assert!(text.contains("✓ Notify on failure"));
    assert!(text.contains("juniper"), "the submenu beside it");
}

fn options() -> Vec<SelectOption> {
    vec![
        SelectOption::new("Every hour"),
        SelectOption::new("Every night"),
        SelectOption::new("By hand").disabled(true),
        SelectOption::new("Every week"),
    ]
}

#[test]
fn a_select_opens_on_its_choice_and_chooses() {
    let look = look();
    let sel = Select::new(&look, options()).value(Some(1));
    let mut s = SelectState::default();
    assert_eq!(sel.handle(&mut s, press(Key::Enter)), SelectEvent::Opened);
    assert_eq!(s.highlight, 1);
    sel.handle(&mut s, press(Key::Down));
    assert_eq!(s.highlight, 3, "past the disabled option");
    assert_eq!(sel.handle(&mut s, press(Key::Enter)), SelectEvent::Chose(3));
    assert!(!s.open);
    let mut s = SelectState {
        open: true,
        highlight: 0,
    };
    assert_eq!(sel.handle(&mut s, press(Key::Escape)), SelectEvent::Closed);
}

#[test]
fn a_closed_select_chooses_by_typing_and_a_disabled_one_answers_nothing() {
    let look = look();
    let sel = Select::new(&look, options()).value(Some(0));
    let mut s = SelectState::default();
    assert_eq!(sel.handle(&mut s, press(Key::Char('e'))), SelectEvent::Chose(1));
    assert!(!s.open, "the list stays closed");
    let ro = Select::new(&look, options()).read_only(true);
    assert_eq!(ro.handle(&mut s, press(Key::Char('e'))), SelectEvent::Ignored);
    let off = Select::new(&look, options()).disabled(true);
    assert!(!off.focusable());
    assert_eq!(off.handle(&mut s, press(Key::Enter)), SelectEvent::Ignored);
}

#[test]
fn a_select_draws_its_choice_and_chevron_and_its_list_checks_it() {
    let look = look();
    let sel = Select::new(&look, options()).value(Some(1));
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 8));
    let s = SelectState {
        open: true,
        highlight: 1,
    };
    sel.render_trigger(Rect::new(0, 0, 20, 1), &mut buf, &s);
    sel.render_list(buf.area, Rect::new(0, 0, 20, 1), &mut buf, &s);
    let lines: Vec<String> = snapshot::text(&buf).lines().map(String::from).collect();
    assert!(
        lines[0].starts_with("▏Every night") && lines[0].contains('▾'),
        "{lines:?}"
    );
    assert!(lines[3].contains("Every night") && lines[3].contains('✓'), "{lines:?}");
}

fn confirm_dialog(look: &Look) -> Dialog<'_> {
    Dialog::new(look, "Delete nightly-export?")
        .width(DialogWidth::Sm)
        .confirm(Confirm::new("Delete").destructive())
}

#[test]
fn a_dialog_closes_on_escape_and_confirms_on_enter() {
    let look = look();
    let d = confirm_dialog(&look);
    assert_eq!(d.handle(press(Key::Escape)), DialogEvent::Close);
    assert_eq!(
        d.clone().focus(DialogFocus::Confirm).handle(press(Key::Enter)),
        DialogEvent::Confirm
    );
    assert_eq!(
        d.clone().focus(DialogFocus::Cancel).handle(press(Key::Enter)),
        DialogEvent::Close
    );
    let held = d.clone().dismissible(false);
    assert_eq!(held.handle(press(Key::Escape)), DialogEvent::Ignored);
    let shut = Dialog::new(&look, "New job")
        .confirm(Confirm::new("Create").why(Some("Give the job a schedule first")))
        .focus(DialogFocus::Confirm);
    assert_eq!(shut.handle(press(Key::Enter)), DialogEvent::Ignored);
    assert_eq!(shut.tip().as_deref(), Some("Give the job a schedule first"));
}

#[test]
fn a_busy_dialog_holds_until_busy_release_then_lets_go() {
    let look = look();
    let busy = Dialog::new(&look, "Export runs").confirm(Confirm::new("Export").busy(true, Some("Exporting")));
    assert_eq!(
        busy.clone().busy_for(1000).handle(press(Key::Escape)),
        DialogEvent::Ignored
    );
    let late = busy.busy_for(5000);
    assert!(late.released());
    assert_eq!(late.handle(press(Key::Escape)), DialogEvent::StopWaiting);
    assert_eq!(
        late.clone().focus(DialogFocus::Cancel).handle(press(Key::Enter)),
        DialogEvent::StopWaiting
    );
    let mut buf = Buffer::empty(Rect::new(0, 0, 120, 30));
    let r = late.area(buf.area, 2);
    (&late).render(r, &mut buf);
    let text = snapshot::text(&buf);
    assert!(text.contains("Stop waiting") && text.contains("Exporting"), "{text}");
}

#[test]
fn a_dialog_stands_centered_at_its_width_or_fills_the_window_as_a_sheet() {
    let look = look();
    let d = confirm_dialog(&look);
    let window = Rect::new(0, 0, 120, 40);
    let r = d.area(window, 3);
    assert_eq!(r.width, 46, "sm: 384 px");
    assert_eq!(r.x, (120 - 46) / 2);
    assert_eq!(
        d.clone().width(DialogWidth::Full).area(window, 3).width,
        116,
        "the window less four"
    );
    let sheet = d.clone().window(Width::Narrow);
    assert_eq!(sheet.area(Rect::new(0, 0, 80, 24), 3), Rect::new(0, 0, 80, 24));
    // The verbs at the footer's end, the confirm last.
    let l = d.layout(r);
    let (cancel, confirm) = (l.cancel.unwrap(), l.confirm.unwrap());
    assert!(cancel.x < confirm.x && cancel.y == confirm.y);
    assert_eq!(confirm.right(), r.right() - 3);
    assert!(l.body.height >= 3);
}

#[test]
fn a_dialog_with_nothing_to_confirm_closes_from_its_corner() {
    let look = look();
    let d = Dialog::new(&look, "About harbor")
        .actions([Button::new(&look, "Open")])
        .focus(DialogFocus::Close);
    let r = d.area(Rect::new(0, 0, 100, 30), 2);
    let l = d.layout(r);
    assert!(l.close.is_some() && l.cancel.is_none());
    assert_eq!(d.handle(press(Key::Enter)), DialogEvent::Close);
    assert_eq!(
        d.clone().focus(DialogFocus::Action(0)).handle(press(Key::Enter)),
        DialogEvent::Action(0)
    );
}

#[test]
fn a_pointer_picks_menu_rows_and_select_options() {
    use ratatui::layout::Position;
    let look = look();
    let m = menu(&look);
    let window = Rect::new(0, 0, 60, 20);
    let anchor = Rect::new(1, 0, 9, 1);
    let mut s = MenuState {
        open: true,
        highlight: 1,
        sub: None,
    };
    let (area, _) = m.areas(window, anchor, &s);
    // The popup's border, then its rows: a label, items, a disabled one, a separator.
    assert_eq!(
        m.hit(window, anchor, Position::new(area.x + 3, area.y + 1), &s),
        None,
        "a label"
    );
    assert_eq!(
        m.hit(window, anchor, Position::new(area.x + 3, area.y + 2), &s),
        Some((1, None))
    );
    assert_eq!(
        m.hit(window, anchor, Position::new(area.x + 3, area.y + 4), &s),
        None,
        "a disabled row"
    );
    assert_eq!(m.press(&mut s, (1, None)), MenuEvent::Action(1, None));
    assert!(!s.open);
    let mut s = MenuState {
        open: true,
        highlight: 1,
        sub: None,
    };
    m.press(&mut s, (2, None));
    assert_eq!(s.sub, Some((2, 0)), "a submenu's row opens it");

    let sel = Select::new(&look, options()).value(Some(0));
    let anchor = Rect::new(0, 0, 20, 1);
    let mut st = SelectState::default();
    assert_eq!(sel.press(&mut st, None), SelectEvent::Opened);
    let list = sel.list_area(window, anchor);
    assert_eq!(
        sel.option_at(window, anchor, Position::new(list.x + 2, list.y + 2), &st),
        Some(1)
    );
    assert_eq!(
        sel.option_at(window, anchor, Position::new(list.x + 2, list.y + 3), &st),
        None,
        "disabled"
    );
    assert_eq!(sel.press(&mut st, Some(1)), SelectEvent::Chose(1));
    assert!(!st.open);
}

#[test]
fn a_menu_opens_acts_and_closes_by_the_pointer() {
    use ratatui::layout::Position;
    use sokkel_tui::{MenuEvent, MenuState, Pointer, PointerKind};
    let look = look();
    let m = menu(&look);
    let window = Rect::new(0, 0, 80, 24);
    let anchor = Rect::new(10, 2, 8, 1);
    let press = |x, y| Pointer {
        kind: PointerKind::Press,
        at: Position::new(x, y),
    };
    let mut st = MenuState::default();
    assert_eq!(m.pointer(&mut st, window, anchor, press(12, 2)), MenuEvent::Opened);
    assert!(st.open);
    let (area, _) = m.areas(window, anchor, &st);
    assert_eq!(
        m.pointer(&mut st, window, anchor, press(area.x + 2, area.y)),
        MenuEvent::Ignored,
        "the box's edge"
    );
    assert_eq!(m.pointer(&mut st, window, anchor, press(70, 20)), MenuEvent::Closed);
    assert!(!st.open);
}

#[test]
fn a_radio_row_shows_its_check_and_keeps_the_menu_open() {
    use sokkel_tui::{MenuEvent, MenuState};
    let look = look();
    let m = DropdownMenu::new(
        &look,
        [
            PopupRow::Label("Scheme".into()),
            Item::new("Light").radio(false).into(),
            Item::new("Dark").radio(true).into(),
        ],
    );
    let mut st = MenuState {
        open: true,
        highlight: 1,
        sub: None,
    };
    assert_eq!(m.handle(&mut st, press(Key::Enter)), MenuEvent::Action(1, None));
    assert!(st.open, "a radio row keeps the menu open");
    let window = Rect::new(0, 0, 40, 10);
    let mut buf = Buffer::empty(window);
    m.render(window, Rect::new(0, 0, 10, 1), &mut buf, &st);
    assert!(snapshot::text(&buf).contains("✓ Dark"));
}
