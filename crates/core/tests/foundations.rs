use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use sokkel_tui::cells::{Width, cols};
use sokkel_tui::look::{Rgba, nearest_256};
use sokkel_tui::{Depth, Look, Scheme, TextRole, snapshot, tokens};

#[test]
fn every_theme_in_both_schemes_has_a_look() {
    let themes = tokens::themes();
    assert_eq!(themes.len(), 13);
    for theme in themes {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let look = Look::new(theme, scheme).expect("a look");
            assert!(matches!(look.token("foreground"), Color::Rgb(..)));
        }
    }
    assert!(Look::new("no-such-theme", Scheme::Dark).is_none());
}

#[test]
fn every_paint_draws_opaque_in_every_theme() {
    let all = tokens::resolved();
    for theme in tokens::themes() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let look = Look::new(theme, scheme).unwrap();
            for (key, paint) in all["paints"][theme][scheme.key()].as_object().unwrap() {
                for field in ["ground", "edge", "ink", "rule", "mark", "placeholder"] {
                    if paint
                        .get(field)
                        .and_then(|v| v.as_str())
                        .is_some_and(|v| v.starts_with('#'))
                    {
                        let c = look.paint(key, field);
                        assert!(c.is_some_and(Rgba::is_opaque), "{theme} {key} {field}");
                    }
                }
            }
        }
    }
}

#[test]
fn a_state_the_contract_does_not_paint_is_drawn_as_rest() {
    let look = Look::new("tidevann", Scheme::Dark).unwrap();
    assert_eq!(
        look.paint_hex("table.base.pressed.row", "ink"),
        look.paint_hex("table.base.rest.row", "ink")
    );
}

#[test]
fn a_blended_paint_is_read_in_its_flat_form() {
    // tidevann light: the default button's hover ground is primary/80; the contract flattens it.
    let look = Look::new("tidevann", Scheme::Light).unwrap();
    assert_eq!(look.paint_hex("button.default.hover.container", "ground"), "#006aa0cc");
    assert_eq!(
        look.paint("button.default.hover.container", "ground"),
        Rgba::parse("#3287b2")
    );
    // An inherited ink has its flat form too.
    assert_eq!(look.paint_hex("button.outline.rest.container", "ink"), "inherit");
    assert!(look.paint("button.outline.rest.container", "ink").is_some());
}

#[test]
fn a_disabled_paint_is_read_in_its_faded_form() {
    let look = Look::new("tidevann", Scheme::Light).unwrap();
    assert_eq!(
        look.faded("button.default.rest.container", "ground"),
        Rgba::parse("#7db2ce")
    );
    assert!(look.faded("text-field.base.disabled.control", "ink").is_some());
    assert!(look.faded("text-field.base.rest.label", "ink").is_some());
}

#[test]
fn a_transparent_edge_does_not_show() {
    let look = Look::new("tidevann", Scheme::Dark).unwrap();
    assert!(!look.shows("button.default.rest.container", "edge"));
    assert!(look.shows("button.outline.rest.container", "edge"));
}

#[test]
fn ansi256_is_the_nearest_of_16_to_255_by_oklab() {
    assert_eq!(nearest_256(Rgba(0, 0, 0, 255)), 16);
    assert_eq!(nearest_256(Rgba(255, 255, 255, 255)), 231);
    assert_eq!(nearest_256(Rgba(255, 0, 0, 255)), 196);
    assert_eq!(nearest_256(Rgba(128, 128, 128, 255)), 244);
}

#[test]
fn fewer_colors_and_none() {
    let look = Look::new("tidevann", Scheme::Dark).unwrap().with_depth(Depth::Ansi256);
    assert!(matches!(look.token("primary"), Color::Indexed(16..=255)));
    let look = look.with_depth(Depth::None);
    assert_eq!(look.token("primary"), Color::Reset);
}

#[test]
fn corners_follow_the_radius_ladder() {
    assert_eq!(
        Look::new("tidevann", Scheme::Dark).unwrap().corners("radius-surface"),
        BorderType::Rounded
    );
    assert_eq!(
        Look::new("terminal", Scheme::Dark).unwrap().corners("radius-surface"),
        BorderType::Plain
    );
}

#[test]
fn text_roles_keep_their_weight() {
    let look = Look::new("tidevann", Scheme::Light).unwrap();
    assert!(
        TextRole::PageTitle
            .style(&look)
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert!(
        !TextRole::Body
            .style(&look)
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
}

#[test]
fn widths_and_columns() {
    assert_eq!(Width::of(Rect::new(0, 0, 80, 24)), Width::Narrow);
    assert_eq!(Width::of(Rect::new(0, 0, 100, 30)), Width::Medium);
    assert_eq!(Width::of(Rect::new(0, 0, 120, 40)), Width::Wide);
    assert_eq!(cols(0.0), 0);
    assert_eq!(cols(2.0), 1);
    assert_eq!(cols(32.0), 4);
}

#[test]
fn a_snapshot_is_what_the_buffer_holds() {
    let mut term = Terminal::new(TestBackend::new(12, 3)).unwrap();
    term.draw(|f| {
        f.render_widget(
            Paragraph::new("Runs").block(Block::new().borders(Borders::ALL)),
            f.area(),
        );
    })
    .unwrap();
    let text = snapshot::text(term.backend().buffer());
    assert_eq!(text, "┌──────────┐\n│Runs      │\n└──────────┘\n");
    let svg = snapshot::svg(term.backend().buffer(), Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255));
    assert!(svg.starts_with("<svg") && svg.contains(">R</text>"));
}

#[test]
fn text_roles_and_gaps_are_the_contracts_cells() {
    let look = Look::new("tidevann", Scheme::Dark).unwrap();
    assert!(
        TextRole::Head
            .style(&look)
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert!(
        !TextRole::Help
            .style(&look)
            .add_modifier
            .contains(ratatui::style::Modifier::BOLD)
    );
    assert_eq!(sokkel_tui::cells::contract_px_per_col(), sokkel_tui::cells::PX_PER_COL);
    assert_eq!(sokkel_tui::cells::gap_rows(&look, 12.0), 1);
    assert_eq!(sokkel_tui::cells::gap_rows(&look, 6.0), 0);
    let compact = look.with_density(sokkel_tui::Density::Compact);
    assert_eq!(sokkel_tui::cells::gap_rows(&compact, 24.0), 0);
}

#[test]
fn the_cells_constants_are_the_contracts() {
    let cells = &tokens::index()["cells"];
    assert_eq!(cells["breakpoints"]["md"]["columns"], sokkel_tui::cells::MD_COLS);
    assert_eq!(cells["breakpoints"]["lg"]["columns"], sokkel_tui::cells::LG_COLS);
    let sizes: Vec<(u16, u16)> = cells["checkSizes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["columns"].as_u64().unwrap() as u16,
                s["rows"].as_u64().unwrap() as u16,
            )
        })
        .collect();
    assert_eq!(sizes, sokkel_tui::cells::CHECK_SIZES);
}

#[test]
fn sixteen_colors_go_by_role_and_hover_draws_in_reverse() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier};
    use sokkel_tui::Depth;
    let look = Look::new("tidevann", Scheme::Dark).unwrap().with_depth(Depth::Ansi16);
    assert_eq!(look.token("critical-ink"), Color::Indexed(1), "critical: red");
    assert_eq!(look.token("warning-ink"), Color::Indexed(3), "warning: yellow");
    assert_eq!(look.token("good-ink"), Color::Indexed(2), "good: green");
    assert_eq!(look.token("muted-foreground"), Color::Indexed(8), "quiet: bright black");
    assert_eq!(look.token("background"), Color::Reset, "the terminal's own ground");
    let hover = look.paint("dashboard-panel.base.hover.tool", "ground").unwrap();
    let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
    buf[(0, 0)].set_bg(look.terminal(hover)).set_symbol("x");
    look.finish(&mut buf);
    assert!(
        buf[(0, 0)].modifier.contains(Modifier::REVERSED),
        "a hover ground in reverse video"
    );
    assert_eq!(buf[(0, 0)].bg, Color::Reset);
}

#[test]
fn a_scrim_flattens_what_lies_under_a_layer() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::Color;
    use sokkel_tui::Depth;
    let look = Look::new("tidevann", Scheme::Light).unwrap();
    let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
    buf[(0, 0)]
        .set_bg(look.token("background"))
        .set_fg(look.token("foreground"));
    let area = buf.area;
    look.scrim(&mut buf, area);
    assert_ne!(
        buf[(0, 0)].bg,
        look.token("background"),
        "the ground laid under the backdrop"
    );
    let sixteen = look.clone().with_depth(Depth::Ansi16);
    let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
    let area = buf.area;
    sixteen.scrim(&mut buf, area);
    assert_eq!((buf[(0, 0)].fg, buf[(0, 0)].bg), (Color::Indexed(8), Color::Reset));
}
