use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Button, Density, Fact, Look, NoteTone, Rail, RailEntry, RailFoot, RailGroup, Scheme, ShellLayout, StatusLine, Tip,
    TopBar, TopBarFocus, snapshot,
};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn lines(buf: &Buffer) -> Vec<String> {
    snapshot::text(buf).lines().map(str::to_string).collect()
}

fn draw(w: u16, h: u16, widget: impl Widget) -> Vec<String> {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    widget.render(buf.area, &mut buf);
    lines(&buf)
}

fn rail(look: &Look) -> Rail<'_> {
    Rail::new(look, "T", "Tidevann")
        .groups([
            RailGroup::new(
                Some("Workspace"),
                [
                    RailEntry::new("Runs").icon(">").count("1,200").active(),
                    RailEntry::new("Hosts").icon("#").unread(),
                ],
            ),
            RailGroup::new(Some("Projects"), [RailEntry::new("harbor")]),
        ])
        .collapsed(false, true)
        .foot(RailFoot {
            letter: "a".into(),
            color: "kind-violet".into(),
            name: "ada".into(),
        })
}

#[test]
fn the_frame_puts_the_rail_beside_the_bar_page_and_foot() {
    let look = look();
    let s = ShellLayout::new(&look, Rect::new(0, 0, 120, 30), 29);
    assert_eq!(s.rail, Some(Rect::new(0, 0, 29, 30)), "the window's full height");
    assert_eq!(s.top, Rect::new(29, 0, 91, 2), "the bar and its rule at regular");
    assert_eq!(s.foot, Rect::new(29, 28, 91, 2));
    assert_eq!(s.content, Rect::new(29, 2, 91, 26));
    assert_eq!(s.width, Width::Wide);
    let page = s.page(&look);
    assert_eq!((page.x, page.y), (31, 3), "the page inset, a blank row on top");

    let narrow = ShellLayout::new(&look, Rect::new(0, 0, 80, 24), 29);
    assert_eq!(narrow.rail, None, "below md the rail is a drawer");
    assert_eq!(
        ShellLayout::drawer(Rect::new(0, 0, 30, 24), 29).width,
        26,
        "at most the window less four"
    );

    let compact = look.with_density(Density::Compact);
    let c = ShellLayout::new(&compact, Rect::new(0, 0, 120, 30), 29);
    assert_eq!((c.top.height, c.foot.height), (1, 1), "no rule rows at compact");
}

#[test]
fn the_rail_draws_its_head_groups_entries_and_foot() {
    let look = look();
    let l = draw(29, 12, rail(&look));
    assert!(
        l[0].starts_with("  T  Tidevann"),
        "a column of padding, the mark tile: {l:?}"
    );
    assert!(l[0].contains("«"), "the collapse control");
    assert!(l[1].trim().ends_with('│'), "the row level with the top bar's rule");
    assert!(l[2].starts_with(" WORKSPACE"));
    assert!(l[3].starts_with("▌> Runs"), "the active entry's bar");
    assert!(l[3].contains("1,200"));
    assert!(l[4].contains("Hosts") && l[4].contains('●'), "an unread dot");
    assert!(
        l[5].trim_matches(|c| c == ' ' || c == '│').is_empty(),
        "a blank row before a title"
    );
    assert!(l[6].starts_with(" PROJECTS"));
    assert!(
        l[10].starts_with('─'),
        "the foot cell's rule, level with the status line's"
    );
    assert!(l[11].contains(" a ") && l[11].contains("ada"));
    assert!(
        l.iter().all(|r| r.ends_with('│')),
        "the edge: a column of the track glyph"
    );
}

#[test]
fn collapsed_it_is_its_icons_and_says_a_focused_one_in_the_status_line() {
    let look = look();
    let r = rail(&look).collapsed(true, true).focused(Some((0, 1)));
    assert_eq!(r.width(), 7);
    let l = draw(7, 12, &r);
    assert!(l[0].contains(" T ") && !l[0].contains('»'), "the mark alone expands it");
    assert!(l[2].contains('─'), "a rule in a title's place");
    assert!(l[3].contains('>'));
    assert!(l[4].starts_with('▌'), "keyboard focus at the entry's start");
    assert_eq!(r.tip().as_deref(), Some("Hosts  Unread"));
    let layout = r.layout(Rect::new(0, 0, 7, 12), 0);
    assert_eq!(layout.entries.len(), 3);
    assert_eq!(
        layout.collapse,
        Some(Rect::new(0, 0, 6, 1)),
        "collapsed, the head expands it"
    );
}

#[test]
fn the_top_bar_holds_search_trail_palette_and_actions() {
    let look = look();
    let bar = TopBar::new(&look)
        .trail(["Tidevann", "Runs"])
        .window(Width::Wide)
        .search("")
        .actions([Button::new(&look, "New")]);
    let l = draw(100, 2, &bar);
    assert!(l[0].contains("⌕ Search"), "{l:?}");
    assert!(l[0].contains("Tidevann › Runs"));
    assert!(l[0].contains("Open the command palette Ctrl K"));
    assert!(l[0].trim_end().ends_with("New"));
    assert!(l[1].chars().all(|c| c == '─'), "the rule under the bar at regular");
    let layout = bar.layout(Rect::new(0, 0, 100, 1));
    assert_eq!(layout.search.unwrap().width, 30, "256 px from lg");
    assert_eq!(layout.crumbs.len(), 2);
}

#[test]
fn below_md_it_is_the_menu_the_last_crumb_and_the_search_icon() {
    let look = look();
    let bar = TopBar::new(&look)
        .trail(["Tidevann", "Runs"])
        .window(Width::Narrow)
        .search("")
        .focus(TopBarFocus::Menu);
    let l = draw(60, 1, &bar);
    assert!(l[0].starts_with("┃ ≡ ┃"), "{l:?}");
    assert!(l[0].contains("‹ Runs") && !l[0].contains("Tidevann"));
    assert!(l[0].trim_end().ends_with('⌕'));
}

#[test]
fn the_status_line_says_its_note_its_facts_and_a_focused_tip() {
    let look = look();
    let line = StatusLine::new(&look).note("Live", NoteTone::Live).live(true).facts([
        Fact::labeled("host", "api.example.org").code(),
        Fact::new("Updated 12s ago"),
    ]);
    let l = draw(80, 2, &line);
    assert!(l[0].chars().all(|c| c == '─'));
    assert!(l[1].starts_with(" ● Live"));
    assert!(
        l[1].trim_end().ends_with("host api.example.org  Updated 12s ago"),
        "{l:?}"
    );
    let tipped = line.clone().tip(Some(Tip {
        words: "Nothing to export yet".into(),
        shortcut: Some("Ctrl E".into()),
    }));
    let l = draw(80, 2, &tipped);
    assert!(
        l[1].starts_with(" Nothing to export yet  Ctrl E"),
        "the tip takes the note's place"
    );
    let l = draw(30, 2, &line);
    assert!(!l[1].contains("host"), "facts take at most 70 percent: {l:?}");
}

#[test]
fn a_parent_folds_its_children_and_an_entry_says_its_description() {
    let look = look();
    let rail = |open: bool| {
        Rail::new(&look, "T", "Tidevann").groups([RailGroup::new(
            None,
            [
                RailEntry::new("Schedules").description("Every job's next run"),
                RailEntry::new("Hosts").parent(open),
                RailEntry::new("api.example.org").child(),
            ],
        )])
    };
    let shut = rail(false).layout(Rect::new(0, 0, 29, 20), 0);
    assert_eq!(shut.entries.len(), 2, "the shut parent's child left out");
    assert_eq!(shut.entries[0].1.height, 2, "an entry and its description");
    let open = rail(true).layout(Rect::new(0, 0, 29, 20), 0);
    assert_eq!(open.entries.len(), 3);
    assert_eq!(open.entries[2].0, (0, 2));
    let l = draw(29, 12, rail(true));
    assert!(l.iter().any(|r| r.contains("Every job's next run")));
    assert!(
        l.iter().any(|r| r.starts_with("   api.example.org")),
        "under the parent's label: {l:?}"
    );
    assert!(l.iter().any(|r| r.contains("Hosts") && r.contains('▾')));
    let collapsed = rail(true).collapsed(true, true).layout(Rect::new(0, 0, 7, 20), 0);
    assert_eq!(collapsed.entries.len(), 2, "collapsed, children wait for their flyout");
}

#[test]
fn a_rail_entry_shows_its_status_dot_and_says_its_word_when_focused() {
    use sokkel_tui::StatusTone;
    let look = look();
    let rail = Rail::new(&look, "T", "Tidevann")
        .groups([RailGroup::new(
            None,
            [RailEntry::new("juniper").status(StatusTone::Critical, "fail")],
        )])
        .focused(Some((0, 0)));
    let mut buf = Buffer::empty(Rect::new(0, 0, 29, 8));
    (&rail).render(buf.area, &mut buf);
    let l = lines(&buf);
    let y = l.iter().position(|r| r.contains("juniper")).unwrap();
    let x = l[y].rfind('●').unwrap();
    let x = l[y][..x].chars().count() as u16;
    assert_eq!(buf[(x, y as u16)].fg, look.token("critical"));
    assert_eq!(rail.tip().as_deref(), Some("juniper  fail"));
}

#[test]
fn the_rails_edge_resizes_it_by_a_drag_and_by_keys() {
    use ratatui::layout::Position;
    use sokkel_tui::{Key, KeyPress, Pointer, PointerKind, Shell, ShellState};
    let look = look();
    let shell = Shell::new(&look, rail(&look), TopBar::new(&look), StatusLine::new(&look));
    let mut state = ShellState::default();
    let window = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(window);
    shell.render(window, &mut buf, &mut state);
    let (min, own, max) = Rail::sizes();
    let edge = state.edge_area().expect("an expanded rail beside the page");
    assert_eq!(edge.x, own - 1, "the rail's last column");
    let at = |kind, x| Pointer {
        kind,
        at: Position::new(x, 10),
    };
    state.pointer(at(PointerKind::Press, edge.x));
    state.pointer(at(PointerKind::Drag, edge.x + 6));
    assert_eq!(state.rail_size, Some(own + 6), "the edge follows the pointer");
    state.pointer(at(PointerKind::Drag, 2));
    assert_eq!(state.rail_size, Some(min), "never narrower than minSize");
    state.pointer(at(PointerKind::Release, 2));
    state.pointer(at(PointerKind::Drag, 60));
    assert_eq!(state.rail_size, Some(min), "let go, a drag moves nothing");
    shell.render(window, &mut buf, &mut state);
    assert_eq!(state.edge_area().unwrap().x, min - 1, "drawn at its new width");
    assert!(state.edge_key(KeyPress::new(Key::End)));
    assert_eq!(state.rail_size, Some(max));
    assert!(state.edge_key(KeyPress::new(Key::Left)));
    assert_eq!(state.rail_size, Some(max - 2), "16 px a press");
    assert!(state.edge_key(KeyPress::new(Key::Enter)));
    assert_eq!(state.rail_size, None, "Enter puts back its own width");
}
