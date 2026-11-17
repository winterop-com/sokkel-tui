//! sokkel-design's files, embedded: the resolved tokens, the index and the glyphs.
//!
//! `tokens/` holds sokkel-design's own files at the pinned commit, fetched by `make tokens` and never
//! edited here. `tokens.resolved.json` holds every theme in light and dark (colors as sRGB hex,
//! lengths in px, text roles), the density levels, and the resolved paints keyed
//! `<part>.<variant>.<state>.<piece>`, each with its opaque `flat` and `faded` forms.
//! `index.json` holds the cell grid (`cells`) and the color depths (`colorDepth`); `glyphs.json`
//! the glyphs a part draws in cells, in Unicode and ASCII.

use serde_json::Value;
use std::sync::OnceLock;

const RESOLVED: &str = include_str!("../tokens/tokens.resolved.json");
const INDEX: &str = include_str!("../tokens/index.json");
const GLYPHS: &str = include_str!("../tokens/glyphs.json");

/// sokkel-design's index, parsed once: `cells`, `colorDepth` and the rest.
pub fn index() -> &'static Value {
    static INDEX_JSON: OnceLock<Value> = OnceLock::new();
    INDEX_JSON.get_or_init(|| serde_json::from_str(INDEX).expect("index.json is sokkel-design's and parses"))
}

/// sokkel-design's glyphs, parsed once.
pub fn glyphs() -> &'static Value {
    static GLYPHS_JSON: OnceLock<Value> = OnceLock::new();
    GLYPHS_JSON.get_or_init(|| serde_json::from_str(GLYPHS).expect("glyphs.json is sokkel-design's and parses"))
}

/// The whole resolved file, parsed once.
pub fn resolved() -> &'static Value {
    static TOKENS: OnceLock<Value> = OnceLock::new();
    TOKENS.get_or_init(|| serde_json::from_str(RESOLVED).expect("tokens.resolved.json is sokkel-design's and parses"))
}

/// Light or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scheme {
    Light,
    #[default]
    Dark,
}

impl Scheme {
    pub fn key(self) -> &'static str {
        match self {
            Scheme::Light => "light",
            Scheme::Dark => "dark",
        }
    }
}

/// The two density levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    #[default]
    Regular,
    Compact,
}

impl Density {
    pub fn key(self) -> &'static str {
        match self {
            Density::Regular => "regular",
            Density::Compact => "compact",
        }
    }
}

/// Every theme the contract resolves, in its order.
pub fn themes() -> Vec<&'static str> {
    resolved()["themes"]
        .as_object()
        .map(|m| m.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

/// The theme a sokkel app starts in.
pub fn default_theme() -> &'static str {
    resolved()["default"]["theme"].as_str().unwrap_or("tidevann")
}

/// The sokkel-ui commit the resolved file was made from.
pub fn sokkel_ui_commit() -> &'static str {
    resolved()["sokkelUi"]["commit"].as_str().unwrap_or("")
}
