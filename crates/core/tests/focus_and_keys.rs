use ratatui::layout::{Position, Rect};
use sokkel_tui::{Binding, Focus, FocusId, InputKind, Key, KeyContext, KeyPress, Keys, ShortcutWords, Shortcuts};

fn ids() -> (FocusId, FocusId, FocusId) {
    (FocusId::new("search"), FocusId::new("filter"), FocusId::new("new"))
}

/// One frame of the page: a search field and two buttons, in tab order.
fn frame(focus: &mut Focus) {
    let (search, filter, new) = ids();
    focus.begin();
    focus.text_stop(search, Rect::new(0, 0, 20, 1));
    focus.stop(filter, Rect::new(30, 0, 10, 1));
    focus.stop(new, Rect::new(40, 0, 7, 1));
    focus.end();
}

#[test]
fn tab_walks_the_order_round_and_back() {
    let (search, filter, new) = ids();
    let mut focus = Focus::new();
    frame(&mut focus);
    assert_eq!(focus.current(), None);
    assert!(focus.handle(Key::Tab.into()));
    assert_eq!(focus.current(), Some(search));
    focus.handle(Key::Tab.into());
    focus.handle(Key::Tab.into());
    assert_eq!(focus.current(), Some(new));
    focus.handle(Key::Tab.into());
    assert_eq!(focus.current(), Some(search), "round to the first");
    focus.handle(Key::BackTab.into());
    assert_eq!(focus.current(), Some(new));
    assert!(!focus.handle(Key::Enter.into()));
    focus.handle(Key::BackTab.into());
    assert_eq!(focus.current(), Some(filter));
}

#[test]
fn the_ring_shows_for_the_keyboard_only() {
    let (_, filter, _) = ids();
    let mut focus = Focus::new();
    frame(&mut focus);
    assert_eq!(focus.press(Position::new(32, 0)), Some(filter));
    assert!(focus.is(filter));
    assert!(!focus.ring(filter));
    assert_eq!(focus.input_kind(), InputKind::Pointer);
    focus.saw_key();
    assert!(focus.ring(filter));
    assert_eq!(
        focus.press(Position::new(25, 0)),
        None,
        "a press on no stop leaves focus"
    );
    assert!(focus.is(filter));
}

#[test]
fn typing_is_known_by_the_stop() {
    let (search, filter, _) = ids();
    let mut focus = Focus::new();
    frame(&mut focus);
    focus.set(search);
    assert!(focus.typing());
    focus.set(filter);
    assert!(!focus.typing());
}

#[test]
fn a_layer_keeps_tab_inside_and_gives_focus_back() {
    let (_, filter, _) = ids();
    let dialog = FocusId::new("dialog");
    let (cancel, delete) = (dialog.child(0), dialog.child(1));
    let mut focus = Focus::new();
    frame(&mut focus);
    focus.set(filter);
    focus.open_layer(dialog, Some(cancel));
    assert!(focus.under_layer());
    let draw = |focus: &mut Focus| {
        let (search, filter, new) = ids();
        focus.begin();
        focus.text_stop(search, Rect::new(0, 0, 20, 1));
        focus.stop(filter, Rect::new(30, 0, 10, 1));
        focus.stop(new, Rect::new(40, 0, 7, 1));
        focus.enter_layer(dialog);
        focus.stop(cancel, Rect::new(10, 10, 10, 1));
        focus.stop(delete, Rect::new(22, 10, 10, 1));
        focus.leave_layer();
        focus.end();
    };
    draw(&mut focus);
    assert!(focus.is(cancel));
    focus.handle(Key::Tab.into());
    assert!(focus.is(delete));
    focus.handle(Key::Tab.into());
    assert!(focus.is(cancel), "Tab stays inside the layer");
    assert_eq!(
        focus.press(Position::new(32, 0)),
        None,
        "the page under a layer takes no press"
    );
    assert_eq!(focus.close_layer(), Some(dialog));
    assert!(focus.is(filter), "focus goes back to what opened it");
}

#[test]
fn focus_on_a_stop_that_went_away_falls_to_the_first() {
    let (search, _, _) = ids();
    let mut focus = Focus::new();
    focus.set(FocusId::new("gone"));
    frame(&mut focus);
    assert!(focus.is(search));
}

#[test]
fn a_child_is_its_own_stop() {
    let row = FocusId::new("runs");
    assert_ne!(row.child(0), row.child(1));
    assert_eq!(row.child(3), FocusId::new("runs").child(3));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Palette,
    Shortcuts,
    Search,
    GoRuns,
    GoJobs,
}

fn keys() -> Shortcuts<Act> {
    Shortcuts::new()
        .bind(Binding::new(Keys::Mod('k'), Act::Palette, "Open the command palette").under_layer())
        .bind(Binding::new(
            Keys::Char('?'),
            Act::Shortcuts,
            "Show the keyboard shortcuts",
        ))
        .bind(Binding::new(Keys::Char('/'), Act::Search, "Search"))
        .bind(Binding::new(Keys::Then('g', 'r'), Act::GoRuns, "Go to runs"))
        .bind(Binding::new(Keys::Then('g', 'j'), Act::GoJobs, "Go to jobs"))
}

fn at(ms: u64) -> KeyContext {
    KeyContext {
        now_ms: ms,
        ..KeyContext::default()
    }
}

#[test]
fn a_shortcut_is_the_character_it_types() {
    let mut s = keys();
    assert_eq!(s.handle(Key::Char('?').into(), at(0)), Some(Act::Shortcuts));
    assert_eq!(s.handle(Key::Char('/').into(), at(0)), Some(Act::Search));
    assert_eq!(s.handle(KeyPress::ctrl(Key::Char('k')), at(0)), Some(Act::Palette));
    assert_eq!(s.handle(KeyPress::ctrl(Key::Char('K')), at(0)), Some(Act::Palette));
    assert_eq!(s.handle(Key::Char('k').into(), at(0)), None, "k alone is not Mod K");
}

#[test]
fn an_alt_chord_is_never_heard() {
    let mut s = keys();
    let alt = KeyPress {
        key: Key::Char('?'),
        ctrl: false,
        alt: true,
    };
    assert_eq!(s.handle(alt, at(0)), None);
}

#[test]
fn keys_keep_quiet_in_a_field_and_under_a_layer_unless_asked() {
    let mut s = keys();
    let typing = KeyContext { typing: true, ..at(0) };
    assert_eq!(s.handle(Key::Char('?').into(), typing), None);
    assert_eq!(s.handle(Key::Char('g').into(), typing), None);
    assert_eq!(s.pending(), None, "a sequence never starts in a field");
    let layer = KeyContext {
        under_layer: true,
        ..at(0)
    };
    assert_eq!(s.handle(Key::Char('/').into(), layer), None);
    assert_eq!(
        s.handle(KeyPress::ctrl(Key::Char('k')), layer),
        Some(Act::Palette),
        "the palette's key closes it again"
    );
}

#[test]
fn a_sequence_takes_its_second_key_within_a_second() {
    let mut s = keys();
    assert_eq!(s.handle(Key::Char('g').into(), at(0)), None);
    assert_eq!(s.pending(), Some('g'));
    assert_eq!(s.handle(Key::Char('j').into(), at(900)), Some(Act::GoJobs));
    assert_eq!(s.pending(), None);
    s.handle(Key::Char('g').into(), at(5000));
    assert_eq!(s.handle(Key::Char('r').into(), at(6200)), None, "too late");
    s.handle(Key::Char('g').into(), at(7000));
    assert_eq!(s.handle(Key::Escape.into(), at(7100)), None);
    assert_eq!(s.pending(), None, "another key drops the sequence");
}

#[test]
fn labels_are_written_in_words_that_are_fields() {
    let w = ShortcutWords::default();
    assert_eq!(Keys::Mod('k').label(&w), "Ctrl K");
    assert_eq!(Keys::Char('?').label(&w), "?");
    assert_eq!(Keys::Then('g', 'r').label(&w), "G then R");
    assert_eq!(Keys::Char(' ').label(&w), "Space");
}
