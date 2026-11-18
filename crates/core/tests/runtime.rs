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

#[test]
fn a_banner_takes_its_row_under_the_top_bar_and_its_words_go_to_the_status_line_when_cut() {
    use sokkel_tui::{Banner, BannerTone};
    let look = look();
    let words = "disconnected from https://api.example.org: connection refused (os error 61)";
    let shell = |banner: bool| {
        Shell::new(
            &look,
            Rail::new(&look, "T", "Tidevann"),
            TopBar::new(&look),
            StatusLine::new(&look).note("Live", NoteTone::Live),
        )
        .banner(banner.then(|| Banner::new(&look, BannerTone::Critical, words)))
    };
    let narrow = Rect::new(0, 0, 60, 20);
    let mut buf = Buffer::empty(narrow);
    let mut st = ShellState::default();
    let without = shell(false).render(narrow, &mut buf, &mut st);
    let mut buf = Buffer::empty(narrow);
    let with = shell(true).render(narrow, &mut buf, &mut st);
    assert_eq!(with.y, without.y + 1, "the page a row lower while it shows");
    let text = sokkel_tui::snapshot::text(&buf);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines[2].starts_with(" ! disconnected from") && lines[2].contains('…'),
        "{lines:?}"
    );
    // The status line takes the words as its tip, in place of its note: as much as its row holds.
    assert!(lines[19].contains("disconnected from https://api.example.org") && !lines[19].contains("Live"));
    assert_eq!(
        buf[(3, 2)].bg,
        look.terminal(look.paint("banner.critical.rest.container", "ground").unwrap())
    );
}

#[test]
fn full_screen_drops_the_rail_and_the_top_bar_and_escape_leaves_it() {
    let look = look();
    let shell = Shell::new(
        &look,
        Rail::new(&look, "T", "Tidevann").groups([RailGroup::new(None, [RailEntry::new("Runs")])]),
        TopBar::new(&look).trail(["Tidevann", "Runs"]),
        StatusLine::new(&look).note("Live", NoteTone::Live),
    );
    let window = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(window);
    let mut st = ShellState::default();
    st.kiosk = true;
    let page = shell.render(window, &mut buf, &mut st);
    assert!(page.x < 5 && page.y < 3, "the page from the window's start: {page:?}");
    let text = sokkel_tui::snapshot::text(&buf);
    assert!(
        !text.contains("Tidevann") && text.contains("Live"),
        "no rail, no top bar, the status line kept"
    );
    assert_eq!(st.key(KeyPress::new(Key::Escape)), ShellEvent::Changed);
    assert!(!st.kiosk);
}

/// An app whose rail has a parent of two hosts; the page says which entry was chosen.
struct Hosts {
    chosen: String,
}

impl sokkel_tui::app::ShellApp for Hosts {
    fn chrome<'a>(&'a self, look: &'a Look) -> sokkel_tui::app::Chrome<'a> {
        sokkel_tui::app::Chrome {
            rail: Rail::new(look, "T", "Tidevann").groups([RailGroup::new(
                None,
                [
                    RailEntry::new("Hosts").icon("▣").parent(false),
                    RailEntry::new("api.example.org").child(),
                    RailEntry::new("harbor.example.org").child(),
                ],
            )]),
            top: TopBar::new(look),
            foot: StatusLine::new(look),
        }
    }

    fn page(&mut self, f: &mut ratatui::Frame, area: Rect, _cx: &mut sokkel_tui::app::Cx) {
        f.buffer_mut().set_string(
            area.x + 30,
            area.bottom() - 1,
            format!("chose {}", self.chosen),
            ratatui::style::Style::new(),
        );
    }

    fn entry(&mut self, group: usize, entry: usize) {
        self.chosen = format!("{group}.{entry}");
    }
}

#[test]
fn a_collapsed_parent_opens_its_children_in_a_flyout() {
    let shot = |keys: &[&str]| {
        let keys: Vec<KeyPress> = keys.iter().filter_map(|k| KeyPress::named(k)).collect();
        let app = Hosts { chosen: "none".into() };
        sokkel_tui::snapshot::text(&sokkel_tui::app::shot(
            app,
            look(),
            (100, 24),
            |s| s.collapsed = true,
            &keys,
        ))
    };
    let open = shot(&["tab", "enter"]);
    assert!(
        open.contains("api.example.org") && open.contains("harbor.example.org"),
        "{open}"
    );
    assert!(open.contains("chose none"), "the parent itself is no place");
    assert!(
        shot(&["tab", "enter", "down", "enter"]).contains("chose 0.2"),
        "harbor, by its place"
    );
}

/// A tall page beside a side panel.
struct Paneled;

impl sokkel_tui::app::ShellApp for Paneled {
    fn chrome<'a>(&'a self, look: &'a Look) -> sokkel_tui::app::Chrome<'a> {
        sokkel_tui::app::Chrome {
            rail: Rail::new(look, "T", "Tidevann"),
            top: TopBar::new(look),
            foot: StatusLine::new(look),
        }
    }

    fn page(&mut self, f: &mut ratatui::Frame, area: Rect, _cx: &mut sokkel_tui::app::Cx) {
        f.buffer_mut()
            .set_string(area.x, area.y, "the page", ratatui::style::Style::new());
    }

    fn page_rows(&self, _look: &Look, _width: u16) -> Option<u16> {
        Some(200)
    }

    fn panel_width(&self) -> Option<u16> {
        Some(44)
    }

    fn panel(&mut self, f: &mut ratatui::Frame, area: Rect, _cx: &mut sokkel_tui::app::Cx) {
        f.buffer_mut()
            .set_string(area.x, area.y, "the panel", ratatui::style::Style::new());
    }
}

#[test]
fn a_panel_over_a_tall_page_below_md_covers_it_whole() {
    // Below md the panel covers the page: no page, no scroller, and no panic.
    let text = sokkel_tui::snapshot::text(&sokkel_tui::app::shot(Paneled, look(), (80, 30), |_| {}, &[]));
    assert!(text.contains("the panel") && !text.contains("the page"), "{text}");
    // Wide, the tall page scrolls beside it.
    let text = sokkel_tui::snapshot::text(&sokkel_tui::app::shot(Paneled, look(), (160, 45), |_| {}, &[]));
    assert!(text.contains("the panel") && text.contains("the page"), "{text}");
}

#[test]
fn left_and_right_go_back_and_forward_along_the_routes() {
    let shot = |keys: &[&str]| {
        let keys: Vec<KeyPress> = keys.iter().filter_map(|k| KeyPress::named(k)).collect();
        let app = Places { at: "a".into() };
        sokkel_tui::snapshot::text(&sokkel_tui::app::shot(app, look(), (120, 30), |_| {}, &keys))
    };
    assert!(shot(&["b", "left"]).contains("at a"), "Left the page leaves goes back");
    assert!(
        shot(&["b", "left", "right"]).contains("at b"),
        "Right goes forward again"
    );
    assert!(
        shot(&["b", "left", "right", "right"]).contains("at b"),
        "nothing further forward"
    );
    // A new route clears what Forward had: back to a, then b again by its shortcut, then Right
    // has nowhere to go.
    assert!(shot(&["b", "esc", "b", "right"]).contains("at b"));
    assert!(shot(&["b", "esc", "b", "left"]).contains("at a"));
}

/// Places whose page holds one control.
struct Stopped(Places);

impl sokkel_tui::app::ShellApp for Stopped {
    fn route(&self) -> String {
        self.0.route()
    }
    fn go(&mut self, route: &str) {
        self.0.go(route)
    }
    fn chrome<'a>(&'a self, look: &'a Look) -> sokkel_tui::app::Chrome<'a> {
        self.0.chrome(look)
    }
    fn page(&mut self, f: &mut ratatui::Frame, area: Rect, cx: &mut sokkel_tui::app::Cx) {
        self.0.page(f, area, cx);
        cx.focus.stop(
            sokkel_tui::FocusId::new("control"),
            Rect {
                y: area.y + 1,
                height: 1,
                ..area
            },
        );
    }
    fn shortcuts(&self) -> Vec<sokkel_tui::Binding<String>> {
        self.0.shortcuts()
    }
    fn command(&mut self, id: &str) -> sokkel_tui::app::Ran {
        self.0.command(id)
    }
}

#[test]
fn a_focused_control_on_the_page_keeps_its_arrows() {
    let shot = |keys: &[&str]| {
        let keys: Vec<KeyPress> = keys.iter().filter_map(|k| KeyPress::named(k)).collect();
        let app = Stopped(Places { at: "a".into() });
        sokkel_tui::snapshot::text(&sokkel_tui::app::shot(app, look(), (120, 30), |_| {}, &keys))
    };
    // On b, focus moves to the page's control: Left is the control's, though it leaves it.
    assert!(shot(&["b", "left"]).contains("at b"));
    // Escape still goes back.
    assert!(shot(&["b", "esc"]).contains("at a"));
}
