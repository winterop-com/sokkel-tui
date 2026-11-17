//! The look: a theme, a scheme, a density, the color depth and the character set the terminal
//! can show.
//!
//! Every color a part draws comes from here, opaque as the contract resolves it. A paint is read
//! by its resolved key (`button.default.hover.container`) in its `flat` form, which the resolver
//! writes over the surface each piece stands on (`standsOn`), and a disabled one in its `faded`
//! form. The renderer never mixes (principle 20). In 256 colors each is the nearest of xterm's
//! 16 to 255 by OKLab, as `colorDepth.ansi256` says; with no color, words and glyphs carry it.

use crate::glyphs::{Charset, Glyphs};
use crate::tokens::{Density, Scheme, resolved};
use ratatui::style::Color;
use ratatui::widgets::BorderType;
use serde_json::Value;

/// How many colors the terminal shows (`colorDepth`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Depth {
    /// 24-bit color: the theme as the contract resolves it.
    #[default]
    TrueColor,
    /// xterm's indices 16 to 255, each the nearest to the resolved color by OKLab.
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

/// The theme, scheme and density an app draws in, and the terminal's color depth and characters.
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

    pub fn with_charset(mut self, charset: Charset) -> Look {
        self.charset = charset;
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
    pub fn charset(&self) -> Charset {
        self.charset
    }

    /// The glyphs this look draws with.
    pub fn glyphs(&self) -> &'static Glyphs {
        self.charset.glyphs()
    }

    /// A color token as the resolved file writes it (`primary` -> `#006aa0`).
    pub fn color_hex(&self, name: &str) -> &'static str {
        self.tokens["colors"][name]
            .as_str()
            .unwrap_or_else(|| panic!("the contract has no color {name}"))
    }

    /// An opaque color token (`foreground`, `ring`, `critical-ink`). A translucent one (`hover`)
    /// is read through the paint that uses it, never here.
    pub fn color(&self, name: &str) -> Rgba {
        let c = parse(self.color_hex(name));
        debug_assert!(c.is_opaque(), "{name} is translucent: read it through its paint");
        c
    }

    /// A resolved paint, or its `rest` where the contract does not paint the state: picking a
    /// paint, not mixing one.
    fn paint_of(&self, key: &str) -> Option<&'static Value> {
        self.paints.get(key).or_else(|| self.paints.get(rest_of(key)))
    }

    /// One field of a resolved paint as the file writes it (`#006aa0cc`, `inherit`).
    pub fn paint_hex(&self, key: &str, field: &str) -> &'static str {
        self.field_of(key, field)
            .unwrap_or_else(|| panic!("the contract paints no {field} for {key}"))
    }

    fn field_of(&self, key: &str, field: &str) -> Option<&'static str> {
        let own = |k: &str| self.paints.get(k)?.get(field)?.as_str();
        own(key).or_else(|| own(&rest_of(key)))
    }

    /// Whether the contract paints this field for this key (or its `rest`).
    pub fn has_paint(&self, key: &str, field: &str) -> bool {
        self.field_of(key, field).is_some()
    }

    /// A paint's field, opaque: its `flat` form where the resolved one blends or inherits, else
    /// the resolved one. `None` where the paint has no such field.
    pub fn paint(&self, key: &str, field: &str) -> Option<Rgba> {
        let own = |k: &str| -> Option<Rgba> {
            let p = self.paints.get(k)?;
            if let Some(flat) = p.get("flat").and_then(|f| f.get(field)).and_then(Value::as_str) {
                return Rgba::parse(flat);
            }
            Rgba::parse(p.get(field)?.as_str()?).filter(|c| c.is_opaque())
        };
        own(key).or_else(|| own(&rest_of(key)))
    }

    /// A paint's field as a disabled control shows it: its `faded` form, opaque.
    pub fn faded(&self, key: &str, field: &str) -> Option<Rgba> {
        let own = |k: &str| -> Option<Rgba> {
            let hex = self
                .paints
                .get(k)?
                .get("faded")?
                .get("disabled")?
                .get(field)?
                .as_str()?;
            Rgba::parse(hex)
        };
        own(key).or_else(|| own(&rest_of(key)))
    }

    /// The color of a paint's `ring` or `outline` (`{color, width}`), opaque.
    pub fn ring(&self, key: &str, field: &str) -> Option<Rgba> {
        let p = self.paint_of(key)?;
        if let Some(flat) = p.get("flat").and_then(|f| f.get(field)).and_then(Value::as_str) {
            return Rgba::parse(flat);
        }
        Rgba::parse(p.get(field)?.get("color")?.as_str()?).filter(|c| c.is_opaque())
    }

    /// Whether a paint field shows at all: a color that is not fully transparent. A transparent
    /// edge is no edge, whatever its flat form.
    pub fn shows(&self, key: &str, field: &str) -> bool {
        self.field_of(key, field).and_then(Rgba::parse).is_some_and(|c| c.3 > 0)
    }

    /// An opaque color in the terminal's depth.
    pub fn terminal(&self, c: Rgba) -> Color {
        match self.depth {
            Depth::TrueColor => Color::Rgb(c.0, c.1, c.2),
            Depth::Ansi256 => Color::Indexed(nearest_256(c)),
            Depth::None => Color::Reset,
        }
    }

    /// A color token in the terminal's depth.
    pub fn token(&self, name: &str) -> Color {
        self.terminal(self.color(name))
    }

    /// A paint field in the terminal's depth; panics where the contract paints none.
    pub fn fill(&self, key: &str, field: &str) -> Color {
        self.terminal(
            self.paint(key, field)
                .unwrap_or_else(|| panic!("the contract paints no {field} for {key}")),
        )
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

    /// A theme word (`table-head-transform`).
    pub fn word(&self, name: &str) -> Option<&'static str> {
        self.tokens["words"].get(name)?.as_str()
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

    /// A text role's resolved size, line height, weight and tracking.
    pub fn text_role(&self, role: &str) -> &'static Value {
        &self.tokens["text"][role]
    }

    /// Corners in cells: the radius ladder is one bit, rounded or square.
    pub fn corners(&self, role: &str) -> BorderType {
        if self.length(role) > 0.0 {
            BorderType::Rounded
        } else {
            BorderType::Plain
        }
    }

    /// A box's lines and corners from glyphs.json: rounded or square by the radius role, ASCII in
    /// the ASCII character set.
    pub fn border_set(&self, role: &str) -> ratatui::symbols::border::Set<'static> {
        let b = self.box_glyphs(role);
        ratatui::symbols::border::Set {
            top_left: b.top_start,
            top_right: b.top_end,
            bottom_left: b.bottom_start,
            bottom_right: b.bottom_end,
            vertical_left: b.down,
            vertical_right: b.down,
            horizontal_top: b.across,
            horizontal_bottom: b.across,
        }
    }

    /// A box's glyphs, its tees included, by the radius role.
    pub fn box_glyphs(&self, role: &str) -> &'static crate::glyphs::BoxGlyphs {
        let g = self.glyphs();
        match (self.charset, self.corners(role)) {
            (Charset::Ascii, _) => &g.box_ascii,
            (Charset::Unicode, BorderType::Rounded) => &g.box_rounded,
            (Charset::Unicode, _) => &g.box_square,
        }
    }

    /// The spinner frame to draw after `elapsed_ms` of busy: one frame per `motion-fast`.
    pub fn spinner_frame(&self, elapsed_ms: u64) -> &'static str {
        let frames = &self.glyphs().spinner;
        frames[((elapsed_ms / self.duration_ms("motion-fast").max(1)) as usize) % frames.len()]
    }
}

fn parse(hex: &str) -> Rgba {
    Rgba::parse(hex).unwrap_or_else(|| panic!("not a resolved color: {hex}"))
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

/// The sRGB value of one of xterm's 256 colors. 0 to 15 are the terminal's own; these are xterm's
/// defaults, for a picture of a buffer only.
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

/// An sRGB color in OKLab.
pub fn oklab(c: Rgba) -> [f64; 3] {
    let lin = |v: u8| {
        let v = v as f64 / 255.0;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(c.0), lin(c.1), lin(c.2));
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    ]
}

/// The nearest of xterm's cube and grays (16 to 255) by distance in OKLab, the lower index on a
/// tie (`colorDepth.ansi256`).
pub fn nearest_256(c: Rgba) -> u8 {
    let want = oklab(c);
    let d = |i: u8| {
        let o = oklab(xterm_256(i));
        (o[0] - want[0]).powi(2) + (o[1] - want[1]).powi(2) + (o[2] - want[2]).powi(2)
    };
    let mut best = 16u8;
    let mut best_d = d(16);
    for i in 17..=255u8 {
        let di = d(i);
        if di < best_d {
            best = i;
            best_d = di;
        }
    }
    best
}
