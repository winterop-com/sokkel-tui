//! The look: a theme, a scheme, a density and the color depth the terminal can show.
//!
//! Every color a part draws comes from here. A paint is read by its resolved key
//! (`button.default.hover.container`) and drawn as it is: the renderer never mixes (principle 20).
//!
//! Two things here stand in for what the contract does not hold yet, and leave once it does
//! (`docs/proposals/sokkel-design-tui.md`):
//! - **Flattening.** A terminal cannot blend, and some paints carry alpha (`#006aa0cc`). Until the
//!   resolver writes opaque paints over the surface each piece stands on, [`flat`] lays a color
//!   over the surface the caller names.
//! - **Fewer colors.** For a terminal without truecolor, [`Depth::Ansi256`] picks the nearest of
//!   xterm's 256. The contract's ANSI map would replace it.

use crate::glyphs::{Charset, Glyphs};
use crate::tokens::{Density, Scheme, resolved};
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use serde_json::Value;

/// How many colors the terminal shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Depth {
    /// 24-bit color: the theme as the contract resolves it.
    #[default]
    TrueColor,
    /// xterm's 256 colors, each the nearest to the resolved one.
    Ansi256,
    /// No color (`NO_COLOR`): bold, reverse and words carry the meaning.
    None,
}

impl Depth {
    /// What the environment says: `NO_COLOR`, then `COLORTERM`.
    pub fn detect() -> Depth {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return Depth::None;
        }
        match std::env::var("COLORTERM").as_deref() {
            Ok("truecolor") | Ok("24bit") => Depth::TrueColor,
            _ => Depth::Ansi256,
        }
    }
}

/// An sRGB color with alpha, as the resolved file writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba(pub u8, pub u8, pub u8, pub u8);

impl Rgba {
    /// `#rrggbb` or `#rrggbbaa`.
    pub fn parse(hex: &str) -> Option<Rgba> {
        let h = hex.strip_prefix('#')?;
        if h.len() != 6 && h.len() != 8 {
            return None;
        }
        let b = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
        Some(Rgba(b(0)?, b(2)?, b(4)?, if h.len() == 8 { b(6)? } else { 255 }))
    }

    pub fn is_opaque(self) -> bool {
        self.3 == 255
    }
}

/// `color` laid over `under`, which is taken as opaque. Stands in for the resolver's opaque paints.
pub fn flat(color: Rgba, under: Rgba) -> Rgba {
    let t = color.3 as f32 / 255.0;
    let m = |f: u8, u: u8| (f as f32 * t + u as f32 * (1.0 - t)).round() as u8;
    Rgba(m(color.0, under.0), m(color.1, under.1), m(color.2, under.2), 255)
}

/// The theme, scheme and density an app draws in, and the terminal's color depth.
#[derive(Debug, Clone)]
pub struct Look {
    theme: String,
    scheme: Scheme,
    density: Density,
    depth: Depth,
    charset: Charset,
    tokens: &'static Value,
    paints: &'static Value,
}

impl Look {
    /// A look in a theme the contract has; `None` for a theme it lacks.
    pub fn new(theme: &str, scheme: Scheme) -> Option<Look> {
        let all = resolved();
        let tokens = all["themes"].get(theme)?.get(scheme.key())?;
        let paints = all["paints"].get(theme)?.get(scheme.key())?;
        Some(Look {
            theme: theme.to_string(),
            scheme,
            density: Density::default(),
            depth: Depth::TrueColor,
            charset: Charset::Unicode,
            tokens,
            paints,
        })
    }

    pub fn with_density(mut self, density: Density) -> Look {
        self.density = density;
        self
    }

    pub fn with_depth(mut self, depth: Depth) -> Look {
        self.depth = depth;
        self
    }

    pub fn theme(&self) -> &str {
        &self.theme
    }
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }
    pub fn density(&self) -> Density {
        self.density
    }
    pub fn depth(&self) -> Depth {
        self.depth
    }

    /// A color token as the resolved file writes it (`primary` -> `#006aa0`).
    pub fn color_hex(&self, name: &str) -> &'static str {
        self.tokens["colors"][name]
            .as_str()
            .unwrap_or_else(|| panic!("the contract has no color {name}"))
    }

    /// One field of a resolved paint (`ground`, `edge`, `ink`, `rule`, `mark`, `placeholder`).
    /// A state the contract does not paint is drawn as `rest`, which is picking a paint, not mixing one.
    pub fn paint_hex(&self, key: &str, field: &str) -> &'static str {
        if let Some(v) = self.paints.get(key).and_then(|p| p.get(field)).and_then(Value::as_str) {
            return v;
        }
        let rest = rest_of(key);
        self.paints
            .get(&rest)
            .and_then(|p| p.get(field))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("the contract paints no {field} for {key}"))
    }

    /// Whether the contract paints this field for this key (or its `rest`).
    pub fn has_paint(&self, key: &str, field: &str) -> bool {
        let has = |k: &str| {
            self.paints
                .get(k)
                .and_then(|p| p.get(field))
                .and_then(Value::as_str)
                .is_some()
        };
        has(key) || has(&rest_of(key))
    }

    /// A color token, drawn over the surface named by `over` (a color token).
    pub fn color(&self, name: &str, over: &str) -> Color {
        self.draw(self.color_hex(name), self.color_hex(over))
    }

    /// A paint field, drawn over the surface named by `over` (a color token).
    pub fn paint(&self, key: &str, field: &str, over: &str) -> Color {
        self.draw(self.paint_hex(key, field), self.color_hex(over))
    }

    /// A paint field drawn over another paint's ground (a row over its table's frame).
    pub fn paint_over(&self, key: &str, field: &str, over_key: &str, over_field: &str) -> Color {
        let under = self.opaque(self.paint_hex(over_key, over_field));
        self.terminal(flat(self.parse(self.paint_hex(key, field)), under))
    }

    /// A paint field as a surface other colors lie on, already flat over `over`.
    pub fn surface_of(&self, key: &str, field: &str, over: &str) -> Rgba {
        flat(
            self.parse(self.paint_hex(key, field)),
            self.opaque(self.color_hex(over)),
        )
    }

    /// Draw `hex` over `under`, both written as the resolved file writes colors.
    pub fn draw(&self, hex: &str, under: &str) -> Color {
        self.terminal(flat(self.parse(hex), self.opaque(under)))
    }

    /// An opaque color in the terminal's depth.
    pub fn terminal(&self, c: Rgba) -> Color {
        match self.depth {
            Depth::TrueColor => Color::Rgb(c.0, c.1, c.2),
            Depth::Ansi256 => Color::Indexed(nearest_256(c)),
            Depth::None => Color::Reset,
        }
    }

    /// A length token in px (`radius-surface`, `spacing-finger`).
    pub fn length(&self, name: &str) -> f64 {
        self.tokens["lengths"][name]
            .as_f64()
            .unwrap_or_else(|| panic!("the contract has no length {name}"))
    }

    /// A density length in px at this look's level (`row`, `page-inset-wide`).
    pub fn density_px(&self, name: &str) -> f64 {
        resolved()["density"][self.density.key()][name]
            .as_f64()
            .unwrap_or_else(|| panic!("the contract has no density length {name}"))
    }

    /// A text role's resolved size, line height, weight and tracking.
    pub fn text_role(&self, role: &str) -> &'static Value {
        &self.tokens["text"][role]
    }

    /// A box's lines and corners: the radius ladder as one bit, rounded or square, in the look's
    /// character set (`+`, `-` and `|` in ASCII).
    pub fn border_set(&self, role: &str) -> ratatui::symbols::border::Set<'static> {
        use ratatui::symbols::border;
        match (self.charset, self.corners(role)) {
            (Charset::Ascii, _) => border::Set {
                top_left: "+",
                top_right: "+",
                bottom_left: "+",
                bottom_right: "+",
                vertical_left: "|",
                vertical_right: "|",
                horizontal_top: "-",
                horizontal_bottom: "-",
            },
            (Charset::Unicode, BorderType::Rounded) => border::ROUNDED,
            (Charset::Unicode, _) => border::PLAIN,
        }
    }

    /// Corners in cells: the radius ladder is one bit, rounded or square.
    pub fn corners(&self, role: &str) -> BorderType {
        if self.length(role) > 0.0 {
            BorderType::Rounded
        } else {
            BorderType::Plain
        }
    }

    /// A color token as a surface other colors lie on (`background`, `card`, `popover`, `sidebar`).
    pub fn surface(&self, name: &str) -> Rgba {
        self.opaque(self.color_hex(name))
    }

    /// A color token laid over a surface.
    pub fn color_on(&self, name: &str, under: Rgba) -> Rgba {
        flat(self.parse(self.color_hex(name)), under)
    }

    /// A paint field laid over a surface. `None` where the contract writes no color there
    /// (`inherit`, or a field the paint lacks).
    pub fn paint_on(&self, key: &str, field: &str, under: Rgba) -> Option<Rgba> {
        if !self.has_paint(key, field) {
            return None;
        }
        Rgba::parse(self.paint_hex(key, field)).map(|c| flat(c, under))
    }

    /// The color of a paint's `ring` or `outline` (`{color, width, offset}`), laid over a surface.
    pub fn paint_ring_on(&self, key: &str, field: &str, under: Rgba) -> Option<Rgba> {
        let hex = self.paints.get(key)?.get(field)?.get("color")?.as_str()?;
        Rgba::parse(hex).map(|c| flat(c, under))
    }

    /// Whether a paint field is drawn at all: present, a color, and not fully transparent.
    pub fn paint_shows(&self, key: &str, field: &str) -> bool {
        self.has_paint(key, field) && Rgba::parse(self.paint_hex(key, field)).is_some_and(|c| c.3 > 0)
    }

    /// A color as a disabled control shows it: laid over its surface at `disabled-opacity`.
    /// A stand-in, as [`flat`] is, until the resolver writes disabled paints opaque.
    pub fn faded(&self, color: Rgba, under: Rgba) -> Rgba {
        let a = (self.number("disabled-opacity") * 255.0).round() as u8;
        flat(Rgba(color.0, color.1, color.2, a), under)
    }

    /// A number token (`disabled-opacity`).
    pub fn number(&self, name: &str) -> f64 {
        self.tokens["numbers"][name]
            .as_f64()
            .unwrap_or_else(|| panic!("the contract has no number {name}"))
    }

    /// A duration token in milliseconds (`motion-fast`, `busy-release`, `tip-hold`).
    pub fn duration_ms(&self, name: &str) -> u64 {
        self.tokens["durations"][name]
            .as_u64()
            .unwrap_or_else(|| panic!("the contract has no duration {name}"))
    }

    /// The glyphs this look draws with.
    pub fn glyphs(&self) -> &'static Glyphs {
        self.charset.glyphs()
    }

    pub fn charset(&self) -> Charset {
        self.charset
    }

    pub fn with_charset(mut self, charset: Charset) -> Look {
        self.charset = charset;
        self
    }

    /// The spinner frame to draw after `elapsed_ms` of busy: one frame per `motion-fast`.
    pub fn spinner_frame(&self, elapsed_ms: u64) -> &'static str {
        let frames = self.glyphs().spinner;
        frames[((elapsed_ms / self.duration_ms("motion-fast").max(1)) as usize) % frames.len()]
    }

    fn parse(&self, hex: &str) -> Rgba {
        Rgba::parse(hex).unwrap_or_else(|| panic!("not a resolved color: {hex}"))
    }

    /// A surface is opaque by the time anything lies on it; a translucent one lies on `background`.
    fn opaque(&self, hex: &str) -> Rgba {
        let c = self.parse(hex);
        if c.is_opaque() {
            c
        } else {
            flat(c, self.parse(self.color_hex("background")))
        }
    }
}

/// The ink of what a piece stands on, for a paint whose ink is `inherit`: `foreground` on the
/// page, `popover-foreground` in a popup, `sidebar-foreground` on the rail.
pub fn ink_on(surface: &str) -> &'static str {
    match surface {
        "card" => "card-foreground",
        "popover" => "popover-foreground",
        "sidebar" => "sidebar-foreground",
        _ => "foreground",
    }
}

/// `table.base.hover.row` -> `table.base.rest.row`.
fn rest_of(key: &str) -> String {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.rest.{}", parts[0], parts[1], parts[3])
    } else {
        key.to_string()
    }
}

/// The sRGB value of one of xterm's 256 colors.
pub fn xterm_256(i: u8) -> Rgba {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    match i {
        0..=15 => {
            let (r, g, b) = BASE[i as usize];
            Rgba(r, g, b, 255)
        }
        16..=231 => {
            let n = i - 16;
            let step = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            Rgba(step(n / 36), step((n / 6) % 6), step(n % 6), 255)
        }
        _ => {
            let v = 8 + (i - 232) * 10;
            Rgba(v, v, v, 255)
        }
    }
}

/// The nearest of xterm's cube and grays (16 to 255), by distance in sRGB weighted for the eye.
fn nearest_256(c: Rgba) -> u8 {
    let d = |a: Rgba| {
        let (dr, dg, db) = (
            a.0 as i32 - c.0 as i32,
            a.1 as i32 - c.1 as i32,
            a.2 as i32 - c.2 as i32,
        );
        2 * dr * dr + 4 * dg * dg + 3 * db * db
    };
    (16..=255u8).min_by_key(|&i| d(xterm_256(i))).unwrap_or(16)
}
