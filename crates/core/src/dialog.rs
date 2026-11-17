//! Dialog, from sokkel-design `parts/dialog`.
//!
//! A box over the page: a title and a line under it, the app's body, and a footer with the verbs,
//! the confirming one last. A busy confirm holds the dialog until `busy-release`, after which
//! Cancel says "Stop waiting". Tab stays inside it ([`crate::Focus`]'s layers), and closing gives
//! focus back to what opened it.
//!
//! In cells (the part carries no `cells` note yet; sokkel-tui's reading, in its proposal) the box
//! is centered, `sm` 384 px (46 columns), `md` 512 (61), `lg` 672 (80), `xl` 896 (107), `full` the
//! window less 4 columns, at most the window less 4 across and 2 down; below md a sheet fills the
//! window. Its 16 px padding is two columns across and a row down at regular; its pieces are a
//! blank row apart at regular. The footer is a rule row and a row of verbs on the footer's ground.
//! What lies under the dialog is drawn as it is: the backdrop's dim waits on open-questions T2.

use crate::button::{Button, ButtonVariant};
use crate::cells::{Width, by_density, cols};
use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Widget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogWidth {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
    /// The window less 32 px across and down: a workbench.
    Full,
}

/// The confirming verb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    pub label: String,
    /// Why it cannot confirm now: shut, still focusable, the reason its tip.
    pub why: Option<String>,
    pub busy: bool,
    /// What it says while busy (`Exporting`), in place of its label.
    pub busy_label: Option<String>,
    pub destructive: bool,
}

impl Confirm {
    pub fn new(label: impl Into<String>) -> Confirm {
        Confirm {
            label: label.into(),
            why: None,
            busy: false,
            busy_label: None,
            destructive: false,
        }
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn why(mut self, why: Option<impl Into<String>>) -> Self {
        self.why = why.map(Into::into);
        self
    }

    pub fn busy(mut self, busy: bool, label: Option<impl Into<String>>) -> Self {
        self.busy = busy;
        self.busy_label = label.map(Into::into);
        self
    }
}

/// The words a dialog says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogWords {
    pub cancel: String,
    pub back: String,
    pub close: String,
    pub stop_waiting: String,
}

impl Default for DialogWords {
    fn default() -> Self {
        DialogWords {
            cancel: "Cancel".into(),
            back: "Back".into(),
            close: "Close".into(),
            stop_waiting: "Stop waiting".into(),
        }
    }
}

/// Which of the dialog's own controls has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogFocus {
    #[default]
    None,
    Close,
    Cancel,
    Back,
    Action(usize),
    Confirm,
}

/// What a key did to the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogEvent {
    Ignored,
    /// Closed: Escape, Cancel or the corner close. Focus goes back to what opened it.
    Close,
    /// Back was pressed.
    Back,
    /// An action was pressed, by its place.
    Action(usize),
    Confirm,
    /// Let a busy confirm go after `busy-release`: the app stops the work if it can.
    StopWaiting,
}

/// Where the dialog put its pieces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogLayout {
    pub body: Rect,
    pub close: Option<Rect>,
    pub cancel: Option<Rect>,
    pub back: Option<Rect>,
    pub actions: Vec<Rect>,
    pub confirm: Option<Rect>,
}

/// A dialog over the page.
#[derive(Debug, Clone)]
pub struct Dialog<'a> {
    look: &'a Look,
    title: String,
    description: Option<String>,
    width: DialogWidth,
    confirm: Option<Confirm>,
    back: bool,
    actions: Vec<Button<'a>>,
    dismissible: bool,
    note: Option<String>,
    busy_ms: u64,
    focus: DialogFocus,
    words: DialogWords,
    sheet: bool,
}

impl<'a> Dialog<'a> {
    pub fn new(look: &'a Look, title: impl Into<String>) -> Dialog<'a> {
        Dialog {
            look,
            title: title.into(),
            description: None,
            width: DialogWidth::Md,
            confirm: None,
            back: false,
            actions: Vec::new(),
            dismissible: true,
            note: None,
            busy_ms: 0,
            focus: DialogFocus::None,
            words: DialogWords::default(),
            sheet: false,
        }
    }

    pub fn description(mut self, words: impl Into<String>) -> Self {
        self.description = Some(words.into());
        self
    }

    pub fn width(mut self, width: DialogWidth) -> Self {
        self.width = width;
        self
    }

    /// The one confirming verb: it brings Cancel, and no corner close.
    pub fn confirm(mut self, confirm: Confirm) -> Self {
        self.confirm = Some(confirm);
        self
    }

    pub fn back(mut self, on: bool) -> Self {
        self.back = on;
        self
    }

    pub fn actions(mut self, actions: impl IntoIterator<Item = Button<'a>>) -> Self {
        self.actions = actions.into_iter().collect();
        self
    }

    /// False: no Cancel and no close; Escape leaves it open.
    pub fn dismissible(mut self, on: bool) -> Self {
        self.dismissible = on;
        self
    }

    /// The app's note at the footer's start.
    pub fn note(mut self, words: impl Into<String>) -> Self {
        self.note = Some(words.into());
        self
    }

    /// How long a busy confirm has held the dialog, for `busy-release`.
    pub fn busy_for(mut self, ms: u64) -> Self {
        self.busy_ms = ms;
        self
    }

    pub fn focus(mut self, focus: DialogFocus) -> Self {
        self.focus = focus;
        self
    }

    pub fn words(mut self, words: DialogWords) -> Self {
        self.words = words;
        self
    }

    /// The window's width decides its form: below md a sheet filling the window.
    pub fn window(mut self, width: Width) -> Self {
        self.sheet = width == Width::Narrow;
        self
    }

    fn busy(&self) -> bool {
        self.confirm.as_ref().is_some_and(|c| c.busy)
    }

    /// A busy confirm has held the dialog past `busy-release`: Cancel says "Stop waiting".
    pub fn released(&self) -> bool {
        self.busy() && self.busy_ms >= self.look.duration_ms("busy-release")
    }

    fn has_cancel(&self) -> bool {
        self.dismissible && self.confirm.is_some()
    }

    fn has_close(&self) -> bool {
        self.dismissible && self.confirm.is_none()
    }

    fn has_footer(&self) -> bool {
        self.confirm.is_some() || self.back || !self.actions.is_empty() || self.note.is_some()
    }

    /// Answer a key while the dialog is open.
    pub fn handle(&self, key: KeyPress) -> DialogEvent {
        if key.alt || key.ctrl {
            return DialogEvent::Ignored;
        }
        if key.is(Key::Escape) {
            return if !self.dismissible || (self.busy() && !self.released()) {
                DialogEvent::Ignored
            } else if self.released() {
                DialogEvent::StopWaiting
            } else {
                DialogEvent::Close
            };
        }
        if !key.is_press() {
            return DialogEvent::Ignored;
        }
        match self.focus {
            DialogFocus::Confirm => {
                let c = self.confirm.as_ref();
                if c.is_some_and(|c| c.why.is_none() && !c.busy) {
                    DialogEvent::Confirm
                } else {
                    DialogEvent::Ignored
                }
            }
            DialogFocus::Cancel if self.released() => DialogEvent::StopWaiting,
            DialogFocus::Cancel if !self.busy() => DialogEvent::Close,
            DialogFocus::Close => DialogEvent::Close,
            DialogFocus::Back if !self.busy() => DialogEvent::Back,
            DialogFocus::Action(i) => DialogEvent::Action(i),
            _ => DialogEvent::Ignored,
        }
    }

    fn box_width(&self, window: Rect) -> u16 {
        let w = match self.width {
            DialogWidth::Sm => cols(384.0),
            DialogWidth::Md => cols(512.0),
            DialogWidth::Lg => cols(672.0),
            DialogWidth::Xl => cols(896.0),
            DialogWidth::Full => window.width,
        };
        w.min(window.width.saturating_sub(4))
    }

    fn description_lines(&self, inner_w: u16) -> u16 {
        self.description
            .as_ref()
            .map_or(0, |d| text::wrap(d, inner_w as usize).len() as u16)
    }

    /// Rows round a body of `body` rows at a box `w` columns wide.
    fn rows(&self, w: u16, body: u16) -> u16 {
        let pad = by_density(self.look, 1, 0);
        let gap = by_density(self.look, 1, 0);
        let inner_w = w.saturating_sub(2 + 4);
        let header = 1 + self.description_lines(inner_w);
        let footer = if self.has_footer() { 2 } else { 0 };
        2 + pad + header + gap + body + if footer > 0 { gap + footer } else { pad }
    }

    /// Where the dialog stands in `window`, around a body of `body` rows: centered; a sheet
    /// filling the window where [`Dialog::window`] said the window is below md.
    pub fn area(&self, window: Rect, body: u16) -> Rect {
        if self.sheet {
            return window;
        }
        let w = self.box_width(window);
        let h = self.rows(w, body).min(window.height.saturating_sub(2));
        Rect::new(
            window.x + (window.width - w) / 2,
            window.y + (window.height - h) / 2,
            w,
            h,
        )
    }

    fn verbs(&self) -> Vec<(DialogFocus, Button<'a>)> {
        let look = self.look;
        let mut v = Vec::new();
        if self.has_cancel() {
            let words = if self.released() {
                &self.words.stop_waiting
            } else {
                &self.words.cancel
            };
            let b = Button::new(look, words.clone())
                .variant(ButtonVariant::Ghost)
                .disabled(self.busy() && !self.released());
            v.push((DialogFocus::Cancel, b));
        }
        if self.back {
            let b = Button::new(look, self.words.back.clone())
                .variant(ButtonVariant::Outline)
                .disabled(self.busy());
            v.push((DialogFocus::Back, b));
        }
        for (i, a) in self.actions.iter().enumerate() {
            v.push((DialogFocus::Action(i), a.clone()));
        }
        if let Some(c) = &self.confirm {
            let words = if c.busy {
                c.busy_label.clone().unwrap_or_else(|| c.label.clone())
            } else {
                c.label.clone()
            };
            let variant = if c.destructive {
                ButtonVariant::Destructive
            } else {
                ButtonVariant::Default
            };
            let b = Button::new(look, words)
                .variant(variant)
                .busy(c.busy, self.busy_ms)
                .reason(c.why.clone());
            v.push((DialogFocus::Confirm, b));
        }
        v.into_iter()
            .map(|(f, b)| {
                let focused = self.focus == f;
                (f, b.focused(focused))
            })
            .collect()
    }

    /// Where the pieces land in the dialog's `area`.
    pub fn layout(&self, area: Rect) -> DialogLayout {
        let mut out = DialogLayout::default();
        let pad = by_density(self.look, 1, 0);
        let gap = by_density(self.look, 1, 0);
        let side = u16::from(!self.sheet);
        let inner = Rect {
            x: area.x + side + 2,
            y: area.y + side + pad,
            width: area.width.saturating_sub(2 * side + 4),
            height: area.height.saturating_sub(2 * side + pad),
        };
        if self.has_close() {
            out.close = Some(Rect::new(area.right().saturating_sub(side + 4), area.y + side, 3, 1));
        }
        let header = 1 + self.description_lines(inner.width);
        let footer = if self.has_footer() { 2 + gap } else { pad };
        out.body = Rect {
            y: inner.y + header + gap,
            height: inner.height.saturating_sub(header + gap + footer),
            ..inner
        };
        if self.has_footer() {
            let y = area.bottom().saturating_sub(side + 1);
            let mut x = inner.right();
            let verbs = self.verbs();
            for (f, b) in verbs.iter().rev() {
                x = x.saturating_sub(b.width());
                let r = Rect::new(x, y, b.width(), 1);
                x = x.saturating_sub(1);
                match f {
                    DialogFocus::Cancel => out.cancel = Some(r),
                    DialogFocus::Back => out.back = Some(r),
                    DialogFocus::Confirm => out.confirm = Some(r),
                    DialogFocus::Action(_) => out.actions.insert(0, r),
                    _ => {}
                }
            }
        }
        out
    }

    /// What the status line says while a verb has focus: a shut confirm's reason (principle 23).
    pub fn tip(&self) -> Option<String> {
        match self.focus {
            DialogFocus::Confirm => self.confirm.as_ref().and_then(|c| c.why.clone()),
            DialogFocus::Close => Some(self.words.close.clone()),
            _ => None,
        }
    }
}

impl Widget for &Dialog<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 12 || area.height < 4 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let sheet = self.sheet;
        let state = if sheet { "phone" } else { "rest" };
        let bkey = format!("dialog.base.{state}.box");
        let ground = look.paint(&bkey, "ground").unwrap_or_else(|| look.color("popover"));
        let ink = look
            .paint(&bkey, "ink")
            .unwrap_or_else(|| look.color("popover-foreground"));
        Clear.render(area, buf);
        if sheet {
            buf.set_style(area, Style::new().bg(t(ground)).fg(t(ink)));
        } else {
            let edge = look.paint(&bkey, "edge").unwrap_or(ground);
            let block = Block::new()
                .borders(Borders::ALL)
                .border_set(look.border_set("radius-surface"))
                .border_style(Style::new().fg(t(edge)).bg(t(look.color("background"))));
            let inner = block.inner(area);
            block.render(area, buf);
            buf.set_style(inner, Style::new().bg(t(ground)).fg(t(ink)));
        }
        let layout = self.layout(area);
        let side = u16::from(!sheet);
        let x = area.x + side + 2;
        let w = area.width.saturating_sub(2 * side + 4);
        let mut y = area.y + side + by_density(look, 1, 0);

        // The header: the title, and the line under it, muted.
        let title_room = w.saturating_sub(if self.has_close() { 4 } else { 0 }) as usize;
        buf.set_string(
            x,
            y,
            text::truncate(&self.title, title_room, glyphs.ellipsis),
            TextRole::DetailTitle.style(look).fg(t(ink)),
        );
        y += 1;
        if let Some(d) = &self.description {
            let di = look
                .paint("dialog.base.rest.description", "ink")
                .unwrap_or_else(|| look.color("muted-foreground"));
            for line in text::wrap(d, w as usize) {
                buf.set_string(x, y, line, TextRole::Body.style(look).fg(t(di)));
                y += 1;
            }
        }
        if let Some(r) = layout.close {
            let b = Button::icon(look, glyphs.close, self.words.close.clone())
                .variant(ButtonVariant::Ghost)
                .focused(self.focus == DialogFocus::Close);
            b.render(r, buf);
        }

        // The footer: a rule, then the note and the verbs on its ground.
        if self.has_footer() {
            let fkey = "dialog.base.rest.footer";
            let fg = look.paint(fkey, "ground").unwrap_or(ground);
            let rule = look.paint(fkey, "rule").unwrap_or(fg);
            let fy = area.bottom().saturating_sub(side + 2);
            let band = Rect::new(area.x + side, fy, area.width.saturating_sub(2 * side), 2);
            buf.set_style(band, Style::new().bg(t(fg)));
            for cx in band.x..band.right() {
                buf[(cx, fy)].set_symbol(glyphs.rule).set_fg(t(rule));
            }
            if let Some(n) = &self.note {
                let ni = look
                    .paint("dialog.base.rest.note", "ink")
                    .unwrap_or_else(|| look.color("muted-foreground"));
                let first = layout
                    .cancel
                    .or(layout.back)
                    .or(layout.actions.first().copied())
                    .or(layout.confirm)
                    .map_or(band.right(), |r| r.x);
                let room = first.saturating_sub(x + 2) as usize;
                buf.set_string(
                    x,
                    fy + 1,
                    text::truncate(n, room, glyphs.ellipsis),
                    TextRole::Help.style(look).fg(t(ni)).bg(t(fg)),
                );
            }
            let verbs = self.verbs();
            let rects = layout
                .cancel
                .into_iter()
                .chain(layout.back)
                .chain(layout.actions.iter().copied())
                .chain(layout.confirm);
            for ((_, b), r) in verbs.iter().zip(rects) {
                b.render(r, buf);
            }
        }
    }
}
