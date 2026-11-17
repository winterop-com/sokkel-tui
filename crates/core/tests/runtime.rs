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

#[test]
fn the_search_takes_typing_until_enter_and_says_where_the_cursor_is() {
    let look = look();
    let shell = Shell::new(
        &look,
        Rail::new(&look, "T", "Tidevann"),
        TopBar::new(&look).search(""),
        StatusLine::new(&look),
    );
    let window = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(window);
    let mut st = ShellState::default();
    shell.render(window, &mut buf, &mut st);
    assert!(st.focus_search());
    for c in "sync".chars() {
        assert_eq!(st.key(KeyPress::new(Key::Char(c))), ShellEvent::Search);
    }
    assert_eq!(st.query, "sync");
    shell.render(window, &mut buf, &mut st);
    let cursor = st.cursor_position().expect("a cursor while typing");
    // The field at column 30 (the rail, its edge, a gap): edge, magnifier, a space, the query.
    assert_eq!(cursor.x, 30 + 3 + 4, "after the magnifier and the query: {cursor:?}");
    assert!(sokkel_tui::snapshot::text(&buf).contains("⌕ sync"));
    assert_eq!(st.key(KeyPress::new(Key::Enter)), ShellEvent::Changed);
    assert!(!st.searching);
    shell.render(window, &mut buf, &mut st);
    assert_eq!(st.cursor_position(), None);
}

#[test]
fn the_look_is_kept_under_the_webs_keys_and_read_back() {
    use sokkel_tui::app::{APPEARANCE_KEY, SCHEME_KEY, read_look, write_look};
    let dir = std::env::temp_dir().join(format!("sokkel-tui-kept-{}", std::process::id()));
    let path = dir.join("appearance.json");
    let amber = Look::new("amber", Scheme::Light)
        .unwrap()
        .with_density(Density::Compact);
    write_look(&path, &amber).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(SCHEME_KEY) && text.contains(APPEARANCE_KEY) && text.contains("\"amber\""));
    let back = read_look(&path, &look()).unwrap();
    assert_eq!(
        (back.theme(), back.scheme(), back.density()),
        ("amber", Scheme::Light, Density::Compact)
    );
    assert!(read_look(&dir.join("missing.json"), &look()).is_none());
    std::fs::remove_dir_all(dir).unwrap();
}

/// A small app on the runtime: one refused command.
struct Refuses;

impl sokkel_tui::app::ShellApp for Refuses {
    fn chrome<'a>(&'a self, look: &'a Look) -> sokkel_tui::app::Chrome<'a> {
        sokkel_tui::app::Chrome {
            rail: Rail::new(look, "T", "Tidevann").groups([RailGroup::new(None, [RailEntry::new("Runs")])]),
            top: TopBar::new(look).trail(["Tidevann", "Runs"]),
            foot: StatusLine::new(look),
        }
    }

    fn page(&mut self, _f: &mut ratatui::Frame, _area: Rect, _cx: &mut sokkel_tui::app::Cx) {}

    fn commands(&self) -> Vec<sokkel_tui::CommandGroup> {
        vec![sokkel_tui::CommandGroup::new(
            "Runs",
            [sokkel_tui::Command::new("retry", "Retry sync-inventory")],
        )]
    }

    fn command(&mut self, _id: &str) -> sokkel_tui::app::Ran {
        sokkel_tui::app::Ran::Refused(Some("sync-inventory is already running".into()))
    }
}

#[test]
fn the_runtime_opens_the_palette_and_keeps_it_open_with_a_refusal() {
    let keys: Vec<KeyPress> = ["ctrl-k", "r", "e", "t", "r", "y", "enter"]
        .iter()
        .filter_map(|k| KeyPress::named(k))
        .collect();
    let buf = sokkel_tui::app::shot(Refuses, look(), (120, 30), |_| {}, &keys);
    let text = sokkel_tui::snapshot::text(&buf);
    assert!(text.contains("⌕ retry"), "the query kept: {text}");
    assert!(text.contains("sync-inventory is already running"));
    // A second Ctrl+K closes it; Escape with nothing open leaves the app running.
    let keys: Vec<KeyPress> = ["ctrl-k", "ctrl-k", "esc"]
        .iter()
        .filter_map(|k| KeyPress::named(k))
        .collect();
    let text = sokkel_tui::snapshot::text(&sokkel_tui::app::shot(Refuses, look(), (120, 30), |_| {}, &keys));
    assert!(!text.contains("Search or run a command"));
}

/// An app with two places: `b` goes to the second, the page says where it is.
struct Places {
    at: String,
}

impl sokkel_tui::app::ShellApp for Places {
    fn route(&self) -> String {
        self.at.clone()
    }

    fn go(&mut self, route: &str) {
        self.at = route.into();
    }

    fn chrome<'a>(&'a self, look: &'a Look) -> sokkel_tui::app::Chrome<'a> {
        sokkel_tui::app::Chrome {
            rail: Rail::new(look, "T", "Tidevann"),
            top: TopBar::new(look),
            foot: StatusLine::new(look),
        }
    }

    fn page(&mut self, f: &mut ratatui::Frame, area: Rect, _cx: &mut sokkel_tui::app::Cx) {
        f.buffer_mut()
            .set_string(area.x, area.y, format!("at {}", self.at), ratatui::style::Style::new());
    }

    fn shortcuts(&self) -> Vec<sokkel_tui::Binding<String>> {
        vec![sokkel_tui::Binding::new(
            sokkel_tui::Keys::Char('b'),
            "b".into(),
            "Go to b",
        )]
    }

    fn command(&mut self, id: &str) -> sokkel_tui::app::Ran {
        self.at = id.into();
        sokkel_tui::app::Ran::Done
    }
}

#[test]
fn escape_with_nothing_open_goes_back_along_the_routes() {
    let shot = |keys: &[&str]| {
        let keys: Vec<KeyPress> = keys.iter().filter_map(|k| KeyPress::named(k)).collect();
        let app = Places { at: "a".into() };
        sokkel_tui::snapshot::text(&sokkel_tui::app::shot(app, look(), (120, 30), |_| {}, &keys))
    };
    assert!(shot(&["b"]).contains("at b"));
    assert!(shot(&["b", "backspace"]).contains("at b"), "Backspace is no Back key");
    assert!(shot(&["b", "esc"]).contains("at a"));
    assert!(shot(&["b", "esc", "esc"]).contains("at a"), "nothing further back");
}
