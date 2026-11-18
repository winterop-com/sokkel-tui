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

#[test]
fn a_strip_is_its_icons_over_short_labels_and_never_resizes() {
    use sokkel_tui::{Shell, ShellState};
    let look = look();
    let strip = rail(&look).strip();
    assert_eq!(strip.width(), 9, "72 wide");
    assert_eq!(
        strip.clone().collapsed(false, true).width(),
        9,
        "it neither collapses nor expands"
    );
    let lines = draw(9, 20, strip.clone());
    let icon = lines
        .iter()
        .position(|l| l.trim() == ">" || l.contains(" > "))
        .expect("Runs' icon");
    assert!(lines[icon + 1].contains("Runs"), "its label under it: {lines:?}");
    let shell = Shell::new(&look, strip, TopBar::new(&look), StatusLine::new(&look));
    let mut state = ShellState::default();
    let window = Rect::new(0, 0, 120, 30);
    shell.render(window, &mut Buffer::empty(window), &mut state);
    assert_eq!(state.edge_area(), None, "no edge to drag");
}

#[test]
fn a_side_panel_stands_at_the_contents_end_and_covers_the_page_below_md() {
    use sokkel_tui::{Shell, ShellState};
    let look = look();
    let shell = Shell::new(&look, rail(&look), TopBar::new(&look), StatusLine::new(&look)).panel(Some(40));
    let mut state = ShellState::default();
    let wide = Rect::new(0, 0, 140, 30);
    let page = shell.render(wide, &mut Buffer::empty(wide), &mut state);
    let panel = state.panel_area().expect("a panel");
    assert_eq!(panel.right(), 140, "at the content's end");
    assert_eq!(panel.width, 39, "40 behind its edge");
    assert!(page.right() < panel.x, "the page beside it");
    let narrow = Rect::new(0, 0, 80, 24);
    let page = shell.render(narrow, &mut Buffer::empty(narrow), &mut state);
    assert_eq!(state.panel_area().unwrap().width, 80, "below md it covers the page");
    assert_eq!(page.height, 0);
}

#[test]
fn a_detail_pane_sets_its_terms_in_one_column_and_presses_its_verbs() {
    use ratatui::layout::Position;
    use sokkel_tui::{Button, DetailPane, PaneSection};
    let look = look();
    let pane = DetailPane::new(&look)
        .sections([
            PaneSection::new("Run").facts([("State", Some("failed")), ("Started by", None)]),
            PaneSection::new("Host").facts([("Host", Some("juniper.example.org"))]),
        ])
        .actions([Button::new(&look, "Retry"), Button::new(&look, "Cancel run")]);
    let area = Rect::new(0, 0, 40, pane.height(40));
    let lines = draw(40, area.height, &pane);
    let at = |w: &str| lines.iter().position(|l| l.contains(w)).unwrap();
    assert!(lines[at("RUN")].starts_with("  RUN"), "upper case, padded: {lines:?}");
    let col = |w: &str| lines[at(w)].find(w).unwrap();
    assert_eq!(
        col("failed"),
        lines[at("Started by")].find('—').unwrap(),
        "values in one column"
    );
    let l = pane.layout(area);
    let (_, cancel) = l.actions[1];
    assert_eq!(pane.press(area, Position::new(cancel.x + 1, cancel.y)), Some(1));
}

#[test]
fn a_rail_fits_its_longest_label_and_says_a_cut_one_whole() {
    use sokkel_tui::{Shell, ShellState};
    let look = look();
    let long = "Chap Core master, climate-demo-niger";
    let rail = || {
        Rail::new(&look, "C", "chap-checker").groups([RailGroup::new(
            Some("Instances"),
            [RailEntry::new(long).icon("●"), RailEntry::new("local").icon("●")],
        )])
    };
    let (min, own, max) = Rail::sizes();
    assert_eq!(rail().width(), own, "its own width unless it fits");
    let fitted = rail().fit().width();
    assert!(fitted > own && fitted <= max, "{fitted}");
    assert!(draw(fitted, 6, rail().fit()).iter().any(|l| l.contains(long)), "whole");
    assert_eq!(rail().size(Some(min)).width(), min, "the app's own size");
    // Cut short, the focused entry says the whole label in the status line.
    let shell = Shell::new(&look, rail(), TopBar::new(&look), StatusLine::new(&look));
    let mut state = ShellState::default();
    state.rail_focus = Some((0, 0));
    let window = Rect::new(0, 0, 120, 30);
    let mut buf = Buffer::empty(window);
    shell.render(window, &mut buf, &mut state);
    assert!(lines(&buf)[29].contains(long), "{:?}", lines(&buf)[29]);
}

#[test]
fn the_rails_width_is_kept_across_runs() {
    use sokkel_tui::app::{read_rail_size, write_rail_size};
    let dir = std::env::temp_dir().join(format!("sokkel-tui-rail-{}", std::process::id()));
    let look_path = dir.join("tidevann").join("appearance.json");
    assert_eq!(read_rail_size(&look_path), None);
    write_rail_size(&look_path, Some(36)).unwrap();
    assert_eq!(read_rail_size(&look_path), Some(36));
    write_rail_size(&look_path, None).unwrap();
    assert_eq!(read_rail_size(&look_path), None, "its own width again");
}

#[test]
fn a_detail_panes_long_fact_wraps_in_its_column() {
    use sokkel_tui::{DetailPane, PaneSection};
    let look = look();
    let url = "https://climate-demo.dhis2.example.org/api/routes/run";
    let pane = DetailPane::new(&look)
        .sections([PaneSection::new("Check").facts([("URL", Some(url)), ("State", Some("failed"))])]);
    let width = 40;
    let h = pane.height(width);
    let lines = draw(width, h, &pane);
    // The whole URL, over two rows in the value's column, and the next fact under it.
    let joined: String = lines.iter().map(|l| l.trim()).collect::<Vec<_>>().join("");
    assert!(joined.replace("URL", "").contains(url), "{lines:?}");
    let at = |w: &str| lines.iter().position(|l| l.contains(w)).unwrap();
    assert!(at("State") >= at("URL") + 2, "{lines:?}");
    let col = lines[at("URL")].find("https").unwrap();
    assert_eq!(
        lines[at("URL") + 1].find(|c: char| c != ' ').unwrap(),
        col,
        "in the value's column"
    );
}

#[test]
fn the_rail_gives_way_to_a_page_that_needs_the_room() {
    use ratatui::layout::Position;
    use sokkel_tui::{Pointer, PointerKind, Shell, ShellState};
    let look = look();
    let shell = Shell::new(&look, rail(&look).fit(), TopBar::new(&look), StatusLine::new(&look)).page_needs(Some(69));
    let page = |w: u16, state: &mut ShellState| {
        let window = Rect::new(0, 0, w, 30);
        shell.render(window, &mut Buffer::empty(window), state)
    };
    // Room enough: the rail as it is.
    let mut s = ShellState::default();
    assert!(page(140, &mut s).width >= 69 && !s.gave_way());
    // Short: narrower first, then its icons, the page keeping what it needs.
    let p = page(100, &mut s);
    assert!(p.width >= 69 && !s.gave_way(), "narrowed: {p:?}");
    let p = page(90, &mut s);
    assert!(p.width >= 69 && s.gave_way(), "collapsed: {p:?}");
    // Expand on a rail that gave way keeps it open, the page then as it may.
    let mark = Position::new(1, 0);
    s.pointer(Pointer {
        kind: PointerKind::Press,
        at: mark,
    });
    assert!(s.kept_open && !s.collapsed);
    page(90, &mut s);
    assert!(!s.gave_way());
    // A width the reader dragged wins too.
    let mut dragged = ShellState::default();
    dragged.rail_size = Some(Rail::sizes().2);
    page(100, &mut dragged);
    assert!(!dragged.gave_way() && dragged.edge_area().unwrap().x == Rail::sizes().2 - 1);
}

#[test]
fn a_status_reads_the_same_in_a_cell_a_header_and_a_pane() {
    use sokkel_tui::{Cell, DetailPane, PageHeader, PaneSection, Status, StatusTone};
    let look = look();
    let color = |t: &str| look.color(t);
    // Health states through the contract's map: ok good, warn warning, fail and error critical.
    assert_eq!(Status::of("ok").colors(&look).0, color("good"));
    assert_eq!(Status::of("warn").colors(&look).0, color("warning"));
    assert_eq!(Status::of("fail").colors(&look).0, color("critical"));
    assert_eq!(Status::of("error").colors(&look).0, color("critical"));
    // A run's state by its own color; one no map knows in neutral; an app's own tone wins.
    assert_eq!(Status::of("failed").colors(&look).0, color("status-failed"));
    assert_eq!(Status::of("completed-with-errors").words(), "completed-with-errors");
    assert_eq!(Status::of("unheard-of").colors(&look).0, color("faint"));
    assert_eq!(
        Status::toned("degraded", StatusTone::Warning).colors(&look).0,
        color("warning")
    );
    // The same status as a cell, a header's lead and a pane's fact: its dot in its color.
    let dot = look.glyphs().status_dot;
    let warn = Status::of("warn");
    let ink_at = |buf: &Buffer| {
        buf.content()
            .iter()
            .find(|c| c.symbol() == dot)
            .map(|c| c.fg)
            .expect("a dot")
    };
    let want = look.terminal(color("warning"));
    let header = PageHeader::new(&look, "climate-demo-niger").lead_status(warn.clone());
    let mut buf = Buffer::empty(Rect::new(0, 0, 60, 4));
    (&header).render(buf.area, &mut buf);
    assert_eq!(ink_at(&buf), want, "the header's lead");
    let pane = DetailPane::new(&look).sections([PaneSection::new("Check").status("Health", warn.clone())]);
    let mut buf = Buffer::empty(Rect::new(0, 0, 40, pane.height(40)));
    (&pane).render(buf.area, &mut buf);
    assert_eq!(ink_at(&buf), want, "the pane's fact");
    assert!(lines(&buf).iter().any(|l| l.contains("warn")));
    assert_eq!(Cell::state(warn).words(), "warn");
}
