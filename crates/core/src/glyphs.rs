//! The glyphs the parts draw, from sokkel-design's `glyphs.json`, in its Unicode or ASCII form.
//!
//! A terminal cannot choose its font, so every glyph a part draws has a form any terminal shows.
//! Every glyph comes from glyphs.json; the bare brackets a control's ends take without color are the
//! ASCII forms of its edge.

use crate::tokens;
use serde_json::Value;
use std::sync::OnceLock;

/// Which set of glyphs the terminal can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Charset {
    /// glyphs.json's Unicode forms.
    #[default]
    Unicode,
    /// glyphs.json's ASCII forms.
    Ascii,
}

impl Charset {
    /// What the locale says: Unicode where `LC_ALL`, `LC_CTYPE` or `LANG` names UTF-8.
    pub fn detect() -> Charset {
        let utf8 = ["LC_ALL", "LC_CTYPE", "LANG"]
            .iter()
            .filter_map(|v| std::env::var(v).ok())
            .find(|v| !v.is_empty())
            .is_some_and(|v| {
                let v = v.to_ascii_lowercase();
                v.contains("utf-8") || v.contains("utf8")
            });
        if utf8 { Charset::Unicode } else { Charset::Ascii }
    }

    pub fn glyphs(self) -> &'static Glyphs {
        static UNICODE: OnceLock<Glyphs> = OnceLock::new();
        static ASCII: OnceLock<Glyphs> = OnceLock::new();
        match self {
            Charset::Unicode => UNICODE.get_or_init(|| Glyphs::read(self)),
            Charset::Ascii => ASCII.get_or_init(|| Glyphs::read(self)),
        }
    }
}

/// A box's corners, sides and tees (glyphs.json, `box`).
#[derive(Debug)]
pub struct BoxGlyphs {
    pub top_start: &'static str,
    pub top_end: &'static str,
    pub bottom_start: &'static str,
    pub bottom_end: &'static str,
    pub across: &'static str,
    pub down: &'static str,
    pub tee_start: &'static str,
    pub tee_end: &'static str,
}

/// One set of glyphs.
#[derive(Debug)]
pub struct Glyphs {
    /// Busy or loading, one frame each `motion-fast`.
    pub spinner: Vec<&'static str>,
    /// A critical issue's mark (`circle-alert`) and a warning's (`triangle-alert`).
    pub issue_critical: &'static str,
    pub issue_warning: &'static str,
    /// Text cut short at its end; ASCII's is three columns.
    pub ellipsis: &'static str,
    pub bullet: &'static str,
    /// Try again's turning arrow (`rotate-cw`).
    pub retry: &'static str,
    /// Back (`chevron-left`).
    pub back: &'static str,
    /// More: the button that holds what did not fit (the `ellipsis` icon).
    pub more: &'static str,
    /// After each crumb of a trail (`chevron-right`).
    pub chevron: &'static str,
    pub chevron_down: &'static str,
    /// The top bar's menu button, below md (`menu`).
    pub menu: &'static str,
    /// The search icon (`search`).
    pub search: &'static str,
    /// The rail's collapse control (`panel-left-close`) and expand control (`panel-left-open`).
    pub collapse: &'static str,
    pub expand: &'static str,
    /// A dialog's corner close (`x`).
    pub close: &'static str,
    pub sort_ascending: &'static str,
    pub sort_descending: &'static str,
    pub sortable: &'static str,
    pub status_dot: &'static str,
    /// A value that is not there (principle 10).
    pub missing: &'static str,
    pub check: &'static str,
    pub checkbox_off: &'static str,
    pub checkbox_on: &'static str,
    pub checkbox_mixed: &'static str,
    /// The active entry's bar, and keyboard focus at a row's start.
    pub focus_mark: &'static str,
    pub rule: &'static str,
    pub scroll_thumb: &'static str,
    pub scroll_track: &'static str,
    pub box_rounded: BoxGlyphs,
    pub box_square: BoxGlyphs,
    pub box_ascii: BoxGlyphs,
    /// The required star, a typed character.
    pub required: &'static str,
    /// The edge of a one-row control at its ends (`edge-start`, `edge-end`; T3).
    pub edge_start: &'static str,
    pub edge_end: &'static str,
    /// Keyboard focus on a one-row control, in the cell beside each end (`focus-ring-start`,
    /// `focus-ring-end`; T4).
    pub ring_start: &'static str,
    pub ring_end: &'static str,
    /// A control's ends where no color can show it: typed brackets.
    pub bare_start: &'static str,
    pub bare_end: &'static str,
    /// A critical box's top rule (`rule-heavy`).
    pub heavy_rule: &'static str,
    /// One dot of the graph tone's dot grid (`grid-dot`).
    pub dot: &'static str,
    /// sokkel-tui's own until glyphs.json holds them (proposal c4): a sparkline's eight levels,
    /// lowest first, and a panel's stale mark.
    pub spark: [&'static str; 8],
    pub stale: &'static str,
    /// The arrows of a change, up and down (`arrow-up`, `arrow-down`).
    pub arrow_up: &'static str,
    pub arrow_down: &'static str,
    /// A banner's mark in the info tone (`info`).
    pub info: &'static str,
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

impl Glyphs {
    fn read(charset: Charset) -> Glyphs {
        let file = tokens::glyphs();
        let form = match charset {
            Charset::Unicode => "unicode",
            Charset::Ascii => "ascii",
        };
        let pick = |v: &Value, name: &str| -> &'static str {
            leak(
                v.get(form)
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| panic!("glyphs.json has no {form} form for {name}")),
            )
        };
        let glyph = |name: &str| pick(&file["glyphs"][name], name);
        let icon = |name: &str| pick(&file["icons"][name], name);
        let boxed = |set: &str| {
            let b = &file["box"][set];
            let g = |k: &str| {
                leak(
                    b[k].as_str()
                        .unwrap_or_else(|| panic!("glyphs.json box {set} has no {k}")),
                )
            };
            BoxGlyphs {
                top_start: g("topStart"),
                top_end: g("topEnd"),
                bottom_start: g("bottomStart"),
                bottom_end: g("bottomEnd"),
                across: g("across"),
                down: g("down"),
                tee_start: g("teeStart"),
                tee_end: g("teeEnd"),
            }
        };
        Glyphs {
            spinner: file["glyphs"]["spinner"][form]
                .as_array()
                .expect("glyphs.json has spinner frames")
                .iter()
                .map(|f| leak(f.as_str().unwrap_or("|")))
                .collect(),
            issue_critical: icon("circle-alert"),
            issue_warning: icon("triangle-alert"),
            ellipsis: glyph("ellipsis"),
            bullet: glyph("bullet"),
            retry: icon("rotate-cw"),
            back: icon("chevron-left"),
            more: icon("ellipsis"),
            chevron: glyph("chevron-right"),
            chevron_down: glyph("chevron-down"),
            menu: icon("menu"),
            search: icon("search"),
            collapse: icon("panel-left-close"),
            expand: icon("panel-left-open"),
            close: icon("x"),
            sort_ascending: glyph("sort-ascending"),
            sort_descending: glyph("sort-descending"),
            sortable: glyph("sortable"),
            status_dot: glyph("status-dot"),
            missing: glyph("missing"),
            check: glyph("check"),
            checkbox_off: glyph("checkbox-off"),
            checkbox_on: glyph("checkbox-on"),
            checkbox_mixed: glyph("checkbox-mixed"),
            focus_mark: glyph("focus-mark"),
            rule: glyph("rule"),
            scroll_thumb: glyph("scroll-thumb"),
            scroll_track: glyph("scroll-track"),
            box_rounded: boxed("rounded"),
            box_square: boxed("square"),
            box_ascii: boxed("ascii"),
            required: "*",
            edge_start: glyph("edge-start"),
            edge_end: glyph("edge-end"),
            ring_start: glyph("focus-ring-start"),
            ring_end: glyph("focus-ring-end"),
            bare_start: "[",
            bare_end: "]",
            heavy_rule: glyph("rule-heavy"),
            dot: glyph("grid-dot"),
            spark: if charset == Charset::Unicode {
                ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"]
            } else {
                ["_", ".", "-", "=", "+", "*", "#", "@"]
            },
            stale: if charset == Charset::Unicode { "◷" } else { "!" },
            arrow_up: icon("arrow-up"),
            arrow_down: icon("arrow-down"),
            info: icon("info"),
        }
    }
}
