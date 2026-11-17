use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};
use sokkel_tui::text::{truncate, wrap};
use sokkel_tui::{
    Button, ButtonVariant, Charset, FieldEvent, FieldFocus, FieldState, FieldWords, IconPlace, Issue, Key, KeyPress,
    Look, RequiredMark, Scheme, TextField, snapshot,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn draw(w: u16, h: u16, f: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(f).unwrap();
    term.backend().buffer().clone()
}

fn row(buf: &Buffer, y: u16) -> String {
    snapshot::text(buf).lines().nth(y as usize).unwrap().to_string()
}

fn rgb(look: &Look, key: &str, field: &str) -> Color {
    look.fill(key, field)
}

#[test]
fn a_button_is_drawn_in_its_paint() {
    let look = look();
    let b = Button::new(&look, "Export");
    assert_eq!(b.width(), 8, "the label and a column of padding at each end");
    let buf = draw(8, 1, |f| f.render_widget(&b, f.area()));
    assert_eq!(row(&buf, 0), " Export ");
    let ground = rgb(&look, "button.default.rest.container", "ground");
    assert_eq!(buf[(0, 0)].bg, ground);
    assert_eq!(buf[(1, 0)].bg, ground);
    assert!(buf[(1, 0)].modifier.contains(Modifier::BOLD), "text.action is bold");
}

#[test]
fn focus_draws_the_ring_outside_and_keeps_the_place() {
    let look = look();
    let at_rest = Button::new(&look, "New").width();
    let b = Button::new(&look, "New").focused(true);
    assert_eq!(b.width(), at_rest);
    let buf = draw(b.width() + 2, 1, |f| f.render_widget(&b, Rect::new(1, 0, b.width(), 1)));
    assert_eq!(row(&buf, 0), "┃ New ┃", "the ring in the cell beside each end");
}

#[test]
fn an_edge_is_drawn_in_the_pad_cells() {
    let look = look();
    let b = Button::new(&look, "Filter").variant(ButtonVariant::Outline);
    let buf = draw(b.width(), 1, |f| f.render_widget(&b, f.area()));
    assert_eq!(row(&buf, 0), "▏Filter▕");
    assert_eq!(buf[(0, 0)].fg, rgb(&look, "button.outline.rest.container", "edge"));
}

#[test]
fn an_inherited_ink_is_its_flat_form() {
    let look = look();
    let b = Button::new(&look, "Open").variant(ButtonVariant::Ghost);
    let buf = draw(b.width(), 1, |f| f.render_widget(&b, f.area()));
    assert_eq!(buf[(1, 0)].fg, rgb(&look, "button.ghost.rest.container", "ink"));
}

#[test]
fn busy_keeps_the_label_and_its_focus_and_ignores_presses() {
    let look = look();
    let b = Button::new(&look, "Export").busy(true, 0).focused(true);
    assert!(b.focusable());
    assert!(!b.presses(Key::Enter.into()));
    let buf = draw(b.width() + 2, 1, |f| f.render_widget(&b, Rect::new(1, 0, b.width(), 1)));
    assert_eq!(row(&buf, 0), "┃ ⠋ Export ┃");
    // One frame per motion-fast.
    assert_eq!(look.spinner_frame(100), "⠙");
}

#[test]
fn a_spinner_takes_the_leading_icon_place() {
    let look = look();
    let b = Button::new(&look, "New run").with_icon("+", IconPlace::Start);
    assert_eq!(b.width(), Button::new(&look, "New run").busy(true, 0).width());
    let busy = b.clone().busy(true, 0);
    let buf = draw(busy.width(), 1, |f| f.render_widget(&busy, f.area()));
    assert_eq!(row(&buf, 0), " ⠋ New run ");
}

#[test]
fn a_reason_disables_but_keeps_the_tab_stop_and_is_the_tip() {
    let look = look();
    let b = Button::new(&look, "Export").reason(Some("Nothing to export yet"));
    assert!(b.focusable());
    assert!(!b.presses(Key::Enter.into()));
    assert_eq!(b.tip().unwrap().words, "Nothing to export yet");
    // Faded over the page: not the rest ground.
    let buf = draw(b.width(), 1, |f| f.render_widget(&b, f.area()));
    assert_ne!(buf[(2, 0)].bg, rgb(&look, "button.default.rest.container", "ground"));

    let gone = Button::new(&look, "Export").disabled(true);
    assert!(!gone.focusable());
}

#[test]
fn enter_and_space_press_an_enabled_button() {
    let look = look();
    let b = Button::new(&look, "New");
    assert!(b.presses(Key::Enter.into()));
    assert!(b.presses(Key::Char(' ').into()));
    assert!(!b.presses(KeyPress::ctrl(Key::Enter)));
    assert!(!b.presses(Key::Char('n').into()));
}

#[test]
fn an_icon_button_is_named_by_its_label() {
    let look = look();
    let b = Button::icon(&look, "⋯", "More actions").shortcut("Ctrl K");
    assert_eq!(b.width(), 3, "an icon button is three columns");
    let tip = b.tip().unwrap();
    assert_eq!(tip.words, "More actions");
    assert_eq!(tip.shortcut.as_deref(), Some("Ctrl K"));
    let buf = draw(3, 1, |f| f.render_widget(&b, f.area()));
    assert_eq!(buf[(1, 0)].fg, rgb(&look, "button.icon-button.rest.container", "ink"));
}

#[test]
fn a_cramped_button_cuts_its_label() {
    let look = look();
    let b = Button::new(&look, "Export everything");
    let buf = draw(10, 1, |f| f.render_widget(&b, f.area()));
    assert_eq!(row(&buf, 0), " Export…  ", "cut at the word, the ground to the end");
}

#[test]
fn ascii_draws_every_glyph_in_ascii() {
    let look = look().with_charset(Charset::Ascii);
    let b = Button::new(&look, "Filter")
        .variant(ButtonVariant::Outline)
        .focused(true);
    let buf = draw(b.width() + 2, 1, |f| f.render_widget(&b, Rect::new(1, 0, b.width(), 1)));
    assert_eq!(row(&buf, 0), ">[Filter]<");
}

#[test]
fn a_field_is_its_label_and_control() {
    let look = look();
    let field = TextField::new(&look, "Job name").placeholder("nightly-export");
    assert_eq!(field.height(30), 2);
    let mut state = FieldState::default();
    let buf = draw(30, 2, |f| f.render_stateful_widget(&field, f.area(), &mut state));
    assert_eq!(row(&buf, 0), format!("{:<30}", "Job name"));
    assert_eq!(row(&buf, 1), format!("▏{:<28}▕", "nightly-export"));
    assert_eq!(
        buf[(1, 1)].fg,
        rgb(&look, "text-field.base.rest.control", "placeholder")
    );
}

#[test]
fn an_issue_never_changes_the_height() {
    let look = look();
    let plain = TextField::new(&look, "Schedule").help("In cron's five fields.");
    let refused = plain.clone().error(Some("Not a schedule."));
    assert_eq!(plain.height(40), refused.height(40));
    let mut state = FieldState::new("0 25 * * *");
    let buf = draw(30, 3, |f| f.render_stateful_widget(&refused, f.area(), &mut state));
    // The value's room ends three columns before the end; the mark one column in from it.
    assert_eq!(row(&buf, 1), format!("▏{:<26} !▕", "0 25 * * *"));
    assert_eq!(state.mark_area(), Some(Rect::new(27, 1, 3, 1)));
    assert_eq!(refused.stops(), 2);
    assert_eq!(refused.mark_name().as_deref(), Some("Error: Not a schedule."));
}

#[test]
fn the_worst_issue_sets_the_look() {
    let look = look();
    let field = TextField::new(&look, "Timeout").issues([Issue::warning("Long.")]);
    let mut state = FieldState::new("90m");
    let buf = draw(20, 2, |f| f.render_stateful_widget(&field, f.area(), &mut state));
    assert_eq!(row(&buf, 1).chars().nth(18), Some('▲'));
    let field = field.issues([Issue::critical("Not a duration.")]);
    let buf = draw(20, 2, |f| f.render_stateful_widget(&field, f.area(), &mut state));
    assert_eq!(row(&buf, 1).chars().nth(18), Some('!'));
}

#[test]
fn typing_edits_and_the_cursor_follows() {
    let look = look();
    let field = TextField::new(&look, "Host");
    let mut state = FieldState::default().focused(FieldFocus::Control);
    for c in "api.example.org".chars() {
        assert_eq!(state.handle(&field, Key::Char(c).into()), FieldEvent::Edited);
    }
    assert_eq!(state.handle(&field, Key::Backspace.into()), FieldEvent::Edited);
    assert_eq!(state.handle(&field, Key::Home.into()), FieldEvent::Moved);
    assert_eq!(state.handle(&field, Key::Delete.into()), FieldEvent::Edited);
    assert_eq!(state.value(), "pi.example.or");
    assert_eq!(state.handle(&field, Key::Enter.into()), FieldEvent::Enter);
    assert_eq!(state.handle(&field, Key::Escape.into()), FieldEvent::Escape);
    // Ctrl chords go to the app.
    assert_eq!(
        state.handle(&field, KeyPress::ctrl(Key::Char('k'))),
        FieldEvent::Ignored
    );

    // A long value scrolls so the cursor stays in view.
    state.handle(&field, Key::End.into());
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    ratatui::widgets::StatefulWidget::render(&field, buf.area, &mut buf, &mut state);
    let pos = state.cursor_position().unwrap();
    assert_eq!(pos.y, 1);
    assert_eq!(pos.x, 8);
    // Eight columns of room: the value's end, and the cell the cursor stands in.
    assert_eq!(row(&buf, 1), "▏mple.or ▕");
}

#[test]
fn wide_glyphs_are_measured() {
    let look = look();
    let field = TextField::new(&look, "Name");
    let mut state = FieldState::new("日本").focused(FieldFocus::Control);
    let mut buf = Buffer::empty(Rect::new(0, 0, 10, 2));
    ratatui::widgets::StatefulWidget::render(&field, buf.area, &mut buf, &mut state);
    assert_eq!(state.cursor_position().unwrap().x, 5);
    state.handle(&field, Key::Left.into());
    assert_eq!(state.handle(&field, Key::Backspace.into()), FieldEvent::Edited);
    assert_eq!(state.value(), "本");
}

#[test]
fn read_only_moves_but_never_edits_and_disabled_is_no_stop() {
    let look = look();
    let field = TextField::new(&look, "Project").read_only(true);
    let mut state = FieldState::new("harbor").focused(FieldFocus::Control);
    assert_eq!(state.handle(&field, Key::Char('x').into()), FieldEvent::Ignored);
    assert_eq!(state.handle(&field, Key::Left.into()), FieldEvent::Moved);
    assert_eq!(state.value(), "harbor");
    assert_eq!(TextField::new(&look, "Owner").disabled(true).stops(), 0);
}

#[test]
fn the_mark_opens_its_popover_and_escape_closes_it() {
    let look = look();
    let field = TextField::new(&look, "Schedule").issues([Issue::critical("Not a schedule.")
        .detail("Five fields.")
        .action("Use 0 2 * * *")]);
    let mut state = FieldState::new("0 25 * * *").focused(FieldFocus::Mark);
    assert_eq!(state.handle(&field, Key::Enter.into()), FieldEvent::IssuesOpened);
    let buf = draw(50, 12, |f| {
        f.render_stateful_widget(&field, Rect::new(0, 0, 50, 2), &mut state);
        if let Some(p) = field.popover(&state) {
            f.render_widget(p, f.area());
        }
    });
    let text = snapshot::text(&buf);
    assert!(text.contains("! Not a schedule."), "{text}");
    assert!(text.contains("Five fields."));
    assert!(text.contains("╭"));
    assert_eq!(field.tip(&state).as_deref(), Some("Error: Not a schedule."));
    assert_eq!(state.handle(&field, Key::Escape.into()), FieldEvent::IssuesClosed);
    assert!(field.popover(&state).is_none());
}

#[test]
fn required_says_its_word_and_words_are_fields() {
    let look = look();
    let words = FieldWords {
        required: "needed".into(),
        ..FieldWords::default()
    };
    let field = TextField::new(&look, "Owner").required(RequiredMark::Word).words(words);
    let mut state = FieldState::default();
    let buf = draw(20, 2, |f| f.render_stateful_widget(&field, f.area(), &mut state));
    assert!(row(&buf, 0).starts_with("Owner needed"));
    let d = FieldWords::default();
    assert_eq!(d.errors.say(1), "1 error");
    assert_eq!(d.errors.say(1200), "1200 errors");
    assert_eq!(d.warnings.say(1), "1 warning");
}

#[test]
fn text_is_cut_and_wrapped_in_cells() {
    assert_eq!(truncate("nightly-export", 8, "…"), "nightly…");
    assert_eq!(truncate("harbor", 8, "…"), "harbor");
    assert_eq!(wrap("cron reads five fields", 10), vec!["cron reads", "five", "fields"]);
    assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
    assert_eq!(wrap("日本語", 4), vec!["日本", "語"]);
}

#[test]
fn a_text_area_wraps_grows_and_moves_by_lines() {
    let look = look();
    let field = TextField::new(&look, "Note").multiline(3, 5);
    assert_eq!(field.height(20), 4, "the label and three rows at the fewest");
    let mut state = FieldState::default().focused(FieldFocus::Control);
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 8));
    ratatui::widgets::StatefulWidget::render(&field, buf.area, &mut buf, &mut state);
    for c in "nightly-export failed twice".chars() {
        state.handle(&field, Key::Char(c).into());
    }
    assert_eq!(
        state.handle(&field, Key::Enter.into()),
        FieldEvent::Edited,
        "Enter starts a line"
    );
    for c in "ask ada".chars() {
        state.handle(&field, Key::Char(c).into());
    }
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 8));
    ratatui::widgets::StatefulWidget::render(&field, buf.area, &mut buf, &mut state);
    let lines: Vec<String> = snapshot::text(&buf).lines().map(String::from).collect();
    // 18 columns of room: the first paragraph wraps into two lines, the second is a third.
    assert_eq!(lines[1], "▏nightly-export fai▕");
    assert_eq!(lines[2], "▏led twice         ▕");
    assert_eq!(lines[3], "▏ask ada           ▕");
    assert_eq!(field.height_with(20, &state), 4);
    assert_eq!(state.cursor_position(), Some(ratatui::layout::Position::new(8, 3)));
    // Up goes to the line before, at the same column.
    assert_eq!(state.handle(&field, Key::Up.into()), FieldEvent::Moved);
    state.handle(&field, Key::Home.into());
    state.handle(&field, Key::Char('X').into());
    assert!(
        state.value().contains("\nask ada") && state.value().contains("Xled"),
        "{:?}",
        state.value()
    );
    // More lines grow it, up to five rows.
    for _ in 0..6 {
        state.handle(&field, Key::End.into());
        state.handle(&field, Key::Enter.into());
    }
    assert_eq!(field.height_with(20, &state), 6, "the label and five rows at the most");
}

#[test]
fn fields_in_a_row_share_their_description_rows() {
    let look = look();
    let plain = TextField::new(&look, "Minute");
    let told = TextField::new(&look, "Hour").description("0 to 23, the host's time.");
    let shared = TextField::level([(&plain, 14), (&told, 14)]);
    assert_eq!(shared, 2, "the description wraps to two rows at 14 columns");
    let plain = plain.in_row(shared);
    assert_eq!(plain.height(14), 4, "label, two shared rows, control");
    let mut buf = Buffer::empty(Rect::new(0, 0, 14, 4));
    ratatui::widgets::StatefulWidget::render(&plain, buf.area, &mut buf, &mut FieldState::new("0"));
    let lines: Vec<String> = snapshot::text(&buf).lines().map(String::from).collect();
    assert!(lines[3].starts_with("▏0"), "the control on the fourth row: {lines:?}");
}
