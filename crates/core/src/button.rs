//! Button, from sokkel-design `parts/button`.
//!
//! Words, an icon and words, or an icon alone named by its label. A button that cannot act now
//! says why; a busy one keeps its place, its look and its focus.
//!
//! In cells (the part's `cells` note) a button is one row at every size: the sizes keep their text
//! role (`text.badge` for `xs` and `sm`, `text.action` above), and padding is one column. Its edge
//! is drawn in its pad cells (`▏Filter▕`, open-questions T3, decided b). Keyboard focus draws the
//! ring in the cell beside each end, outside its width, half a cell off the box as
//! `focus-ring-offset` keeps it off (T4, as decided); a row of buttons keeps a column between them.
//! A tooltip has no floating box here: [`Button::tip`] gives its words for the status line
//! (principle 23).

use crate::keys::KeyPress;
use crate::look::{Depth, Look, Rgba};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

/// The look. One default (filled) button per group, the main action, placed last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

impl ButtonVariant {
    /// The variant's name in the paints (`button.<variant>.<state>.container`).
    pub fn key(self) -> &'static str {
        match self {
            ButtonVariant::Default => "default",
            ButtonVariant::Outline => "outline",
            ButtonVariant::Secondary => "secondary",
            ButtonVariant::Ghost => "ghost",
            ButtonVariant::Destructive => "destructive",
            ButtonVariant::Link => "link",
        }
    }
}

/// The sizes. In cells every size is one row with one column of padding; the size picks the text
/// role. The square icon sizes are [`Button::icon`]'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Xs,
    Sm,
    #[default]
    Default,
    Lg,
}

impl ButtonSize {
    pub fn role(self) -> TextRole {
        match self {
            ButtonSize::Xs | ButtonSize::Sm => TextRole::Badge,
            ButtonSize::Default | ButtonSize::Lg => TextRole::Action,
        }
    }
}

/// Where the icon stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconPlace {
    #[default]
    Start,
    End,
}

/// What a tooltip would say: for the status line while the button has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tip {
    pub words: String,
    /// A key that does the same, as the app writes it (`Ctrl K`).
    pub shortcut: Option<String>,
}

/// A button. Build it each frame, draw it, and ask it whether a key pressed it.
#[derive(Debug, Clone)]
pub struct Button<'a> {
    look: &'a Look,
    label: String,
    variant: ButtonVariant,
    size: ButtonSize,
    icon: Option<&'a str>,
    icon_place: IconPlace,
    icon_only: bool,
    tooltip: Option<String>,
    shortcut: Option<String>,
    pressed: Option<bool>,
    busy: bool,
    busy_ms: u64,
    reason: Option<String>,
    disabled: bool,
    destructive_hover: bool,
    opens_menu: bool,
    open: bool,
    invalid: bool,
    focused: bool,
    hovered: bool,
}

impl<'a> Button<'a> {
    /// A button with words.
    pub fn new(look: &'a Look, label: impl Into<String>) -> Button<'a> {
        Button {
            look,
            label: label.into(),
            variant: ButtonVariant::Default,
            size: ButtonSize::Default,
            icon: None,
            icon_place: IconPlace::Start,
            icon_only: false,
            tooltip: None,
            shortcut: None,
            pressed: None,
            busy: false,
            busy_ms: 0,
            reason: None,
            disabled: false,
            destructive_hover: false,
            opens_menu: false,
            open: false,
            invalid: false,
            focused: false,
            hovered: false,
        }
    }

    /// An icon button: the glyph alone, named by `label`, which is also its tooltip, one string so
    /// the two never drift. Ghost-like, its glyph in the quiet ink until hovered.
    pub fn icon(look: &'a Look, glyph: &'a str, label: impl Into<String>) -> Button<'a> {
        let mut b = Button::new(look, label);
        b.icon = Some(glyph);
        b.icon_only = true;
        b.size = ButtonSize::Sm;
        b
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// A glyph before the label, or after it with [`IconPlace::End`].
    pub fn with_icon(mut self, glyph: &'a str, place: IconPlace) -> Self {
        self.icon = Some(glyph);
        self.icon_place = place;
        self
    }

    /// Work a press started is under way, for `elapsed_ms` so far: the spinner shows, presses are
    /// ignored, focus and look are kept.
    pub fn busy(mut self, busy: bool, elapsed_ms: u64) -> Self {
        self.busy = busy;
        self.busy_ms = elapsed_ms;
        self
    }

    /// Why it cannot be pressed now. Disables it and keeps it in the tab order; the reason is its
    /// tip.
    pub fn reason(mut self, reason: Option<impl Into<String>>) -> Self {
        self.reason = reason.map(Into::into);
        self
    }

    /// Cannot be pressed and leaves the tab order. Prefer [`Button::reason`].
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// The critical tint under the pointer, for a button that destroys but should not shout.
    pub fn destructive_hover(mut self, on: bool) -> Self {
        self.destructive_hover = on;
        self
    }

    /// It opens a menu, which is `open` now.
    pub fn opens_menu(mut self, open: bool) -> Self {
        self.opens_menu = true;
        self.open = open;
        self
    }

    /// Marked invalid by a form, as a control in a refused group.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The pointer is over it. Kept for terminals that report motion; nothing relies on it.
    pub fn hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    /// Icon button: a longer sentence its tip says in place of the label.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Icon button: the key that does the same, as the app writes it.
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Icon button: on or off, for a button that toggles.
    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = Some(pressed);
        self
    }

    /// Cannot be pressed now: a reason, or disabled.
    pub fn is_disabled(&self) -> bool {
        self.disabled || self.reason.is_some()
    }

    /// Tab reaches it: also while busy and while disabled with a reason, not while disabled
    /// without one.
    pub fn focusable(&self) -> bool {
        !self.disabled || self.reason.is_some()
    }

    /// Whether `key` presses it: Enter or Space, when it is enabled and not busy.
    pub fn presses(&self, key: KeyPress) -> bool {
        key.is_press() && !self.is_disabled() && !self.busy
    }

    /// What its tooltip says, for the status line while it has focus: a reason, else an icon
    /// button's tooltip or label; with the shortcut.
    pub fn tip(&self) -> Option<Tip> {
        let words = self.reason.clone().or_else(|| {
            self.icon_only
                .then(|| self.tooltip.clone().unwrap_or_else(|| self.label.clone()))
        })?;
        Some(Tip {
            words,
            shortcut: self.shortcut.clone(),
        })
    }

    /// The columns it takes: its ground (`cells`: the label measured, a column of padding at each
    /// end, an icon and a gap before it; an icon button three). Keyboard focus draws its ring in the
    /// cell beside each end, outside this width, in the gap a row of buttons keeps between them.
    pub fn width(&self) -> u16 {
        let inner = if self.icon_only {
            1
        } else {
            let lead = usize::from(self.icon.is_some() && self.icon_place == IconPlace::Start || self.busy);
            let trail = usize::from(self.icon.is_some() && self.icon_place == IconPlace::End);
            text::width(&self.label) + 2 * (lead + trail)
        };
        (inner + 2) as u16
    }

    /// The paint state its ground and ink are drawn in.
    fn ground_state(&self) -> &'static str {
        if self.icon_only && self.pressed == Some(true) {
            "pressed"
        } else if self.open {
            "open"
        } else if self.hovered && !self.busy && !self.is_disabled() {
            "hover"
        } else {
            "rest"
        }
    }

    /// The paint state its edge is drawn in.
    fn edge_state(&self) -> &'static str {
        match (self.invalid, self.focused) {
            (true, true) => "invalid+focus",
            (true, false) => "invalid",
            (false, true) => "focus",
            (false, false) => self.ground_state(),
        }
    }

    fn variant_key(&self) -> &'static str {
        if self.icon_only && self.variant == ButtonVariant::Default {
            "icon-button"
        } else {
            self.variant.key()
        }
    }

    /// Whether its paint draws a ground at all in its state.
    fn has_ground(&self) -> bool {
        let state = if self.is_disabled() {
            "rest"
        } else {
            self.ground_state()
        };
        let key = if self.destructive_hover && state == "hover" && !self.is_disabled() {
            "button.destructive-hover.hover.container".to_string()
        } else {
            format!("button.{}.{state}.container", self.variant_key())
        };
        self.look.shows(&key, "ground")
    }

    /// Ground, ink and edge, opaque as the contract resolves them; a disabled button's faded.
    fn colors(&self) -> (Rgba, Rgba, Option<Rgba>) {
        let look = self.look;
        let page = look.color("background");
        let variant = self.variant_key();
        if self.is_disabled() {
            let key = format!("button.{variant}.rest.container");
            let ground = look.faded(&key, "ground").unwrap_or(page);
            let ink = look.faded(&key, "ink").unwrap_or(ground);
            let edge = look.shows(&key, "edge").then(|| look.faded(&key, "edge")).flatten();
            return (ground, ink, edge);
        }
        let state = self.ground_state();
        let key = if self.destructive_hover && state == "hover" {
            "button.destructive-hover.hover.container".to_string()
        } else {
            format!("button.{variant}.{state}.container")
        };
        let ground = look.paint(&key, "ground").unwrap_or(page);
        let ink = look
            .paint(&key, "ink")
            .or_else(|| look.paint(&format!("button.{variant}.rest.container"), "ink"))
            .unwrap_or_else(|| look.color("foreground"));
        let edge_key = format!("button.{variant}.{}.container", self.edge_state());
        let edge = look
            .shows(&edge_key, "edge")
            .then(|| look.paint(&edge_key, "edge"))
            .flatten();
        (ground, ink, edge)
    }
}

impl Widget for Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        (&self).render(area, buf);
    }
}

impl Widget for &Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.width < 2 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let bare = look.depth() == Depth::None;
        let (ground, ink, edge) = self.colors();
        let t = |c: Rgba| look.terminal(c);
        // A ground the contract leaves fully transparent (ghost, link, an icon button at rest) is no
        // ground: the cells keep what they stand on, whatever surface that is.
        let bg = |st: Style| {
            if self.has_ground() { st.bg(t(ground)) } else { st }
        };

        let mut text_style = bg(self.size.role().style(look).fg(t(ink)));
        if self.variant == ButtonVariant::Link && self.ground_state() == "hover" {
            text_style = text_style.add_modifier(Modifier::UNDERLINED);
        }
        // Without color, focus is reversed: the one cue left (principle 22).
        if bare && self.focused {
            text_style = text_style.add_modifier(Modifier::REVERSED);
        }
        let box_style = bg(Style::new().fg(t(edge.unwrap_or(ground))));

        let y = area.y;
        let end = area.x + area.width.min(self.width());
        let mut x = area.x;
        let (start, stop) = if bare {
            (glyphs.bare_start, glyphs.bare_end)
        } else if edge.is_some() {
            (glyphs.edge_start, glyphs.edge_end)
        } else {
            (" ", " ")
        };
        buf[(x, y)].set_symbol(start).set_style(box_style);
        x += 1;

        // The content: the room left before the end's pad cell.
        let room = end.saturating_sub(x + 1) as usize;
        let spinner = self.busy.then(|| look.spinner_frame(self.busy_ms));
        let mut content: Vec<(String, Style)> = Vec::new();
        let icon_style = bg(Style::new().fg(t(ink)));
        if self.icon_only {
            content.push((spinner.or(self.icon).unwrap_or(" ").to_string(), icon_style));
        } else {
            let lead = spinner.or(if self.icon_place == IconPlace::Start {
                self.icon
            } else {
                None
            });
            if let Some(g) = lead {
                content.push((format!("{g} "), icon_style));
            }
            let trail = (self.icon_place == IconPlace::End).then_some(self.icon).flatten();
            let trail_w = if trail.is_some() { 2 } else { 0 };
            let used: usize = content.iter().map(|(s, _)| text::width(s)).sum();
            let label_room = room.saturating_sub(used + trail_w);
            content.push((text::truncate(&self.label, label_room, glyphs.ellipsis), text_style));
            if let Some(g) = trail {
                content.push((format!(" {g}"), icon_style));
            }
        }
        for (s, st) in content {
            for c in s.chars() {
                let mut b = [0u8; 4];
                let w = text::char_width(c) as u16;
                if x + w > end.saturating_sub(1) {
                    break;
                }
                buf[(x, y)].set_symbol(c.encode_utf8(&mut b)).set_style(st);
                for k in 1..w {
                    buf[(x + k, y)].set_symbol("").set_style(st);
                }
                x += w.max(1);
            }
        }
        while x + 1 < end {
            buf[(x, y)].set_symbol(" ").set_style(box_style);
            x += 1;
        }
        if x < end {
            buf[(x, y)].set_symbol(stop).set_style(box_style);
        }

        // Keyboard focus: the ring in the cell beside each end, on what is drawn there.
        if self.focused && !bare {
            let focus_key = format!("button.{}.focus.container", self.variant_key());
            let ring = look.ring(&focus_key, "outline").unwrap_or_else(|| look.color("ring"));
            let whole = buf.area;
            if area.x > whole.x {
                buf[(area.x - 1, y)].set_symbol(glyphs.ring_start).set_fg(t(ring));
            }
            if end < whole.right() {
                buf[(end, y)].set_symbol(glyphs.ring_end).set_fg(t(ring));
            }
        }
    }
}

/// Buttons in a group (`inGroup`): they touch, the group squaring their inner corners, so one
/// divider stands between two neighbors, in the cell they share. Keyboard focus draws its ring on
/// the focused button's own dividers (the cells beside it at the group's ends), never on a
/// neighbor's words.
#[derive(Debug, Clone)]
pub struct ButtonGroup<'a> {
    look: &'a Look,
    buttons: Vec<Button<'a>>,
}

impl<'a> ButtonGroup<'a> {
    pub fn new(look: &'a Look, buttons: impl IntoIterator<Item = Button<'a>>) -> ButtonGroup<'a> {
        ButtonGroup {
            look,
            buttons: buttons.into_iter().collect(),
        }
    }

    /// Its width: the buttons' widths, less the cell each two neighbors share.
    pub fn width(&self) -> u16 {
        let sum: u16 = self.buttons.iter().map(Button::width).sum();
        sum.saturating_sub(self.buttons.len().saturating_sub(1) as u16)
    }

    /// Where each button stands in a group drawn at `area`, for the pointer.
    pub fn areas(&self, area: Rect) -> Vec<Rect> {
        let mut x = area.x;
        self.buttons
            .iter()
            .map(|b| {
                let r = Rect::new(x, area.y, b.width(), 1).intersection(area);
                x += b.width().saturating_sub(1);
                r
            })
            .collect()
    }
}

impl Widget for &ButtonGroup<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        if area.height == 0 || self.buttons.is_empty() {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let bare = look.depth() == Depth::None;
        let t = |c: Rgba| look.terminal(c);
        let areas = self.areas(area);
        let mut focused = None;
        for (k, (b, r)) in self.buttons.iter().zip(&areas).enumerate() {
            let mut b = b.clone();
            if b.focused && !bare {
                focused = Some((k, b.variant_key()));
                b.focused = false;
            }
            (&b).render(*r, buf);
        }
        // The dividers, in the cells neighbors share: the first one's edge, or the border.
        let page = look.color("background");
        let line = self.buttons[0].colors().2.unwrap_or_else(|| look.color("border"));
        for r in areas.iter().skip(1) {
            buf[(r.x, r.y)]
                .set_symbol(look.box_glyphs("radius-control").down)
                .set_style(Style::new().fg(t(line)).bg(t(page)));
        }
        if let Some((k, variant)) = focused {
            let ring = look
                .ring(&format!("button.{variant}.focus.container"), "outline")
                .unwrap_or_else(|| look.color("ring"));
            let r = areas[k];
            let y = r.y;
            let start = if k == 0 {
                r.x.checked_sub(1).filter(|x| *x >= buf.area.x)
            } else {
                Some(r.x)
            };
            let end = if k + 1 == areas.len() {
                Some(r.right()).filter(|x| *x < buf.area.right())
            } else {
                Some(r.right() - 1)
            };
            if let Some(x) = start {
                buf[(x, y)].set_symbol(glyphs.ring_start).set_fg(t(ring));
            }
            if let Some(x) = end {
                buf[(x, y)].set_symbol(glyphs.ring_end).set_fg(t(ring));
            }
        }
    }
}
