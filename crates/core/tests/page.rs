use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use sokkel_tui::page_state::heading;
use sokkel_tui::text::wrap_code;
use sokkel_tui::{
    Button, ButtonVariant, Key, Look, Outcome, PageHeader, PageState, PageStateWords, Scheme, Section, SectionTone,
    SectionVariant, StateTone, snapshot,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn draw(w: u16, h: u16, widget: impl Widget) -> (Buffer, String) {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    widget.render(buf.area, &mut buf);
    let text = snapshot::text(&buf);
    (buf, text)
}

#[test]
fn a_failure_is_headed_by_what_the_server_answered() {
    let w = PageStateWords::default();
    assert_eq!(
        heading(Some(Outcome::Unreachable), StateTone::Warning, &w),
        "The server is not answering"
    );
    assert_eq!(
        heading(Some(Outcome::Status(404)), StateTone::Critical, &w),
        "Nothing is there"
    );
    assert_eq!(
        heading(Some(Outcome::Status(500)), StateTone::Warning, &w),
        "Part of this page could not be read"
    );
    assert_eq!(
        heading(Some(Outcome::Status(422)), StateTone::Critical, &w),
        "The server refused this read"
    );
    assert_eq!(heading(None, StateTone::Critical, &w), "The server refused this read");
}

#[test]
fn a_failed_read_says_its_diagnostic_problems_and_try_again() {
    let look = look();
    let s = PageState::failed(&look, Some(Outcome::Status(422)))
        .diagnostic("The schedule `0 25 * * *` is not one.")
        .problems(["The hour is 0 to 23."])
        .retry(false);
    let h = s.height(50);
    let (buf, text) = draw(50, h, &s);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(h as usize, lines.len());
    assert!(lines[2].contains("! The server refused this read"), "{text}");
    assert!(lines[3].contains("The schedule 0 25 * * * is not one."));
    assert!(lines[4].contains("• The hour is 0 to 23."));
    assert!(lines[6].contains("↻ Try again"));
    // The code part stands on its chip.
    let x = lines[3].find("0 25").unwrap() as u16;
    let chip = look.fill("page-state.base.failed.code", "ground");
    assert_eq!(buf[(x, 3)].bg, chip);
    // Try again is where the layout says, and Enter presses it.
    let r = s.retry_area(Rect::new(0, 0, 50, h)).unwrap();
    assert_eq!(r.y, 6);
    assert_eq!(
        lines[6].chars().nth(r.x as usize + 1),
        Some('↻'),
        "the arrow after the pad cell"
    );
    assert!(s.retry_button().unwrap().presses(Key::Enter.into()));
}

#[test]
fn loading_and_empty_and_unframed() {
    let look = look();
    let (_, text) = draw(30, 5, PageState::loading(&look, 0));
    assert!(text.lines().nth(2).unwrap().contains("⠋ Loading"));
    let empty = PageState::empty(&look, "No runs yet.");
    let (_, text) = draw(30, empty.height(30), &empty);
    assert!(text.contains("No runs yet."));
    let bare = PageState::failed(&look, None).framed(false);
    assert_eq!(bare.height(40), 1, "unframed: no padding rows, no edge");
    assert_eq!(bare.height(30), 2, "the heading wraps");
    let (_, text) = draw(40, 1, &bare);
    assert!(!text.contains('╭'));
    let warn = PageState::failed(&look, None).tone(StateTone::Warning);
    let (_, text) = draw(50, warn.height(50), &warn);
    assert!(text.contains("▲ Part of this page could not be read"));
}

#[test]
fn a_header_keeps_its_actions_beside_the_title_while_there_is_room() {
    let look = look();
    let h = PageHeader::new(&look, "Runs").count("1,200 runs").actions([
        Button::new(&look, "Filter").variant(ButtonVariant::Secondary),
        Button::new(&look, "New"),
    ]);
    assert_eq!(h.height(80), 1);
    let (_, text) = draw(80, 1, &h);
    assert!(text.starts_with("Runs 1,200 runs"));
    assert!(text.trim_end().ends_with("Filter   New"), "{text}");
    // Too narrow: they wrap under the title, a blank row down.
    assert_eq!(h.height(20), 3);
    let l = h.layout(Rect::new(0, 0, 20, 3));
    assert!(l.wrapped);
    assert_eq!(l.actions[0].1.y, 2);
}

#[test]
fn items_fold_into_more_from_the_last() {
    let look = look();
    let h = PageHeader::new(&look, "nightly-export")
        .items(["Run now", "Pause", "Duplicate", "Archive", "Delete"].map(|w| Button::new(&look, w)))
        .primary(Button::new(&look, "Edit"));
    let wide = h.layout(Rect::new(0, 0, 120, 1));
    assert_eq!(wide.folded, 0);
    assert!(wide.more.is_none());
    let narrow = h.layout(Rect::new(0, 0, 64, 1));
    assert!(narrow.folded > 0 && narrow.folded < 5, "{narrow:?}");
    assert!(narrow.more.is_some());
    assert!(narrow.primary.is_some(), "the primary never folds");
    assert_eq!(narrow.items.len() + narrow.folded, 5);
}

#[test]
fn a_trail_shows_only_outside_a_shell() {
    let look = look();
    let inside = PageHeader::new(&look, "Run 4812").trail(["Tidevann", "Runs"], true);
    assert_eq!(inside.height(40), 1);
    let outside = PageHeader::new(&look, "Run 4812").trail(["Tidevann", "Runs"], false);
    let (_, text) = draw(40, 2, &outside);
    assert!(text.starts_with("Tidevann › Runs ›"));
}

#[test]
fn a_boxed_section_has_its_head_over_a_rule() {
    let look = look();
    let s = Section::new(&look, "Recent runs").meta("12");
    let area = Rect::new(0, 0, 30, s.height(30, 1));
    let (_, text) = draw(30, area.height, &s);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with('╭'));
    assert!(lines[1].starts_with("│  Recent runs 12"), "{text}");
    assert!(lines[2].starts_with('├') && lines[2].ends_with('┤'));
    let body = s.body(area);
    assert_eq!((body.x, body.width), (3, 24), "two columns of padding at regular");
    assert_eq!(body.y, 4, "a padding row at regular density");
    assert_eq!(body.height, 1);
}

#[test]
fn compact_drops_the_padding_rows() {
    let look = look().with_density(sokkel_tui::Density::Compact);
    let s = Section::new(&look, "Recent runs");
    assert_eq!(s.height(30, 1), 5);
}

#[test]
fn critical_and_flat() {
    let look = look();
    let s = Section::new(&look, "Failing hosts").tone(SectionTone::Critical);
    let (_, text) = draw(30, s.height(30, 1), &s);
    assert!(text.starts_with("╭━━"));
    assert!(text.contains("▲ Failing hosts"), "{text}");
    let flat = Section::new(&look, "Settings").variant(SectionVariant::Flat);
    let (_, text) = draw(30, flat.height(30, 1), &flat);
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].starts_with("Settings"));
    assert!(lines[1].starts_with("────"));
    assert_eq!(flat.body(Rect::new(0, 0, 30, 10)).y, 3, "the section gap");
}

#[test]
fn code_stays_whole_and_punctuation_keeps_its_word() {
    let lines = wrap_code("runs at `0 2 * * *`.", 12);
    let text: Vec<String> = lines
        .iter()
        .map(|l| l.iter().map(|(s, _)| s.as_str()).collect())
        .collect();
    assert_eq!(text, vec!["runs at", "0 2 * * *."]);
    assert!(lines[1][0].1, "the code part is marked");
}

#[test]
fn a_filled_section_scrolls_its_body_with_a_scroller_at_its_end() {
    let look = look();
    let s = Section::new(&look, "Last runs").fill(12, 3);
    let area = Rect::new(0, 0, 40, 10);
    let body = s.body(area);
    assert_eq!(
        body.width,
        40 - 2 - 4 - 1,
        "the box, the padding, the scroller's column"
    );
    assert_eq!(s.scrolled(area, 12, 3, 100), 12 - body.height, "held at the end");
    assert_eq!(s.scrolled(area, 12, 3, -100), 0);
    let (_, text) = draw(40, 10, &s);
    let x = body.right() as usize;
    let column: String = text.lines().map(|l| l.chars().nth(x).unwrap_or(' ')).collect();
    assert!(
        column.contains('┃') && column.contains('│'),
        "the thumb on its track: {column:?}"
    );
}

#[test]
fn a_header_takes_an_aside_a_lead_one_line_and_words_only() {
    use sokkel_tui::IconPlace;
    let look = look();
    let h = PageHeader::new(&look, "nightly-export, run 4812 from the schedule of every day at two")
        .aside("n", "kind-blue")
        .single_line(true)
        .lead("failed")
        .words_only(true)
        .primary(Button::new(&look, "Open job").with_icon("→", IconPlace::End));
    // 80 columns: wide enough (512 px) for the actions beside the title.
    assert_eq!(h.height(80), 1, "one line, the actions beside it");
    let (_, text) = draw(80, 1, &h);
    assert!(
        text.starts_with(" n  nightly-export"),
        "the tile, then the title: {text:?}"
    );
    assert!(text.contains('…'), "cut short on one line");
    assert!(
        text.contains("● failed") && text.contains("Open job") && !text.contains('→'),
        "{text:?}"
    );
    let l = h.layout(Rect::new(0, 0, 80, 1));
    assert_eq!(l.aside, Some(Rect::new(0, 0, 3, 1)));
    assert!(l.lead.is_some_and(|r| r.x < l.primary.unwrap().x));
}
