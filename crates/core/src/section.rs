//! Section, from sokkel-design `parts/section`.
//!
//! A titled part of a page. Boxed (the default) is a bordered surface whose head is a bar of its
//! own over a rule, its line and ground a framed table's and a card's. Flat is a heading over a
//! rule with the content on the page's own ground. Either takes a count or fact beside the title,
//! a description under it, and actions at the head's end. A tone draws the body as a graph's dot
//! grid or marks the section critical.
//!
//! In cells (the part's `cells` note) the boxed head is one row on the heads' ground (one more per
//! line of a description), its rule a row of tees joined to the box's sides. The head and body are
//! padded two columns at regular and one at compact; the body has a blank row inside the box at
//! the top and bottom at regular, none at compact. A flat head is its title row and a rule row,
//! then a blank row at regular. A critical box draws its top line heavy in `critical` (sokkel-tui's
//! reading of the web's 2 px rule) and a warning mark before the title. Draw the section, then the
//! body into [`Section::body`].

use crate::button::Button;
use crate::cells::by_density;
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

    /// The side padding of the head and the body.
    fn pad_x(&self) -> u16 {
        by_density(self.look, 2, 1)
    }

    /// Body padding across and down.
    fn pad(&self) -> (u16, u16) {
        if self.flush || !self.boxed() {
            (0, 0)
        } else {
            (self.pad_x(), by_density(self.look, 1, 0))
        }
    }

    /// The head's own side padding: inside the box, none over a flat head.
    fn head_pad(&self) -> u16 {
        if self.boxed() { self.pad_x() } else { 0 }
    }

    fn description_lines(&self, width: u16) -> u16 {
        let side = 2 * u16::from(self.boxed());
        let room = width.saturating_sub(side + 2 * self.head_pad());
        self.description
            .as_ref()
            .map_or(0, |d| text::wrap(d, room as usize).len() as u16)
    }

    /// Rows above the body at `width` columns.
    fn top(&self, width: u16) -> u16 {
        let head = 1 + self.description_lines(width);
        if self.boxed() {
            1 + head + 1
        } else {
            head + 1 + by_density(self.look, 1, 0)
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

    /// Where each action lands in `area`, a column apart at the head's end, for focus stops.
    pub fn action_areas(&self, area: Rect) -> Vec<Rect> {
        let side = u16::from(self.boxed());
        let y = area.y + side;
        let mut x = area.right().saturating_sub(side + self.head_pad());
        let mut out: Vec<Rect> = self
            .actions
            .iter()
            .rev()
            .map(|b| {
                x = x.saturating_sub(b.width());
                let r = Rect::new(x, y, b.width(), 1);
                x = x.saturating_sub(1);
                r
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
        let page = look.color("background");
        let critical = self.tone == SectionTone::Critical;
        let paint = |key: &str, field: &str| look.paint(&format!("section.base.{key}"), field);
        let box_ground = paint("rest.box", "ground").unwrap_or(page);
        let head_ground = if self.boxed() {
            paint("rest.head", "ground").unwrap_or(box_ground)
        } else {
            page
        };
        let side = u16::from(self.boxed());
        let head_rows = 1 + self.description_lines(area.width);
        let rule_y;
        let bx = look.box_glyphs("radius-surface");

        if self.boxed() {
            // The box: its border on the page, its inside the card, the head's ground over the rule.
            let edge = paint("rest.box", "edge").unwrap_or(box_ground);
            let block = Block::new()
                .borders(Borders::ALL)
                .border_set(look.border_set("radius-surface"))
                .border_style(Style::new().fg(t(edge)).bg(t(page)));
            let inner = block.inner(area);
            block.render(area, buf);
            buf.set_style(inner, Style::new().bg(t(box_ground)));
            if critical {
                let rule = paint("toneCritical.box", "rule").unwrap_or(edge);
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
                let rule = paint("rest.head", "rule").unwrap_or(edge);
                let st = Style::new().fg(t(rule)).bg(t(box_ground));
                buf[(area.x, rule_y)].set_symbol(bx.tee_start).set_style(st.bg(t(page)));
                for x in area.x + 1..area.right() - 1 {
                    buf[(x, rule_y)].set_symbol(bx.across).set_style(st);
                }
                buf[(area.right() - 1, rule_y)]
                    .set_symbol(bx.tee_end)
                    .set_style(st.bg(t(page)));
            }
        } else {
            rule_y = area.y + head_rows;
            if rule_y < area.bottom() {
                let key = if critical {
                    "toneCritical.flatHead"
                } else {
                    "rest.flatHead"
                };
                let rule = paint(key, "rule").unwrap_or(page);
                let line = if critical { glyphs.heavy_rule } else { glyphs.rule };
                for x in area.x..area.right() {
                    buf[(x, rule_y)].set_symbol(line).set_style(Style::new().fg(t(rule)));
                }
            }
        }

        // The head: mark, title, meta, actions at its end, the description under them.
        let hx = area.x + side + self.head_pad();
        let hy = area.y + side;
        let hw = area.width.saturating_sub(2 * (side + self.head_pad()));
        let actions_w: u16 = self.actions.iter().map(|b| b.width() + 1).sum();
        let mut x = hx;
        if critical {
            let ink = paint("toneCritical.mark", "ink").unwrap_or_else(|| look.color("critical"));
            buf.set_string(x, hy, glyphs.issue_warning, Style::new().fg(t(ink)));
            x += 2;
        }
        let (role, title_key) = if self.boxed() {
            (TextRole::Head, "rest.title")
        } else {
            (TextRole::SectionTitle, "rest.flatTitle")
        };
        let title_ink = paint(title_key, "ink").unwrap_or_else(|| look.color("foreground"));
        let meta_w = self.meta.as_ref().map_or(0, |m| text::width(m) as u16 + 1);
        let room = (hx + hw).saturating_sub(x + actions_w + meta_w);
        let title = text::truncate(&self.title, room as usize, glyphs.ellipsis);
        buf.set_string(x, hy, &title, role.style(look).fg(t(title_ink)));
        x += text::width(&title) as u16;
        let quiet = TextRole::Help.style(look);
        if let Some(m) = &self.meta {
            let ink = paint("rest.meta", "ink").unwrap_or_else(|| look.color("muted-foreground"));
            buf.set_string(x + 1, hy, m, quiet.fg(t(ink)));
        }
        for (b, r) in self.actions.iter().zip(self.action_areas(area)) {
            b.render(r, buf);
        }
        if let Some(d) = &self.description {
            let ink = paint("rest.description", "ink").unwrap_or_else(|| look.color("muted-foreground"));
            for (i, l) in text::wrap(d, hw as usize).into_iter().enumerate() {
                let y = hy + 1 + i as u16;
                if y < rule_y {
                    buf.set_string(hx, y, l, quiet.fg(t(ink)));
                }
            }
        }

        // A graph's body: the canvas with a dot every two columns, every row, centered.
        if self.tone == SectionTone::Graph {
            let body = self.body(area);
            let ground = paint("toneGraph.body", "ground").unwrap_or(box_ground);
            let dot = paint("toneGraph.body", "mark").unwrap_or(ground);
            let (px, py) = self.pad();
            let full = Rect {
                x: body.x - px,
                y: body.y - py,
                width: body.width + 2 * px,
                height: body.height + 2 * py,
            };
            buf.set_style(full, Style::new().bg(t(ground)));
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
