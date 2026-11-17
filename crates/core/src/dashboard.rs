//! The dashboard, from sokkel-design `parts/dashboard-panel` and `parts/stat-panel`: a grid of
//! panels by their spans, the panel frame every panel shares, and the stat tile.
//!
//! The parts carry no `cells` note yet; this is sokkel-tui's reading (its proposal, c4):
//! - The grid is twelve columns of its own width, panels a column apart and rows a row apart (none
//!   at compact). A span reads the grid's width, never the window's: wide from 1024 px (122
//!   columns), tablet-wide from 640 px (76 columns), phone-wide below.
//! - A panel is a box on its ground in its edge, its head a row (the title, the subtitle a column
//!   after it, the stale mark, the tools at its end) over a rule, its body padded two columns across
//!   and a row down at regular (one and none at compact), its foot a rule and a row.
//! - A stat tile is a box: its head (the title, the change at its end), the value in the statistic
//!   role with its unit a column after it, a note, a sparkline across its foot rows in the eight
//!   levels, and a meter toward a target (a track of rules, the tick at the target, Target and its
//!   figure under it). Text is one size, so "large" is the statistic role's weight.
//! - Loading draws the spinner and the word where the content goes: a cell has no half strength.

use crate::button::{Button, ButtonSize, ButtonVariant};
use crate::cells::{by_density, cols};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Widget};

/// The share of the grid's twelve columns a panel takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Span {
    Sixth,
    Quarter,
    #[default]
    Third,
    Half,
    TwoThirds,
    Full,
}

impl Span {
    /// Its columns of twelve in a grid `width` columns wide.
    pub fn columns(self, width: u16) -> u16 {
        let named = match self {
            Span::Sixth => 2,
            Span::Quarter => 3,
            Span::Third => 4,
            Span::Half => 6,
            Span::TwoThirds => 8,
            Span::Full => 12,
        };
        if width >= cols(1024.0) {
            named
        } else if width >= cols(640.0) {
            // Tablet-wide: a sixth takes a third, a quarter half, the rest the row.
            match self {
                Span::Sixth => 4,
                Span::Quarter => 6,
                _ => 12,
            }
        } else {
            // Phone-wide: every span the row but a sixth and a quarter, which take half.
            match self {
                Span::Sixth | Span::Quarter => 6,
                _ => 12,
            }
        }
    }
}

/// Lay panels down a grid `area.width` columns wide: each `(span, rows)` in order, a row of the
/// grid as tall as its tallest panel, panels a column apart and rows a row apart (none at compact).
pub fn grid(look: &Look, area: Rect, panels: &[(Span, u16)]) -> Vec<Rect> {
    let gap_x = 1u16;
    let gap_y = by_density(look, 1, 0);
    let w = area.width;
    // A twelfth's edge, so the columns share the room left by the gaps evenly.
    let edge = |k: u16| ((w + gap_x) as u32 * k as u32 / 12) as u16;
    let mut out = Vec::with_capacity(panels.len());
    let (mut col, mut y, mut row_h) = (0u16, area.y, 0u16);
    for &(span, rows) in panels {
        let n = span.columns(w);
        if col + n > 12 {
            y += row_h + gap_y;
            (col, row_h) = (0, 0);
        }
        let x = area.x + edge(col);
        let width = edge(col + n).saturating_sub(edge(col) + gap_x);
        out.push(Rect::new(x, y, width, rows));
        row_h = row_h.max(rows);
        col += n;
    }
    out
}

/// One step of a threshold scale: from `value` up, a reading takes `color` (a color token); the
/// first step covers everything below the second.
#[derive(Debug, Clone, PartialEq)]
pub struct Threshold {
    pub value: Option<f64>,
    pub color: String,
}

impl Threshold {
    pub fn new(value: Option<f64>, color: impl Into<String>) -> Threshold {
        Threshold {
            value,
            color: color.into(),
        }
    }
}

/// The color token a reading takes on a threshold scale.
pub fn threshold_color(scale: &[Threshold], value: f64) -> Option<&str> {
    let mut color = scale.first().map(|t| t.color.as_str());
    for t in scale.iter().skip(1) {
        if t.value.is_some_and(|v| value >= v) {
            color = Some(&t.color);
        }
    }
    color
}

/// A value written as en-US writes it: thousands apart by commas, `decimals` after the point.
pub fn format_value(value: f64, decimals: usize) -> String {
    let s = format!("{:.*}", decimals, value.abs());
    let (int, frac) = s.split_once('.').map_or((s.as_str(), None), |(i, f)| (i, Some(f)));
    let mut grouped = String::new();
    for (k, c) in int.chars().enumerate() {
        if k > 0 && (int.len() - k) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let sign = if value < 0.0 && s.chars().any(|c| c != '0' && c != '.') {
        "-"
    } else {
        ""
    };
    match frac {
        Some(f) => format!("{sign}{grouped}.{f}"),
        None => format!("{sign}{grouped}"),
    }
}

/// A sparkline of `values` in `width` cells: each cell the level of the values it covers, lowest
/// to highest of all of them, in the eight levels.
pub fn sparkline(look: &Look, values: &[f64], width: usize) -> String {
    let levels = look.glyphs().spark;
    if values.is_empty() || width == 0 {
        return String::new();
    }
    let (lo, hi) = values
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let span = (hi - lo).max(f64::EPSILON);
    (0..width)
        .map(|k| {
            // The values this cell covers, by their places, and their mean.
            let a = k * values.len() / width;
            let b = ((k + 1) * values.len() / width).max(a + 1).min(values.len());
            let mean = values[a..b].iter().sum::<f64>() / (b - a) as f64;
            let level = (((mean - lo) / span) * 7.0).round() as usize;
            levels[level.min(7)]
        })
        .collect()
}

/// The words a panel says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelWords {
    pub expand: String,
    pub menu: String,
    pub empty: String,
    pub failed: String,
    pub retry: String,
    pub stale: String,
    pub loading: String,
    pub no_data: String,
    pub target: String,
}

impl Default for PanelWords {
    fn default() -> Self {
        PanelWords {
            expand: "Expand".into(),
            menu: "Panel actions".into(),
            empty: "No data for this range.".into(),
            failed: "This panel could not load.".into(),
            retry: "Retry".into(),
            stale: "Not up to date".into(),
            loading: "Loading".into(),
            no_data: "No data".into(),
            target: "Target".into(),
        }
    }
}

/// What a panel shows in its content's place: the content, or one of the states that keep apart
/// so no reader takes a failure for no data (principle 11).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PanelRead {
    #[default]
    Content,
    /// Under way since `elapsed` ms: the spinner shows after 300.
    Loading(u64),
    Empty,
    /// What failed; Retry beside it.
    Failed(Option<String>),
}

/// Where a panel put what a press can hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PanelLayout {
    pub body: Rect,
    pub expand: Option<Rect>,
    pub menu: Option<Rect>,
    pub retry: Option<Rect>,
}

/// The frame every dashboard panel shares. Build it each frame; draw it, then the content into
/// [`DashboardPanel::layout`]'s body while it reads [`PanelRead::Content`].
#[derive(Debug, Clone)]
pub struct DashboardPanel<'a> {
    look: &'a Look,
    title: String,
    subtitle: Option<String>,
    stale: Option<String>,
    expandable: bool,
    menu: bool,
    read: PanelRead,
    flush: bool,
    foot: Option<String>,
    focus: Option<PanelTool>,
    words: PanelWords,
}

/// A panel's own tools, for keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTool {
    Expand,
    Menu,
    Retry,
}

impl<'a> DashboardPanel<'a> {
    pub fn new(look: &'a Look, title: impl Into<String>) -> DashboardPanel<'a> {
        DashboardPanel {
            look,
            title: title.into(),
            subtitle: None,
            stale: None,
            expandable: false,
            menu: false,
            read: PanelRead::Content,
            flush: false,
            foot: None,
            focus: None,
            words: PanelWords::default(),
        }
    }

    /// A quiet line after the title: the range the panel covers.
    pub fn subtitle(mut self, words: impl Into<String>) -> Self {
        self.subtitle = Some(words.into());
        self
    }

    /// The content is kept but not up to date: the stale mark and since when, in the head.
    pub fn stale(mut self, since: impl Into<String>) -> Self {
        self.stale = Some(since.into());
        self
    }

    /// Expand opens the content in a full-screen dialog (the app draws it).
    pub fn expandable(mut self, on: bool) -> Self {
        self.expandable = on;
        self
    }

    /// The panel menu's button at the head's end (the app opens a DropdownMenu at it).
    pub fn menu(mut self, on: bool) -> Self {
        self.menu = on;
        self
    }

    pub fn read(mut self, read: PanelRead) -> Self {
        self.read = read;
        self
    }

    /// The content edge to edge, unpadded.
    pub fn flush(mut self, on: bool) -> Self {
        self.flush = on;
        self
    }

    pub fn foot(mut self, words: impl Into<String>) -> Self {
        self.foot = Some(words.into());
        self
    }

    pub fn focus(mut self, tool: Option<PanelTool>) -> Self {
        self.focus = tool;
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

    fn paint(&self, key: &str, field: &str) -> Option<Rgba> {
        self.look.paint(&format!("dashboard-panel.base.{key}"), field)
    }

    /// Where its pieces land in `area`.
    pub fn layout(&self, area: Rect) -> PanelLayout {
        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        let foot = if self.foot.is_some() { 2 } else { 0 };
        let (px, py) = if self.flush {
            (0, 0)
        } else {
            (by_density(self.look, 2, 1), by_density(self.look, 1, 0))
        };
        let body_y = inner.y + 2 + py;
        let body = Rect::new(
            inner.x + px,
            body_y,
            inner.width.saturating_sub(2 * px),
            inner.bottom().saturating_sub(body_y + py + foot),
        );
        let mut x = inner.right().saturating_sub(by_density(self.look, 1, 1));
        let mut tool = |on: bool| {
            on.then(|| {
                x = x.saturating_sub(3);
                let r = Rect::new(x, inner.y, 3, 1);
                x = x.saturating_sub(1);
                r
            })
        };
        let menu = tool(self.menu);
        let expand = tool(self.expandable);
        let retry = matches!(self.read, PanelRead::Failed(_)).then(|| {
            let w = Button::new(self.look, self.words.retry.clone())
                .size(ButtonSize::Sm)
                .width();
            Rect::new(body.x, body.y + 2, w, 1)
        });
        PanelLayout {
            body,
            expand,
            menu,
            retry,
        }
    }

    /// The tool a press at `at` lands on, in a panel drawn at `area`.
    pub fn hit(&self, area: Rect, at: ratatui::layout::Position) -> Option<PanelTool> {
        let l = self.layout(area);
        let on = |r: Option<Rect>| r.is_some_and(|r| r.contains(at));
        if on(l.expand) {
            Some(PanelTool::Expand)
        } else if on(l.menu) {
            Some(PanelTool::Menu)
        } else if on(l.retry) {
            Some(PanelTool::Retry)
        } else {
            None
        }
    }
}

impl Widget for &DashboardPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height < 4 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let page = look.color("background");
        let ground = self.paint("rest.panel", "ground").unwrap_or_else(|| look.color("card"));
        let edge = self
            .paint("rest.panel", "edge")
            .unwrap_or_else(|| look.color("border-strong"));
        let ink = look.color("card-foreground");
        let quiet = look.color("muted-foreground");
        Block::new()
            .borders(Borders::ALL)
            .border_set(look.border_set("radius-surface"))
            .border_style(Style::new().fg(t(edge)).bg(t(page)))
            .render(area, buf);
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
        buf.set_style(inner, Style::new().bg(t(ground)).fg(t(ink)));
        let l = self.layout(area);

        // The head: the title, the subtitle a column after it, the stale mark; the tools at its end.
        let pad = by_density(look, 1, 1);
        let tools_x = [l.expand, l.menu]
            .iter()
            .flatten()
            .map(|r| r.x)
            .min()
            .unwrap_or(inner.right());
        let room = tools_x.saturating_sub(inner.x + pad + 1) as usize;
        let mut x = inner.x + pad;
        let mut put = |buf: &mut Buffer, words: &str, style: Style| {
            let used = x.saturating_sub(inner.x + pad) as usize;
            if used >= room {
                return;
            }
            let s = text::truncate(words, room - used, glyphs.ellipsis);
            buf.set_string(x, inner.y, &s, style);
            x += text::width(&s) as u16 + 1;
        };
        put(buf, &self.title, TextRole::Label.style(look).fg(t(ink)));
        if let Some(sub) = &self.subtitle {
            let ink = self.paint("rest.subtitle", "ink").unwrap_or(quiet);
            put(buf, sub, TextRole::Help.style(look).fg(t(ink)));
        }
        if let (Some(since), false) = (&self.stale, matches!(self.read, PanelRead::Failed(_))) {
            let ink = self
                .paint("rest.stale", "ink")
                .unwrap_or_else(|| look.color("warning-ink"));
            put(
                buf,
                &format!("{} {since}", glyphs.stale),
                TextRole::Help.style(look).fg(t(ink)),
            );
        }
        let tool_ink = self.paint("rest.tool", "ink").unwrap_or(quiet);
        let ring = look.color("ring");
        for (r, glyph, tool) in [
            (l.expand, glyphs.expand, PanelTool::Expand),
            (l.menu, glyphs.more, PanelTool::Menu),
        ] {
            let Some(r) = r else { continue };
            buf.set_string(r.x + 1, r.y, glyph, Style::new().fg(t(tool_ink)));
            if self.focus == Some(tool) {
                buf.set_string(r.x, r.y, glyphs.ring_start, Style::new().fg(t(ring)));
                buf.set_string(r.x + 2, r.y, glyphs.ring_end, Style::new().fg(t(ring)));
            }
        }
        let rule = self.paint("rest.head", "rule").unwrap_or_else(|| look.color("border"));
        for x in inner.x..inner.right() {
            buf[(x, inner.y + 1)].set_symbol(glyphs.rule).set_fg(t(rule));
        }

        // The content's place, in its state.
        let body = l.body;
        match &self.read {
            PanelRead::Content => {}
            _ if body.height == 0 || body.bottom() > inner.bottom() => {}
            PanelRead::Loading(ms) if *ms >= 300 => {
                let words = format!("{} {}", look.spinner_frame(*ms), self.words.loading);
                let w = text::width(&words) as u16;
                let y = body.y + body.height / 2;
                buf.set_string(
                    body.x + body.width.saturating_sub(w) / 2,
                    y,
                    words,
                    TextRole::Help.style(look).fg(t(quiet)),
                );
            }
            PanelRead::Loading(_) => {}
            PanelRead::Empty => {
                let ink = self.paint("empty.empty", "ink").unwrap_or(quiet);
                let s = text::truncate(&self.words.empty, body.width as usize, glyphs.ellipsis);
                let w = text::width(&s) as u16;
                buf.set_string(
                    body.x + body.width.saturating_sub(w) / 2,
                    body.y + body.height / 2,
                    s,
                    TextRole::Body.style(look).fg(t(ink)),
                );
            }
            PanelRead::Failed(why) => {
                let critical = look.color("critical-ink");
                let head = format!("{} {}", glyphs.issue_critical, self.words.failed);
                buf.set_stringn(
                    body.x,
                    body.y,
                    head,
                    body.width as usize,
                    TextRole::Label.style(look).fg(t(critical)),
                );
                if let Some(why) = why {
                    let s = text::truncate(why, body.width as usize, glyphs.ellipsis);
                    buf.set_string(body.x, body.y + 1, s, TextRole::Help.style(look).fg(t(quiet)));
                }
                if let Some(r) = l.retry.filter(|r| r.bottom() <= inner.bottom()) {
                    Button::new(look, self.words.retry.clone())
                        .variant(ButtonVariant::Outline)
                        .size(ButtonSize::Sm)
                        .focused(self.focus == Some(PanelTool::Retry))
                        .render(r, buf);
                }
            }
        }

        // The foot: a rule and a quiet line.
        if let Some(foot) = &self.foot {
            let fy = inner.bottom() - 2;
            let rule = self.paint("rest.foot", "rule").unwrap_or(rule);
            let ink = self.paint("rest.foot", "ink").unwrap_or(quiet);
            for x in inner.x..inner.right() {
                buf[(x, fy)].set_symbol(glyphs.rule).set_fg(t(rule));
            }
            let s = text::truncate(foot, inner.width.saturating_sub(2 * pad) as usize, glyphs.ellipsis);
            buf.set_string(inner.x + pad, fy + 1, s, TextRole::Help.style(look).fg(t(ink)));
        }
    }
}

/// How a stat tile shows its reading's color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorMode {
    /// The value, its unit and its trend in the color.
    #[default]
    Value,
    /// The whole tile in the color, its words in `on-fill`.
    Background,
    None,
}

/// One reading as a dashboard tile.
#[derive(Debug, Clone)]
pub struct StatPanel<'a> {
    look: &'a Look,
    title: String,
    value: Option<f64>,
    unit: Option<String>,
    decimals: usize,
    thresholds: Vec<Threshold>,
    color: Option<String>,
    mode: ColorMode,
    spark: Vec<f64>,
    delta: Option<(f64, String, bool)>,
    target: Option<(f64, Option<(f64, f64)>)>,
    note: Option<String>,
    stale: Option<String>,
    read: PanelRead,
    focused: bool,
    words: PanelWords,
}

impl<'a> StatPanel<'a> {
    pub fn new(look: &'a Look, title: impl Into<String>, value: Option<f64>) -> StatPanel<'a> {
        StatPanel {
            look,
            title: title.into(),
            value,
            unit: None,
            decimals: 0,
            thresholds: Vec::new(),
            color: None,
            mode: ColorMode::Value,
            spark: Vec::new(),
            delta: None,
            target: None,
            note: None,
            stale: None,
            read: PanelRead::Content,
            focused: false,
            words: PanelWords::default(),
        }
    }

    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    /// The scale that picks the reading's color.
    pub fn thresholds(mut self, scale: impl IntoIterator<Item = Threshold>) -> Self {
        self.thresholds = scale.into_iter().collect();
        self
    }

    /// A fixed color token, in place of a scale.
    pub fn color(mut self, token: impl Into<String>) -> Self {
        self.color = Some(token.into());
        self
    }

    pub fn color_mode(mut self, mode: ColorMode) -> Self {
        self.mode = mode;
        self
    }

    /// The trend behind the value: a sparkline across the tile's foot.
    pub fn sparkline(mut self, values: impl IntoIterator<Item = f64>) -> Self {
        self.spark = values.into_iter().collect();
        self
    }

    /// The change against an earlier period, what it is against, and whether up is good.
    pub fn delta(mut self, change: f64, compared: impl Into<String>, up_is_good: bool) -> Self {
        self.delta = Some((change, compared.into(), up_is_good));
        self
    }

    /// A meter toward `target`, between `range` (else 0 and the larger of the reading and it).
    pub fn target(mut self, target: f64, range: Option<(f64, f64)>) -> Self {
        self.target = Some((target, range));
        self
    }

    pub fn note(mut self, words: impl Into<String>) -> Self {
        self.note = Some(words.into());
        self
    }

    pub fn stale(mut self, since: impl Into<String>) -> Self {
        self.stale = Some(since.into());
        self
    }

    pub fn read(mut self, read: PanelRead) -> Self {
        self.read = read;
        self
    }

    /// Keyboard focus on the tile (a tile that is a link or a button).
    pub fn focused(mut self, on: bool) -> Self {
        self.focused = on;
        self
    }

    pub fn words(mut self, words: PanelWords) -> Self {
        self.words = words;
        self
    }

    /// The rows it takes: its edge, the head, the value, a note, the sparkline, a meter.
    pub fn height(&self) -> u16 {
        let note = u16::from(self.note.is_some() || self.stale.is_some());
        let spark = if self.spark.is_empty() { 0 } else { 1 };
        let meter = if self.target.is_some() { 2 } else { 0 };
        2 + 1 + 1 + 1 + note + spark + meter
    }

    fn paint(&self, key: &str, field: &str) -> Option<Rgba> {
        self.look.paint(&format!("stat-panel.base.{key}"), field)
    }

    /// The reading's color token: the fixed color, else the scale's step.
    fn reading_color(&self) -> Option<&str> {
        if self.mode == ColorMode::None {
            return None;
        }
        self.color
            .as_deref()
            .or_else(|| self.value.and_then(|v| threshold_color(&self.thresholds, v)))
    }
}

impl Widget for &StatPanel<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 8 || area.height < 4 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let page = look.color("background");
        let reading = self
            .reading_color()
            .filter(|c| look.has_color(c))
            .map(|c| look.color(c));
        let filled = self.mode == ColorMode::Background && reading.is_some() && self.value.is_some();
        let state = if filled { "filled" } else { "rest" };
        let piece = |name: &str, field: &str| {
            self.paint(&format!("{state}.{name}"), field)
                .or_else(|| self.paint(&format!("rest.{name}"), field))
        };
        let ground = if filled {
            reading.unwrap_or(page)
        } else {
            piece("tile", "ground").unwrap_or_else(|| look.color("card"))
        };
        let edge = if filled {
            ground
        } else {
            piece("tile", "edge").unwrap_or_else(|| look.color("border"))
        };
        let ink = if filled {
            look.color("on-fill")
        } else {
            piece("tile", "ink").unwrap_or_else(|| look.color("card-foreground"))
        };
        let quiet = if filled {
            look.color("on-fill")
        } else {
            look.color("muted-foreground")
        };
        Block::new()
            .borders(Borders::ALL)
            .border_set(look.border_set("radius-surface"))
            .border_style(Style::new().fg(t(edge)).bg(t(page)))
            .render(area, buf);
        let inner = Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2);
        buf.set_style(inner, Style::new().bg(t(ground)).fg(t(ink)));
        let pad = by_density(look, 1, 1);
        let room = inner.width.saturating_sub(2 * pad);
        let x = inner.x + pad;
        let mut y = inner.y;

        // The head: the title, the change at its end.
        let mut delta_w = 0;
        if let Some((change, compared, up_good)) = &self.delta {
            let up = *change >= 0.0;
            let good = up == *up_good;
            let key = if *change == 0.0 {
                "rest"
            } else if good {
                "good"
            } else {
                "bad"
            };
            let d_ink = if filled {
                piece("delta", "ink").unwrap_or(ink)
            } else {
                self.paint(&format!("{key}.delta"), "ink").unwrap_or(quiet)
            };
            let arrow = if up { glyphs.arrow_up } else { glyphs.arrow_down };
            let figure = format!("{arrow} {}", format_value(change.abs(), self.decimals.max(1)));
            // The title first: the change says what it is against only while both fit.
            let title_w = text::width(&self.title) as u16;
            let long = format!("{figure} {compared}");
            let words = if title_w + 2 + text::width(&long) as u16 <= room {
                long
            } else {
                figure
            };
            delta_w = text::width(&words) as u16;
            if title_w.min(8) + 2 + delta_w <= room {
                buf.set_string(
                    inner.right() - pad - delta_w,
                    y,
                    &words,
                    TextRole::Help.style(look).fg(t(d_ink)),
                );
            } else {
                delta_w = 0;
            }
        }
        let title_ink = piece("title", "ink").unwrap_or(quiet);
        let title_room = room.saturating_sub(if delta_w > 0 { delta_w + 1 } else { 0 }) as usize;
        let title = text::truncate(&self.title, title_room, glyphs.ellipsis);
        buf.set_string(x, y, title, TextRole::Label.style(look).fg(t(title_ink)));
        y += 1;
        if self.focused {
            let ring = look.color("ring");
            buf.set_string(inner.x, inner.y, glyphs.focus_mark, Style::new().fg(t(ring)));
        }

        // The value with its unit, or what stands in its place.
        let value_ink = if filled { ink } else { reading.unwrap_or(ink) };
        match (&self.read, self.value) {
            (PanelRead::Failed(_), _) => {
                let f_ink = piece("failure", "ink").unwrap_or_else(|| look.color("critical-ink"));
                let s = text::truncate(&self.words.failed, room as usize, glyphs.ellipsis);
                buf.set_string(x, y, s, TextRole::Help.style(look).fg(t(f_ink)));
            }
            (PanelRead::Loading(ms), _) => {
                let words = format!("{} {}", look.spinner_frame(*ms), self.words.loading);
                buf.set_string(x, y, words, TextRole::Help.style(look).fg(t(quiet)));
            }
            (_, None) => {
                let n_ink = self.paint("noData.value", "ink").unwrap_or(quiet);
                buf.set_string(
                    x,
                    y,
                    &self.words.no_data,
                    TextRole::DetailTitle.style(look).fg(t(n_ink)),
                );
            }
            (_, Some(v)) => {
                let figure = format_value(v, self.decimals);
                buf.set_string(x, y, &figure, TextRole::Statistic.style(look).fg(t(value_ink)));
                if let Some(unit) = &self.unit {
                    let ux = x + text::width(&figure) as u16 + 1;
                    let u_ink = if reading.is_some() || filled {
                        value_ink
                    } else {
                        piece("unit", "ink").unwrap_or(quiet)
                    };
                    buf.set_string(ux, y, unit, TextRole::Label.style(look).fg(t(u_ink)));
                }
            }
        }
        y += 1;
        let has_value = self.value.is_some() && matches!(self.read, PanelRead::Content);

        // The note, or the stale mark under the value.
        if let Some(since) = &self.stale {
            let s_ink = look
                .paint("dashboard-panel.base.rest.stale", "ink")
                .unwrap_or_else(|| look.color("warning-ink"));
            let s_ink = if filled { ink } else { s_ink };
            buf.set_string(
                x,
                y,
                format!("{} {since}", glyphs.stale),
                TextRole::Help.style(look).fg(t(s_ink)),
            );
            y += 1;
        } else if let Some(note) = &self.note {
            let n_ink = piece("note", "ink").unwrap_or(quiet);
            let s = text::truncate(note, room as usize, glyphs.ellipsis);
            buf.set_string(x, y, s, TextRole::Help.style(look).fg(t(n_ink)));
            y += 1;
        }

        // The meter toward the target: the track, the tick, Target and its figure.
        if let (Some((target, range)), Some(v), true) = (self.target, self.value, has_value) {
            let (lo, hi) = range.unwrap_or((0.0, v.max(target)));
            let w = room.min(cols(192.0)).max(4);
            let at =
                |n: f64| (((n - lo) / (hi - lo).max(f64::EPSILON)).clamp(0.0, 1.0) * (w - 1) as f64).round() as u16;
            let (fill, tick) = (at(v), at(target));
            let track = look.color("muted");
            let tick_ink = piece("tick", "ground").unwrap_or(ink);
            for k in 0..w {
                let (g, c) = if k == tick {
                    (glyphs.ring_start, tick_ink)
                } else if k <= fill {
                    (glyphs.heavy_rule, value_ink)
                } else {
                    (glyphs.heavy_rule, track)
                };
                buf[(x + k, y)].set_symbol(g).set_fg(t(c));
            }
            y += 1;
            let l_ink = piece("targetLabel", "ink").unwrap_or(quiet);
            let label = format!("{} {}", self.words.target, format_value(target, self.decimals));
            buf.set_string(x, y, label, TextRole::Help.style(look).fg(t(l_ink)));
            y += 1;
        }

        // The trend: a sparkline across the foot row, in the value's ink.
        if !self.spark.is_empty() && has_value && y < inner.bottom() {
            let line = sparkline(look, &self.spark, room as usize);
            let s_ink = if reading.is_some() || filled { value_ink } else { quiet };
            buf.set_string(x, inner.bottom() - 1, line, Style::new().fg(t(s_ink)));
        }
    }
}
