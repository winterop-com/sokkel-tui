//! The runtime's pieces: the settings dialog's choices, Tab order and presses, the shell's state.

use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use sokkel_tui::settings::{look_chose, look_section};
use sokkel_tui::{
    Density, Fact, Key, KeyPress, Look, NoteTone, Pointer, PointerKind, Rail, RailEntry, RailGroup, Scheme, Setting,
    SettingsDialog, SettingsEvent, SettingsFocus, SettingsGroup, SettingsSection, SettingsState, Shell, ShellEvent,
    ShellState, StatusLine, TopBar,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn press(at: (u16, u16)) -> Pointer {
    Pointer {
        kind: PointerKind::Press,
        at: Position::new(at.0, at.1),
    }
}

#[test]
fn a_choice_turns_by_key_and_by_press_and_the_look_follows() {
    let look = look();
    let dialog = SettingsDialog::new(&look, [SettingsGroup::new(None, [look_section(&look)])]);
    let mut st = SettingsState::default();
    // Tab goes from the search through the entry to the first choice.
    assert_eq!(
        dialog.order(&st)[..3],
        [
            SettingsFocus::Search,
            SettingsFocus::Entry(0, 0),
            SettingsFocus::Control(0)
        ]
    );
    dialog.handle(&mut st, KeyPress::new(Key::Tab));
    dialog.handle(&mut st, KeyPress::new(Key::Tab));
    dialog.handle(&mut st, KeyPress::new(Key::Tab));
    assert_eq!(st.focus, SettingsFocus::Control(1));
    // The scheme is dark (option 1): a press turns it to light (option 0).
    let event = dialog.handle(&mut st, KeyPress::new(Key::Enter));
    assert_eq!(event, SettingsEvent::Chose(1, 0));
    let id = dialog.setting(&st, 1).unwrap().id.clone();
    let light = look_chose(&look, &id, 0).unwrap();
    assert_eq!(light.scheme(), Scheme::Light);
    // By the pointer: the dialog drawn, a press on the density's control.
    let window = Rect::new(0, 0, 120, 40);
    let area = dialog.area(window);
    let mut buf = Buffer::empty(window);
    dialog.render(area, &mut buf, &mut st);
    let control = dialog.layout(area, &st).controls[2].1;
    let event = dialog.press(&mut st, area, Position::new(control.x, control.y));
    assert_eq!(event, SettingsEvent::Chose(2, 1));
    assert_eq!(look_chose(&look, "density", 1).unwrap().density(), Density::Compact);
    // A press outside closes it.
    assert_eq!(dialog.press(&mut st, area, Position::new(0, 0)), SettingsEvent::Close);
}

#[test]
fn a_choice_is_drawn_saying_its_option() {
    let look = look();
    let section = SettingsSection::new(
        "frame",
        "Frame",
        [Setting::new("rail", "Rail", 0).choice(["Expanded", "Collapsed"], 1)],
    );
    let dialog = SettingsDialog::new(&look, [SettingsGroup::new(None, [section])]);
    let window = Rect::new(0, 0, 120, 40);
    let mut buf = Buffer::empty(window);
    let mut st = SettingsState::default();
    dialog.render(dialog.area(window), &mut buf, &mut st);
    assert!(sokkel_tui::snapshot::text(&buf).contains("▏Collapsed▕"));
}

fn shell(look: &Look) -> Shell<'_> {
    Shell::new(
        look,
        Rail::new(look, "T", "Tidevann")
            .groups([RailGroup::new(None, [RailEntry::new("Runs"), RailEntry::new("Jobs")])]),
        TopBar::new(look).trail(["Tidevann", "Runs"]),
        StatusLine::new(look)
            .note("Live", NoteTone::Live)
            .facts([Fact::new("12s ago")]),
    )
}

#[test]
fn the_shell_collapses_goes_to_entries_and_opens_its_drawer_below_md() {
    let look = look();
    let mut st = ShellState::default();
    let wide = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(wide);
    let page = shell(&look).render(wide, &mut buf, &mut st);
    assert!(page.x > 29, "the page beside the rail: {page:?}");
    // The collapse control at the head's end.
    assert_eq!(st.pointer(press((26, 0))), ShellEvent::Changed);
    assert!(st.collapsed);
    shell(&look).render(wide, &mut buf, &mut st);
    // Collapsed, the head expands it again.
    assert_eq!(st.pointer(press((1, 0))), ShellEvent::Changed);
    assert!(!st.collapsed);
    shell(&look).render(wide, &mut buf, &mut st);
    let ShellEvent::Entry(0, 1) = st.pointer(press((3, 3))) else {
        panic!("a press on Jobs goes there")
    };
    // Below md: the menu button opens the drawer, Escape closes it.
    let narrow = Rect::new(0, 0, 60, 30);
    let mut buf = Buffer::empty(narrow);
    shell(&look).render(narrow, &mut buf, &mut st);
    assert_eq!(st.pointer(press((1, 0))), ShellEvent::Changed);
    assert!(st.drawer);
    shell(&look).render_drawer(&mut buf, &mut st);
    assert!(st.covers(Position::new(2, 5)));
    assert_eq!(st.key(KeyPress::new(Key::Escape)), ShellEvent::Changed);
    assert!(!st.drawer);
}

#[test]
fn a_press_on_the_account_asks_for_its_menu() {
    use sokkel_tui::RailFoot;
    let look = look();
    let shell = Shell::new(
        &look,
        Rail::new(&look, "T", "Tidevann")
            .groups([RailGroup::new(None, [RailEntry::new("Runs")])])
            .foot(RailFoot {
                letter: "a".into(),
                color: "kind-violet".into(),
                name: "ada".into(),
            }),
        TopBar::new(&look),
        StatusLine::new(&look),
    );
    let window = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(window);
    let mut st = ShellState::default();
    shell.render(window, &mut buf, &mut st);
    assert_eq!(st.account(), Some(Rect::new(0, 29, 28, 1)));
    assert_eq!(st.pointer(press((5, 29))), ShellEvent::Account);
}
