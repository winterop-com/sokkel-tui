use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use sokkel_tui::cells::{Width, cols};
use sokkel_tui::look::{Rgba, flat};
use sokkel_tui::{Depth, Look, Scheme, TextRole, snapshot, tokens};

#[test]
fn every_theme_in_both_schemes_has_a_look() {
    let themes = tokens::themes();
    assert_eq!(themes.len(), 13);
    for theme in themes {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let look = Look::new(theme, scheme).expect("a look");
            assert!(matches!(look.color("foreground", "background"), Color::Rgb(..)));
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
                        assert!(
                            matches!(look.paint(key, field, "background"), Color::Rgb(..)),
                            "{theme} {key} {field}"
                        );
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
fn flattening_lays_alpha_over_the_surface() {
    // tidevann light: primary/80 over background.
    let over = flat(Rgba::parse("#006aa0cc").unwrap(), Rgba::parse("#f9fafb").unwrap());
    assert_eq!(over, Rgba(50, 135, 178, 255));
}

#[test]
fn fewer_colors_and_none() {
    let look = Look::new("tidevann", Scheme::Dark).unwrap().with_depth(Depth::Ansi256);
    assert!(matches!(look.color("primary", "background"), Color::Indexed(16..=255)));
    let look = look.with_depth(Depth::None);
    assert_eq!(look.color("primary", "background"), Color::Reset);
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
