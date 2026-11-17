//! 09: a dashboard in sokkel's frame. Tidevann's overview: stat tiles (a reading inked or filled
//! by its threshold, a change against yesterday, a sparkline, a meter toward a target), and panels
//! in their states: hosts as a table, a panel still loading, one that failed with Retry, one with
//! nothing for its range, and one not up to date. The readings move while it runs. Narrow the
//! terminal to see the spans give way.
//!
//! Usage: cargo run --example 09_dashboard [-- light] [-- --frame <cols> <rows> <out> [keys...]]
//! Keys: the runtime's (Ctrl+K, `,`, `?`, Tab); `r` retries the failed panel; the wheel, PageUp and
//! PageDown scroll the page. Ctrl+Q quits.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use sokkel_tui::app::{self, Chrome, Cx, Flow, Ran, ShellApp};
use sokkel_tui::dashboard::grid;
use sokkel_tui::{
    Cell, ColorMode, Column, ColumnWidth, Command, CommandGroup, DashboardPanel, Fact, Fill, Key, KeyPress, Legend,
    Look, NoteTone, PageHeader, PanelRead, PanelTool, Pointer, PointerKind, Rail, RailEntry, RailFoot, RailGroup, Row,
    RowTone, Scheme, Series, Span, StatPanel, StatusLine, Table, TableState, Threshold, TimeSeriesPanel, TopBar,
    snapshot,
};

/// The success rate's scale: critical below 95, warning below 99, good from 99.
fn success_scale() -> Vec<Threshold> {
    vec![
        Threshold::new(None, "critical"),
        Threshold::new(Some(95.0), "warning"),
        Threshold::new(Some(99.0), "good"),
    ]
}

struct Overview {
    now: u64,
    /// When the failed panel's retry started, while it reads again.
    retrying: Option<u64>,
    hosts: TableState,
    /// Where the failed panel stands, for a press on its Retry.
    failed_at: Rect,
}

impl Overview {
    /// A reading that moves with time: `base` plus a slow wave.
    fn wave(&self, base: f64, swing: f64, phase: f64) -> f64 {
        base + swing * ((self.now as f64 / 1500.0) + phase).sin()
    }

    fn header<'a>(&self, look: &'a Look) -> PageHeader<'a> {
        PageHeader::new(look, "Overview").description("Every job's runs today, and the hosts they run on.")
    }

    fn tiles<'a>(&self, look: &'a Look) -> Vec<StatPanel<'a>> {
        let success = self.wave(98.7, 1.2, 0.0);
        let latency = self.wave(412.0, 60.0, 1.0);
        vec![
            StatPanel::new(look, "Runs today", Some(self.wave(1284.0, 20.0, 2.0).round()))
                .delta(6.2, "vs yesterday", true)
                .sparkline(self.trend(1200.0, 80.0)),
            StatPanel::new(look, "Success rate", Some(success))
                .unit("%")
                .decimals(1)
                .thresholds(success_scale())
                .target(99.5, Some((90.0, 100.0))),
            StatPanel::new(look, "Failed", Some(3.0))
                .color("critical")
                .color_mode(ColorMode::Background)
                .note("sync-inventory twice"),
            StatPanel::new(look, "p95 latency", Some(latency.round()))
                .unit("ms")
                .delta(-4.0, "vs yesterday", false)
                .sparkline(self.trend(410.0, 50.0)),
            StatPanel::new(look, "Queued", Some(12.0)).note("send-digest waits on harbor"),
            StatPanel::new(look, "Disk on harbor", None).stale("since 11:58"),
        ]
    }

    /// The tiles, then the panels under them, by their spans and rows.
    fn spans(&self, look: &Look) -> Vec<(Span, u16)> {
        let tiles = self.tiles(look);
        let th = tiles.iter().map(StatPanel::height).max().unwrap_or(6);
        let mut spans: Vec<(Span, u16)> = tiles.iter().map(|_| (Span::Sixth, th)).collect();
        spans.extend([
            (Span::TwoThirds, 12),
            (Span::Third, 12),
            (Span::Full, 16),
            (Span::Third, 8),
            (Span::Third, 8),
            (Span::Third, 8),
            (Span::Third, 8),
        ]);
        spans
    }

    fn trend(&self, base: f64, swing: f64) -> Vec<f64> {
        (0..24)
            .map(|k| {
                base + swing * ((k as f64 / 3.0) + self.now as f64 / 4000.0).sin()
                    + (k as f64 * 0.7).cos() * swing / 3.0
            })
            .collect()
    }
}

impl ShellApp for Overview {
    fn id(&self) -> Option<&str> {
        Some("tidevann")
    }

    fn route(&self) -> String {
        "overview".into()
    }

    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a> {
        Chrome {
            rail: Rail::new(look, "T", "Tidevann")
                .groups([
                    RailGroup::new(
                        Some("Workspace"),
                        [
                            RailEntry::new("Overview").icon("◫").active(),
                            RailEntry::new("Runs").icon("▶").count("1,284"),
                            RailEntry::new("Hosts").icon("▣").unread(),
                        ],
                    ),
                    RailGroup::new(
                        Some("Projects"),
                        ["harbor", "juniper", "larkspur"].map(|p| RailEntry::new(p).icon("●")),
                    ),
                ])
                .foot(RailFoot {
                    letter: "a".into(),
                    color: "kind-violet".into(),
                    name: "ada".into(),
                }),
            top: TopBar::new(look).trail(["Tidevann", "Overview"]),
            foot: StatusLine::new(look).note("Live", NoteTone::Live).live(true).facts([
                Fact::labeled("host", "api.example.org").code(),
                Fact::new("Updated just now"),
            ]),
        }
    }

    fn page(&mut self, f: &mut Frame, page: Rect, cx: &mut Cx) {
        let look = cx.look;
        let header = self.header(look);
        let hh = header.height(page.width);
        f.render_widget(&header, Rect { height: hh, ..page });
        let area = Rect {
            y: page.y + hh + 1,
            height: page.height.saturating_sub(hh + 1),
            ..page
        };

        let tiles = self.tiles(look);
        let rects = grid(look, area, &self.spans(look));
        for (tile, r) in tiles.iter().zip(&rects) {
            tile.render(r.intersection(area), f.buffer_mut());
        }

        // Hosts, as a table in a flush panel.
        let mut it = rects[tiles.len()..].iter();
        if let Some(r) = it.next() {
            let p = DashboardPanel::new(look, "Hosts")
                .subtitle("last hour")
                .flush(true)
                .expandable(true)
                .menu(true);
            (&p).render(r.intersection(area), f.buffer_mut());
            let body = p.layout(*r).body;
            let columns = vec![
                Column::new("Host").width(ColumnWidth::Fill(2)),
                Column::new("State"),
                Column::new("Runs"),
                Column::new("p95"),
            ];
            let host = |name: &str, state: &str, runs: &str, p95: &str| {
                Row::new([Cell::text(name), Cell::status(state), Cell::text(runs), Cell::text(p95)])
            };
            let rows = vec![
                host("api.example.org", "succeeded", "642", "388 ms"),
                host("harbor.example.org", "running", "401", "430 ms"),
                host("juniper.example.org", "failed", "212", "—").tone(RowTone::Critical),
                host("larkspur.example.org", "succeeded", "29", "301 ms"),
            ];
            let table = Table::new(look, &columns, &rows).framed(false);
            f.render_stateful_widget(&table, body.intersection(area), &mut self.hosts);
        }
        // Latency over the day, its area under the scale's steps; runs per hour, two series.
        let hours: Vec<String> = (0..24).map(|h| format!("{h:02}")).collect();
        let latency: Vec<f64> = (0..24)
            .map(|h| self.wave(380.0 + 8.0 * h as f64, 40.0, h as f64 / 2.0))
            .collect();
        let series = TimeSeriesPanel::new(
            DashboardPanel::new(look, "p95 latency").subtitle("today"),
            hours.clone(),
        )
        .series([Series::new("p95", latency)])
        .thresholds([
            Threshold::new(None, "good"),
            Threshold::new(Some(450.0), "warning"),
            Threshold::new(Some(550.0), "critical"),
        ])
        .unit("ms")
        .legend(Legend::None);
        if let Some(r) = it.next() {
            (&series).render(r.intersection(area), f.buffer_mut());
        }
        let ok: Vec<f64> = (0..24)
            .map(|h| self.wave(50.0 + 2.0 * h as f64, 12.0, h as f64 / 3.0).round())
            .collect();
        let bad: Vec<f64> = (0..24)
            .map(|h| (self.wave(4.0, 3.0, h as f64).max(0.0)).round())
            .collect();
        let runs = TimeSeriesPanel::new(
            DashboardPanel::new(look, "Runs per hour")
                .subtitle("today")
                .expandable(true),
            hours,
        )
        .series([
            Series::new("succeeded", ok).color("good"),
            Series::new("failed", bad).color("critical"),
            Series::new("yesterday", (0..24).map(|h| 48.0 + 2.0 * h as f64)).dashed(),
        ])
        .fill(Fill::None)
        .legend(Legend::Table);
        if let Some(r) = it.next() {
            (&runs).render(r.intersection(area), f.buffer_mut());
        }
        let failed = match self.retrying {
            Some(t) if cx.now.saturating_sub(t) < 2500 => PanelRead::Loading(cx.now.saturating_sub(t)),
            Some(_) => PanelRead::Empty,
            None => PanelRead::Failed(Some("harbor.example.org answered 502".into())),
        };
        let panels = [
            DashboardPanel::new(look, "Queue by job")
                .subtitle("now")
                .read(PanelRead::Loading(cx.now + 300)),
            DashboardPanel::new(look, "Runs by project")
                .subtitle("today")
                .read(failed),
            DashboardPanel::new(look, "Cancelled runs")
                .subtitle("this week")
                .read(PanelRead::Empty),
            DashboardPanel::new(look, "Schedules")
                .subtitle("next 24 hours")
                .stale("since 11:58")
                .foot("5 schedules"),
        ];
        for (k, (p, r)) in panels.iter().zip(it).enumerate() {
            p.render(r.intersection(area), f.buffer_mut());
            if k == 1 {
                self.failed_at = *r;
            }
        }
    }

    fn page_rows(&self, look: &Look, width: u16) -> Option<u16> {
        let hh = self.header(look).height(width);
        let area = Rect::new(0, hh + 1, width, 0);
        grid(look, area, &self.spans(look)).iter().map(|r| r.bottom()).max()
    }

    fn commands(&self) -> Vec<CommandGroup> {
        vec![CommandGroup::new(
            "Overview",
            [Command::new("retry", "Retry the failed panel").icon("↻")],
        )]
    }

    fn command(&mut self, id: &str) -> Ran {
        if id == "retry" {
            self.retrying = Some(self.now);
        }
        Ran::Done
    }

    fn key(&mut self, key: KeyPress, _cx: &mut Cx) -> Flow {
        if key.key == Key::Char('r') {
            self.command("retry");
            return Flow::Handled;
        }
        Flow::Ignored
    }

    fn pointer(&mut self, p: Pointer, cx: &mut Cx) {
        let panel = DashboardPanel::new(cx.look, "").read(PanelRead::Failed(None));
        if p.kind == PointerKind::Press && panel.hit(self.failed_at, p.at) == Some(PanelTool::Retry) {
            self.command("retry");
        }
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
    }
}

fn overview() -> Overview {
    Overview {
        now: 0,
        retrying: None,
        hosts: TableState::default(),
        failed_at: Rect::default(),
    }
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(at) = args.iter().position(|a| a == "--frame") {
        let scheme = if args.iter().any(|a| a == "light") {
            Scheme::Light
        } else {
            Scheme::Dark
        };
        let look = Look::new("tidevann", scheme).expect("tidevann");
        let size = (
            args[at + 1].parse().expect("columns"),
            args[at + 2].parse().expect("rows"),
        );
        let keys: Vec<KeyPress> = args[at + 4..].iter().filter_map(|k| KeyPress::named(k)).collect();
        let buf = app::shot(overview(), look.clone(), size, |_| {}, &keys);
        let out = &args[at + 3];
        std::fs::write(
            format!("{out}.svg"),
            snapshot::svg(&buf, look.token("background"), look.token("foreground")),
        )?;
        std::fs::write(format!("{out}.txt"), snapshot::text(&buf))?;
        return Ok(());
    }
    app::run_shell(overview(), app::look_from_args("tidevann"))
}
