//! Section, from sokkel-design `parts/section`.
//!
//! A titled part of a page. Boxed (the default) is a bordered surface whose head is a bar of its
//! own over a rule, its line and ground a framed table's and a card's. Flat is a heading over a
//! rule with the content on the page's own ground. Either takes a count or fact beside the title,
//! a description under it, and actions at the head's end. A tone draws the body as a graph's dot
//! grid or marks the section critical.
//!
//! In cells the boxed head is one row (more with a description) on the heads' ground, its rule a
//! row of its own joined to the box's sides (`├──┤`). The body is padded `density.panel-pad`: a
//! column across, and a row down at regular density, none at compact. A critical box draws its top
//! line heavy in `critical`. Draw the section, then the body into [`Section::body`].

use crate::button::Button;
use crate::cells::{cols, rows};
use crate::glyphs::Charset;
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionVariant {
    #[default]
    Boxed,
    Flat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SectionTone {
    #[default]
    Plain,
    Graph,
    Critical,
}

/// A titled part of a page.
#[derive(Debug, Clone)]
pub struct Section<'a> {
    look: &'a Look,
    title: String,
    meta: Option<String>,
    description: Option<String>,
    actions: Vec<Button<'a>>,
    variant: SectionVariant,
    tone: SectionTone,
    flush: bool,
}

impl<'a> Section<'a> {
    pub fn new(look: &'a Look, title: impl Into<String>) -> Section<'a> {
        Section {
            look,
            title: title.into(),
            meta: None,
            description: None,
            actions: Vec::new(),
            variant: SectionVariant::Boxed,
            tone: SectionTone::Plain,
            flush: false,
        }
    }

    /// A count or short fact beside the title.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// A sentence under the title, in the head.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Controls at the head's end.
    pub fn actions(mut self, actions: impl IntoIterator<Item = Button<'a>>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    pub fn variant(mut self, variant: SectionVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn tone(mut self, tone: SectionTone) -> Self {
        self.tone = tone;
        self
    }

    /// No padding round the body: a table or a list runs to the box's edge.
    pub fn flush(mut self, flush: bool) -> Self {
        self.flush = flush;
        self
    }

    fn boxed(&self) -> bool {
        self.variant == SectionVariant::Boxed
    }

    /// The surface the head's actions stand on, a color token.
    fn head_surface(&self) -> &'static str {
        if self.variant == SectionVariant::Boxed {
            "section-head-ground"
        } else {
            "background"
        }
    }

    /// Body padding across and down.
    fn pad(&self) -> (u16, u16) {
        if self.flush || !self.boxed() {
            (0, 0)
        } else {
            let p = self.look.density_px("panel-pad");
            (cols(p), rows(p))
        }
    }

    fn description_lines(&self, width: u16) -> u16 {
        let room = width.saturating_sub(2 * self.head_pad());
        self.description
            .as_ref()
            .map_or(0, |d| text::wrap(d, room as usize).len() as u16)
    }

    fn head_pad(&self) -> u16 {
        if self.boxed() {
            cols(self.look.density_px("panel-pad"))
        } else {
            0
        }
    }

    /// Rows above the body at `width` columns.
    fn top(&self, width: u16) -> u16 {
        let head = 1 + self.description_lines(width.saturating_sub(2));
        if self.boxed() {
            1 + head + 1
        } else {
            head + 1 + rows(self.look.density_px("section-gap"))
        }
    }

    /// The rows it takes around a body of `body` rows.
    pub fn height(&self, width: u16, body: u16) -> u16 {
        let (_, py) = self.pad();
        self.top(width) + 2 * py + body + u16::from(self.boxed())
    }

    /// Where the body goes in `area`.
    pub fn body(&self, area: Rect) -> Rect {
        let (px, py) = self.pad();
        let side = u16::from(self.boxed());
        let top = self.top(area.width);
        Rect {
            x: area.x + side + px,
            y: area.y + top + py,
            width: area.width.saturating_sub(2 * (side + px)),
            height: area.height.saturating_sub(top + 2 * py + side),
        }
    }

    /// Where each action lands in `area`, for focus stops.
    pub fn action_areas(&self, area: Rect) -> Vec<Rect> {
        let side = u16::from(self.boxed());
        let y = area.y + side;
        let mut x = area.right().saturating_sub(side + self.head_pad().saturating_sub(1));
        let mut out: Vec<Rect> = self
            .actions
            .iter()
            .rev()
            .map(|b| {
                x = x.saturating_sub(b.width());
                Rect::new(x, y, b.width(), 1)
            })
            .collect();
        out.reverse();
        out
    }
}

impl Widget for Section<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &Section<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 6 || area.height < 2 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let page = look.surface("background");
        let critical = self.tone == SectionTone::Critical;
        let paint = |key: &str, field: &str, under: Rgba| look.paint_on(&format!("section.base.{key}"), field, under);
        let box_ground = paint("rest.box", "ground", page).unwrap_or(page);
        let head_ground = if self.boxed() {
            paint("rest.head", "ground", box_ground).unwrap_or(box_ground)
        } else {
            page
        };
        let side = u16::from(self.boxed());

        // The box, its head's ground, the rule under the head.
        let head_rows = 1 + self.description_lines(area.width.saturating_sub(2));
        let rule_y;
        if self.boxed() {
            let edge = paint("rest.box", "edge", box_ground).unwrap_or(box_ground);
            let block = Block::new()
                .borders(Borders::ALL)
                .border_set(look.border_set("radius-surface"))
                .border_style(Style::new().fg(t(edge)).bg(t(page)));
            let inner = block.inner(area);
            block.render(area, buf);
            buf.set_style(inner, Style::new().bg(t(box_ground)));
            if critical {
                let rule = paint("toneCritical.box", "rule", page).unwrap_or(edge);
                for x in area.x + 1..area.right() - 1 {
                    buf[(x, area.y)]
                        .set_symbol(glyphs.heavy_rule)
                        .set_style(Style::new().fg(t(rule)).bg(t(page)));
                }
            }
            let head = Rect::new(inner.x, inner.y, inner.width, head_rows.min(inner.height));
            buf.set_style(head, Style::new().bg(t(head_ground)));
            rule_y = area.y + 1 + head_rows;
            if rule_y < area.bottom() - 1 {
                let rule = paint("rest.head", "rule", box_ground).unwrap_or(edge);
                let (l, m, r) = match look.charset() {
                    Charset::Unicode => ("├", "─", "┤"),
                    Charset::Ascii => ("+", "-", "+"),
                };
                let st = Style::new().fg(t(rule)).bg(t(box_ground));
                buf[(area.x, rule_y)].set_symbol(l).set_style(st.bg(t(page)));
                for x in area.x + 1..area.right() - 1 {
                    buf[(x, rule_y)].set_symbol(m).set_style(st);
                }
                buf[(area.right() - 1, rule_y)].set_symbol(r).set_style(st.bg(t(page)));
            }
        } else {
            rule_y = area.y + head_rows;
            if rule_y < area.bottom() {
                let key = if critical {
                    "toneCritical.flatHead"
                } else {
                    "rest.flatHead"
                };
                let rule = paint(key, "rule", page).unwrap_or(page);
                let st = Style::new().fg(t(rule));
                let line = match look.charset() {
                    Charset::Unicode if critical => glyphs.heavy_rule,
                    Charset::Unicode => "─",
                    Charset::Ascii if critical => glyphs.heavy_rule,
                    Charset::Ascii => "-",
                };
                for x in area.x..area.right() {
                    buf[(x, rule_y)].set_symbol(line).set_style(st);
                }
            }
        }

        // The head: mark, title, meta, actions at its end, the description under them.
        let hx = area.x + side + self.head_pad();
        let hy = area.y + side;
        let hw = area.width.saturating_sub(2 * (side + self.head_pad()));
        let actions_w: u16 = self.actions.iter().map(Button::width).sum();
        let mut x = hx;
        if critical {
            let ink = paint("toneCritical.mark", "ink", head_ground).unwrap_or(head_ground);
            buf.set_string(x, hy, glyphs.issue_critical, Style::new().fg(t(ink)));
            x += 2;
        }
        let (role, title_key) = if self.boxed() {
            (TextRole::Head, "rest.title")
        } else {
            (TextRole::SectionTitle, "rest.flatTitle")
        };
        let title_ink = paint(title_key, "ink", head_ground).unwrap_or(head_ground);
        let meta_w = self.meta.as_ref().map_or(0, |m| text::width(m) as u16 + 1);
        let room = (hx + hw).saturating_sub(x + actions_w + meta_w + u16::from(actions_w > 0));
        let title = text::truncate(&self.title, room as usize, glyphs.ellipsis);
        buf.set_string(x, hy, &title, role.style(look).fg(t(title_ink)));
        x += text::width(&title) as u16;
        let quiet = TextRole::Help.style(look);
        if let Some(m) = &self.meta {
            let ink = paint("rest.meta", "ink", head_ground).unwrap_or(head_ground);
            buf.set_string(x + 1, hy, m, quiet.fg(t(ink)));
        }
        for (b, r) in self.actions.iter().zip(self.action_areas(area)) {
            b.clone().on(self.head_surface()).render(r, buf);
        }
        if let Some(d) = &self.description {
            let ink = paint("rest.description", "ink", head_ground).unwrap_or(head_ground);
            for (i, l) in text::wrap(d, hw as usize).into_iter().enumerate() {
                let y = hy + 1 + i as u16;
                if y < rule_y {
                    buf.set_string(hx, y, l, quiet.fg(t(ink)));
                }
            }
        }

        // A graph's body: the canvas with a dot every two columns, every row.
        if self.tone == SectionTone::Graph {
            let body = self.body(area);
            let under = if self.boxed() { box_ground } else { page };
            let ground = paint("toneGraph.body", "ground", under).unwrap_or(under);
            let dot = paint("toneGraph.body", "mark", ground).unwrap_or(ground);
            let (px, py) = self.pad();
            let full = Rect {
                x: body.x - px,
                y: body.y - py,
                width: body.width + 2 * px,
                height: body.height + 2 * py,
            };
            buf.set_style(full, Style::new().bg(t(ground)));
            // Centered: the spare column, if any, split between the two sides.
            let first = full.x + 1 + (full.width.saturating_sub(1) % 2) / 2;
            for y in full.y..full.bottom() {
                for x in (first..full.right().saturating_sub(1)).step_by(2) {
                    buf[(x, y)]
                        .set_symbol(glyphs.dot)
                        .set_style(Style::new().fg(t(dot)).bg(t(ground)));
                }
            }
        }
    }
}
