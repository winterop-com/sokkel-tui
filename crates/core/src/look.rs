//! The look: a theme, a scheme, a density, the color depth and the character set the terminal
//! can show.
//!
//! Every color a part draws comes from here, opaque as the contract resolves it. A paint is read
//! by its resolved key (`button.default.hover.container`) in its `flat` form, which the resolver
//! writes over the surface each piece stands on (`standsOn`), and a disabled one in its `faded`
//! form. The renderer never mixes (principle 20). In 256 colors each is the nearest of xterm's
//! 16 to 255 by OKLab, as `colorDepth.ansi256` says. In sixteen colors each takes the index its
//! token's role gives it (`colorDepth.ansi16.paints`, T1): a paint by the token its source names
//! (`paintSources`), the terminal's own theme deciding what the index looks like, and a `reverse`
//! ground drawn in reverse video ([`Look::finish`]). With no color, words and glyphs carry it.
//!
//! Under a layer (a dialog, the palette, the drawer) the cells are flattened under the scrim at run
//! time ([`Look::scrim`], T2): the one blend a renderer is allowed.

use crate::glyphs::{Charset, Glyphs};
use crate::tokens::{Density, Scheme, index, resolved};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::widgets::BorderType;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// How many colors the terminal shows (`colorDepth`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Depth {
    /// 24-bit color: the theme as the contract resolves it.
    #[default]
    TrueColor,
    /// xterm's indices 16 to 255, each the nearest to the resolved color by OKLab.
    Ansi256,
    /// The terminal's sixteen, by role (T1).
    Ansi16,
    /// No color (`NO_COLOR`): bold, reverse and words carry the meaning.
    None,
}

impl Depth {
    /// What the environment says: `NO_COLOR`, then `COLORTERM`.
    pub fn detect() -> Depth {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return Depth::None;
        }
        // SOKKEL_COLORS says outright (truecolor, 256, 16, none); else what the terminal says.
        match std::env::var("SOKKEL_COLORS").as_deref() {
            Ok("truecolor") | Ok("24bit") => return Depth::TrueColor,
            Ok("256") => return Depth::Ansi256,
            Ok("16") => return Depth::Ansi16,
            Ok("none") => return Depth::None,
            _ => {}
        }
        if matches!(std::env::var("COLORTERM").as_deref(), Ok("truecolor") | Ok("24bit")) {
            return Depth::TrueColor;
        }
        // 256 colors unless the terminal is one known to have only sixteen: guessing sixteen from
        // a plain `xterm` or `screen` would flatten terminals that show 256.
        match std::env::var("TERM").as_deref() {
            Ok("dumb") => Depth::None,
            Ok("linux") | Ok("vt100") | Ok("vt220") | Ok("ansi") | Ok("cons25") => Depth::Ansi16,
            _ => Depth::Ansi256,
        }
    }
}

/// An sRGB color with alpha, as the resolved file writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// In sixteen colors: what each opaque color of this theme and scheme draws as.
    ansi16: Option<Arc<HashMap<Rgba, Ansi16>>>,
}

/// What a color draws as in sixteen colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ansi16 {
    /// The terminal's own default for the field.
    Default,
    /// Reverse video, in place of a ground.
    Reverse,
    Index(u8),
}

/// A cell's color that stands for reverse video in sixteen colors, until [`Look::finish`] turns it
/// into the modifier: index 16 is never drawn at that depth.
const REVERSE: Color = Color::Indexed(16);

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
            ansi16: None,
        })
    }

    pub fn with_density(mut self, density: Density) -> Look {
        self.density = density;
        self
    }

    pub fn with_depth(mut self, depth: Depth) -> Look {
        self.depth = depth;
        self.ansi16 = (depth == Depth::Ansi16).then(|| Arc::new(self.ansi16_map()));
        self
    }

    /// The sixteen-color map: each color token of the theme by its role, then each paint's flat
    /// colors by the token its source names.
    fn ansi16_map(&self) -> HashMap<Rgba, Ansi16> {
        let roles = &index()["colorDepth"]["ansi16"];
        let names: Vec<&str> = roles["tokens"]
            .as_array()
            .map_or(Vec::new(), |a| a.iter().filter_map(Value::as_str).collect());
        let role = |token: &str| -> Ansi16 {
            match roles["paints"].get(token).and_then(Value::as_str) {
                Some("reverse") => Ansi16::Reverse,
                Some(ansi) => names
                    .iter()
                    .position(|n| *n == ansi)
                    .map_or(Ansi16::Default, |i| Ansi16::Index(i as u8)),
                None => Ansi16::Default,
            }
        };
        // Tokens share resolved colors, so a color can stand for several roles: an index wins
        // over the terminal's default, and the default over reverse, which keeps reverse video for
        // the grounds only hover and selection paint.
        let rank = |r: Ansi16| match r {
            Ansi16::Index(_) => 2,
            Ansi16::Default => 1,
            Ansi16::Reverse => 0,
        };
        let mut map: HashMap<Rgba, Ansi16> = HashMap::new();
        let mut put = |c: Rgba, r: Ansi16| {
            let e = map.entry(c).or_insert(r);
            if rank(r) > rank(*e) {
                *e = r;
            }
        };
        if let Some(colors) = self.tokens["colors"].as_object() {
            for (name, hex) in colors {
                if let Some(c) = hex.as_str().and_then(Rgba::parse).filter(|c| c.is_opaque()) {
                    put(c, role(name));
                }
            }
        }
        if let Some(sources) = resolved()["paintSources"][self.scheme.key()].as_object() {
            for (key, fields) in sources {
                let Some(fields) = fields.as_object() else { continue };
                for (field, source) in fields {
                    let Some(source) = source.as_str() else { continue };
                    // The first token the source names: `primary/80`, `mix(card, primary 10%)`.
                    let token = source
                        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                        .find(|w| {
                            !w.is_empty() && *w != "mix" && *w != "none" && !w.chars().all(|c| c.is_ascii_digit())
                        });
                    if let (Some(token), Some(c)) = (token, self.paint(key, field)) {
                        put(c, role(token));
                    }
                }
            }
        }
        map
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

    /// Whether the theme has this color token.
    pub fn has_color(&self, name: &str) -> bool {
        self.tokens["colors"].get(name).is_some()
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
            Depth::Ansi16 => match self.ansi16.as_ref().and_then(|m| m.get(&c)) {
                Some(Ansi16::Index(i)) => Color::Indexed(*i),
                Some(Ansi16::Reverse) => REVERSE,
                _ => Color::Reset,
            },
            Depth::None => Color::Reset,
        }
    }

    /// What the frame needs after it is drawn: in sixteen colors, a cell a `reverse` paint grounds
    /// is drawn in reverse video on the terminal's own colors. The runtime calls it each frame.
    pub fn finish(&self, buf: &mut Buffer) {
        if self.depth != Depth::Ansi16 {
            return;
        }
        let area = buf.area;
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                if cell.bg == REVERSE || cell.fg == REVERSE {
                    cell.set_bg(Color::Reset).set_fg(Color::Reset);
                    cell.modifier.insert(Modifier::REVERSED);
                }
            }
        }
    }

    /// A paint's field as the resolver writes it, alpha and all: only the scrim reads it, the one
    /// blend a renderer makes (T2).
    pub fn paint_raw(&self, key: &str, field: &str) -> Option<Rgba> {
        self.field_of(key, field).and_then(Rgba::parse)
    }

    /// Lay what lies in `area` under a layer's scrim (`colorDepth.underLayer`, T2): in truecolor
    /// and 256 colors each cell's ground and ink flattened under the dialog's backdrop, source-over
    /// in sRGB; in sixteen colors every cell in ansi-bright-black on the terminal's ground; with no
    /// color plain. The glyphs stay.
    pub fn scrim(&self, buf: &mut Buffer, area: Rect) {
        let area = area.intersection(buf.area);
        let scrim = self
            .paint_raw("dialog.base.rest.backdrop", "ground")
            .or_else(|| Rgba::parse(self.color_hex("scrim")))
            .unwrap_or(Rgba(0, 0, 0, 77));
        let page = self.color("background");
        let ink = self.color("foreground");
        let over = |c: Rgba| -> Rgba {
            let a = scrim.3 as f32 / 255.0;
            let mix = |s: u8, d: u8| (s as f32 * a + d as f32 * (1.0 - a)).round() as u8;
            Rgba(mix(scrim.0, c.0), mix(scrim.1, c.1), mix(scrim.2, c.2), 255)
        };
        let back = |c: Color, own: Rgba| -> Rgba {
            match c {
                Color::Rgb(r, g, b) => Rgba(r, g, b, 255),
                Color::Indexed(i) => xterm_256(i),
                _ => own,
            }
        };
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &mut buf[(x, y)];
                match self.depth {
                    Depth::TrueColor | Depth::Ansi256 => {
                        let (bg, fg) = (over(back(cell.bg, page)), over(back(cell.fg, ink)));
                        cell.set_bg(self.terminal(bg)).set_fg(self.terminal(fg));
                    }
                    Depth::Ansi16 => {
                        cell.set_bg(Color::Reset).set_fg(Color::Indexed(8));
                        cell.modifier = Modifier::empty();
                    }
                    Depth::None => {
                        cell.set_bg(Color::Reset).set_fg(Color::Reset);
                        cell.modifier = Modifier::empty();
                    }
                }
            }
        }
    }

    /// The theme's sixteen as the terminal shows them in a sokkel terminal (`ansi-*`), by index.
    pub fn ansi(&self, n: u8) -> Rgba {
        let names = &index()["colorDepth"]["ansi16"]["tokens"];
        names[n as usize].as_str().map_or(Rgba(0, 0, 0, 255), |n| self.color(n))
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
