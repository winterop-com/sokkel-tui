//! sokkel-design's resolved tokens, embedded.
//!
//! `tokens/tokens.resolved.json` is sokkel-design's own file, copied in by `make tokens` and never
//! edited here. It holds every theme in light and dark (colors as sRGB hex, lengths in px, text
//! roles), the density levels, and the resolved paints keyed `<part>.<variant>.<state>.<piece>`.

use serde_json::Value;
use std::sync::OnceLock;

const RESOLVED: &str = include_str!("../tokens/tokens.resolved.json");

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
