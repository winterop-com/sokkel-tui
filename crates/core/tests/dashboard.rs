//! The dashboard: the grid's spans, thresholds, values, the sparkline, the stat tile, the panel.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use sokkel_tui::dashboard::{format_value, grid, sparkline, threshold_color};
use sokkel_tui::{ColorMode, DashboardPanel, Look, PanelRead, Scheme, Span, StatPanel, Threshold, snapshot};

fn look() -> Look {
    Look::new("tidevann", Scheme::Dark).unwrap()
}

fn draw(w: u16, h: u16, widget: impl Widget) -> String {
    let mut buf = Buffer::empty(Rect::new(0, 0, w, h));
    widget.render(buf.area, &mut buf);
    snapshot::text(&buf)
}

#[test]
fn spans_read_the_grids_own_width() {
    assert_eq!(Span::Sixth.columns(130), 2, "wide: as named");
    assert_eq!(Span::Sixth.columns(90), 4, "tablet-wide: a sixth takes a third");
    assert_eq!(Span::Half.columns(90), 12, "tablet-wide: the rest the row");
    assert_eq!(Span::Quarter.columns(60), 6, "phone-wide: a quarter half");
    let look = look();
    let rects = grid(
        &look,
        Rect::new(0, 0, 130, 40),
        &[(Span::Half, 5), (Span::Half, 7), (Span::Full, 3)],
    );
    // 130 columns less the column between: 64 and 65, the twelfths sharing the odd one.
    assert_eq!(rects[0], Rect::new(0, 0, 64, 5));
    assert_eq!((rects[1].x, rects[1].width), (65, 65), "a column apart");
    assert_eq!(rects[2].y, 8, "the next row under the tallest, a row apart");
}

#[test]
fn a_threshold_scale_colors_a_reading_and_values_read_as_en_us() {
    let scale = [
        Threshold::new(None, "critical"),
        Threshold::new(Some(95.0), "warning"),
        Threshold::new(Some(99.0), "good"),
    ];
    assert_eq!(threshold_color(&scale, 80.0), Some("critical"));
    assert_eq!(threshold_color(&scale, 97.0), Some("warning"));
    assert_eq!(threshold_color(&scale, 99.0), Some("good"));
    assert_eq!(format_value(3526.0, 0), "3,526");
    assert_eq!(format_value(99.34, 1), "99.3");
    assert_eq!(format_value(-1234567.5, 1), "-1,234,567.5");
    let look = look();
    assert_eq!(
        sparkline(&look, &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0], 8),
        "▁▂▃▄▅▆▇█"
    );
}

#[test]
fn a_stat_tile_says_its_value_unit_change_meter_and_no_data() {
    let look = look();
    let tile = StatPanel::new(&look, "Success rate", Some(98.7))
        .unit("%")
        .decimals(1)
        .delta(0.4, "vs yesterday", true)
        .target(99.5, Some((90.0, 100.0)));
    let text = draw(40, tile.height(), &tile);
    assert!(
        text.contains("Success rate") && text.contains("↑ 0.4 vs yesterday"),
        "{text}"
    );
    assert!(text.contains("98.7 %"));
    assert!(text.contains("Target 99.5") && text.contains('━'));
    let none = StatPanel::new(&look, "Disk on harbor", None).stale("since 11:58");
    let text = draw(30, none.height(), &none);
    assert!(text.contains("No data") && text.contains("◷ since 11:58"));
    let filled = StatPanel::new(&look, "Failed", Some(3.0))
        .color("critical")
        .color_mode(ColorMode::Background);
    let mut buf = Buffer::empty(Rect::new(0, 0, 20, 5));
    (&filled).render(buf.area, &mut buf);
    assert_eq!(
        buf[(2, 1)].bg,
        look.token("critical"),
        "the tile in the reading's color"
    );
}

#[test]
fn a_panel_keeps_its_states_apart() {
    let look = look();
    let text = |read: PanelRead| {
        draw(
            50,
            9,
            &DashboardPanel::new(&look, "Runs by project")
                .subtitle("today")
                .read(read),
        )
    };
    assert!(text(PanelRead::Empty).contains("No data for this range."));
    let failed = text(PanelRead::Failed(Some("harbor.example.org answered 502".into())));
    assert!(failed.contains("This panel could not load.") && failed.contains("Retry"));
    assert!(text(PanelRead::Loading(400)).contains("Loading"));
    assert!(!text(PanelRead::Loading(100)).contains("Loading"), "not before 300 ms");
    let stale = draw(50, 9, &DashboardPanel::new(&look, "Schedules").stale("since 11:58"));
    assert!(stale.contains("◷ since 11:58"));
}

#[test]
fn a_time_series_draws_its_lines_axis_labels_and_legend() {
    use sokkel_tui::{Legend, Series, TimeSeriesPanel};
    let look = look();
    let hours: Vec<String> = (0..12).map(|h| format!("{h:02}")).collect();
    let p = TimeSeriesPanel::new(DashboardPanel::new(&look, "Runs per hour"), hours)
        .series([
            Series::new("succeeded", (0..12).map(|h| 50.0 + h as f64)),
            Series::new("yesterday", (0..12).map(|h| 48.0 + h as f64)).dashed(),
        ])
        .legend(Legend::Table);
    let text = draw(70, 16, &p);
    assert!(
        text.chars().any(|c| ('\u{2801}'..='\u{28ff}').contains(&c)),
        "lines in braille: {text}"
    );
    assert!(text.contains("00") && text.contains("min") && text.contains("current"));
    assert!(text.contains("succeeded") && text.contains("61"), "the current figure");
    // One series with an area: block columns.
    let area = TimeSeriesPanel::new(DashboardPanel::new(&look, "p95"), ["a", "b", "c"])
        .series([Series::new("p95", [1.0, 2.0, 3.0])])
        .legend(Legend::None);
    assert!(draw(40, 12, &area).contains('█'));
}

#[test]
fn an_expanded_panel_takes_the_window_and_clears_what_stood_there() {
    let window = Rect::new(0, 0, 120, 30);
    assert_eq!(DashboardPanel::expanded_area(window), Rect::new(2, 1, 116, 28));
    assert_eq!(
        DashboardPanel::expanded_area(Rect::new(0, 0, 60, 20)),
        Rect::new(0, 0, 60, 20),
        "a sheet below md"
    );
    let look = look();
    let mut buf = Buffer::empty(Rect::new(0, 0, 30, 6));
    buf.set_string(3, 3, "leftover", ratatui::style::Style::new());
    (&DashboardPanel::new(&look, "Runs")).render(buf.area, &mut buf);
    assert!(!snapshot::text(&buf).contains("leftover"));
}
