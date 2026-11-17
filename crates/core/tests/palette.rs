use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use sokkel_tui::cells::Width;
use sokkel_tui::palette::score;
use sokkel_tui::{
    Command, CommandGroup, CommandPalette, Key, KeyPress, Keys, Look, PaletteEvent, PaletteState, PaletteWords, Scheme,
    Setting, SettingsDialog, SettingsEvent, SettingsFocus, SettingsGroup, SettingsSection, SettingsState, snapshot,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn palette(look: &Look) -> CommandPalette<'_> {
    CommandPalette::new(
        look,
        [
            CommandGroup::new(
                "Go to",
                [
                    Command::new("go-runs", "Runs").keys(Keys::Then('g', 'r')),
                    Command::new("go-jobs", "Jobs"),
                ],
            ),
            CommandGroup::new(
                "Runs",
                [
                    Command::new("export", "Export runs").keywords(["download"]),
                    Command::new("pause", "Pause nightly-export").disabled("It is not running"),
                    Command::new("retry", "Retry sync-inventory"),
                ],
            ),
        ],
    )
}

fn press(k: Key) -> KeyPress {
    KeyPress::new(k)
}

#[test]
fn a_prefix_beats_a_word_start_beats_letters_in_order() {
    assert!(score("exp", "Export runs") > score("run", "Export runs"));
    assert!(score("run", "Export runs") > score("erns", "Export runs"));
    assert_eq!(score("zz", "Export runs"), None);
    assert!(score("", "anything").is_some());
}

#[test]
fn typing_narrows_and_keys_move_past_disabled_rows() {
    let look = look();
    let p = palette(&look);
    let mut s = PaletteState::default();
    for c in "down".chars() {
        assert_eq!(p.handle(&mut s, press(Key::Char(c))), PaletteEvent::Changed);
    }
    assert_eq!(p.matches(&s.query), vec![(1, vec![0])], "a keyword matches");
    assert_eq!(p.handle(&mut s, press(Key::Enter)), PaletteEvent::Run("export".into()));

    let mut s = PaletteState::default();
    p.handle(&mut s, press(Key::Down));
    p.handle(&mut s, press(Key::Down));
    assert_eq!(s.active, 2, "Export runs");
    p.handle(&mut s, press(Key::Down));
    assert_eq!(s.active, 4, "past the disabled row");
    p.handle(&mut s, press(Key::Down));
    assert_eq!(s.active, 4, "it stops at the last");
    p.handle(&mut s, press(Key::Home));
    assert_eq!(s.active, 0);
}

#[test]
fn escape_and_ctrl_k_close_it_unless_a_command_holds_it() {
    let look = look();
    let p = palette(&look);
    let mut s = PaletteState::default();
    assert_eq!(p.handle(&mut s, press(Key::Escape)), PaletteEvent::Close);
    assert_eq!(p.handle(&mut s, KeyPress::ctrl(Key::Char('k'))), PaletteEvent::Close);
    s.running("export", 1000);
    assert_eq!(p.handle(&mut s, press(Key::Escape)), PaletteEvent::Ignored);
    assert_eq!(p.handle(&mut s, KeyPress::ctrl(Key::Char('k'))), PaletteEvent::Ignored);
    assert_eq!(
        p.handle(&mut s, press(Key::Enter)),
        PaletteEvent::Ignored,
        "another choice is ignored"
    );
    s.running("export", 5000);
    assert_eq!(p.handle(&mut s, press(Key::Escape)), PaletteEvent::StopWaiting);
    s.active = 0;
    assert_eq!(p.handle(&mut s, press(Key::Enter)), PaletteEvent::StopWaiting);
}

#[test]
fn a_refusal_stands_under_the_list_until_the_next_key() {
    let look = look();
    let p = palette(&look);
    let mut s = PaletteState::default();
    s.refused(None, &PaletteWords::default());
    let area = p.area(Rect::new(0, 0, 120, 30), &s);
    assert_eq!(area.width, 46, "384 px");
    let mut buf = Buffer::empty(Rect::new(0, 0, 120, 30));
    p.render(area, &mut buf, &mut s);
    let text = snapshot::text(&buf);
    assert!(text.contains("! The command did not run."), "{text}");
    assert!(text.contains("G then R"), "a sequence's keys");
    assert!(
        text.contains("Pause nightly-export It is not running"),
        "a disabled row's reason"
    );
    p.handle(&mut s, press(Key::Char('r')));
    assert!(s.refusal.is_none());
    assert!(s.cursor_position().is_some());
}

#[test]
fn below_md_it_takes_the_window_width() {
    let look = look();
    let p = palette(&look).window(Width::Narrow);
    let a = p.area(Rect::new(0, 0, 80, 24), &PaletteState::default());
    assert_eq!((a.x, a.y, a.width), (2, 1, 76));
}

fn settings(look: &Look) -> SettingsDialog<'_> {
    SettingsDialog::new(
        look,
        [
            SettingsGroup::new(
                None,
                [SettingsSection::new(
                    "general",
                    "General",
                    [
                        Setting::new("theme", "Theme", 16).description("How every page looks."),
                        Setting::new("motion", "Reduce motion", 5).keywords(["animation"]),
                    ],
                )],
            ),
            SettingsGroup::new(
                Some("Workspace"),
                [SettingsSection::new("hosts", "Hosts", [Setting::new("timeout", "Timeout", 10)]).no_reset()],
            ),
        ],
    )
}

#[test]
fn the_search_narrows_the_sections_and_the_settings() {
    let look = look();
    let d = settings(&look);
    assert_eq!(d.sections(""), vec![(0, 0), (1, 0)]);
    assert_eq!(d.sections("animation"), vec![(0, 0)]);
    assert_eq!(d.kept((0, 0), "animation"), vec![1]);
    assert_eq!(d.kept((0, 0), "general"), vec![0, 1], "a section's name keeps it whole");
    assert!(d.sections("nothing like it").is_empty());
    let mut s = SettingsState::default();
    s.shown = (1, 0);
    s.query = "theme".into();
    assert_eq!(
        d.shown(&s),
        Some((0, 0)),
        "the first section left when the chosen one has no match"
    );
}

#[test]
fn entries_show_sections_and_reset_asks_first() {
    let look = look();
    let d = settings(&look);
    let mut s = SettingsState::default();
    s.focus = SettingsFocus::Entry(1, 0);
    assert_eq!(d.handle(&mut s, press(Key::Enter)), SettingsEvent::Changed);
    assert_eq!(s.shown, (1, 0));
    let area = d.area(Rect::new(0, 0, 120, 40));
    assert!(d.layout(area, &s).reset.is_none(), "Hosts says reset: false");
    s.shown = (0, 0);
    let l = d.layout(area, &s);
    assert!(l.reset.is_some() && l.done.is_some());
    assert_eq!(l.controls.len(), 2);
    assert_eq!(l.controls[0].1.right(), area.right() - 3, "a control at its row's end");
    s.focus = SettingsFocus::Reset;
    assert_eq!(d.handle(&mut s, press(Key::Enter)), SettingsEvent::AskReset);
    let confirm = d.reset_confirm(&s);
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 20));
    let r = confirm.area(buf.area, 1);
    ratatui::widgets::Widget::render(&confirm, r, &mut buf);
    assert!(snapshot::text(&buf).contains("Reset General to its defaults?"));
    assert_eq!(d.handle(&mut s, press(Key::Escape)), SettingsEvent::Close);
}

#[test]
fn below_md_it_is_a_list_then_a_section_with_back() {
    let look = look();
    let d = settings(&look).window(Width::Narrow);
    let mut s = SettingsState::default();
    s.focus = SettingsFocus::Entry(0, 0);
    let window = Rect::new(0, 0, 60, 24);
    assert_eq!(d.area(window), window);
    assert!(d.layout(window, &s).controls.is_empty(), "the list alone");
    d.handle(&mut s, press(Key::Enter));
    assert!(s.in_section);
    let l = d.layout(window, &s);
    assert!(l.back.is_some() && l.entries.is_empty());
    assert_eq!(
        l.controls[0].1.x,
        window.x + 2,
        "below md a control stands under its words, at the pane's start"
    );
    assert_eq!(d.handle(&mut s, press(Key::Escape)), SettingsEvent::Changed);
    assert!(!s.in_section, "Escape goes back to the list first");
}

#[test]
fn a_two_line_row_says_its_detail_and_the_foot_its_verb() {
    use sokkel_tui::{Command, CommandGroup, CommandPalette, Keys, PaletteHit, PaletteState};
    let look = Look::new("tidevann", Scheme::Dark).unwrap();
    let p = CommandPalette::new(
        &look,
        [CommandGroup::new(
            "Runs",
            [
                Command::new("a", "nightly-export").detail("harbor, every day at 02:00"),
                Command::new("b", "sync-inventory"),
            ],
        )],
    )
    .width_px(672.0)
    .foot("Tidevann", Some(Keys::Mod('k')));
    let window = Rect::new(0, 0, 120, 30);
    let mut st = PaletteState::default();
    let area = p.area(window, &st);
    let mut buf = Buffer::empty(window);
    p.render(area, &mut buf, &mut st);
    let text = snapshot::text(&buf);
    assert!(text.contains("harbor, every day at 02:00"), "{text}");
    assert!(text.contains("Tidevann") && text.contains("Enter") && text.contains("Command palette"));
    // The detail's row is its command's: a press there runs it.
    let lines: Vec<&str> = text.lines().collect();
    let y = lines.iter().position(|l| l.contains("harbor, every day")).unwrap() as u16;
    let x = lines[y as usize].find("harbor").unwrap() as u16;
    assert_eq!(
        p.hit(area, ratatui::layout::Position::new(x, y), &st),
        Some(PaletteHit::Command(0))
    );
}
