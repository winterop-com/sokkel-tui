//! The glyphs the parts draw, each in a Unicode and an ASCII form.
//!
//! A terminal cannot choose its font, so every glyph a part draws has a form that any terminal
//! shows. These sets stand in for the contract's character sets until it holds them
//! (`docs/proposals/sokkel-design-tui.md`, point 5).

/// Which set of glyphs the terminal can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Charset {
    /// Box drawing, blocks, braille and arrows.
    #[default]
    Unicode,
    /// Printable ASCII only.
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
        match self {
            Charset::Unicode => &UNICODE,
            Charset::Ascii => &ASCII,
        }
    }
}

/// One set of glyphs. Each is one cell wide.
#[derive(Debug)]
pub struct Glyphs {
    /// A control's edge at its start and at its end, drawn in its pad cells.
    pub edge_start: &'static str,
    pub edge_end: &'static str,
    /// Keyboard focus on a control: the ring, in the cell outside each end.
    pub ring_start: &'static str,
    pub ring_end: &'static str,
    /// A control's ends where no color can show it (`NO_COLOR`).
    pub bare_start: &'static str,
    pub bare_end: &'static str,
    /// The busy spinner's frames, one per `motion-fast`.
    pub spinner: &'static [&'static str],
    /// The issue mark: a critical issue and a warning, told apart by shape as well as ink.
    pub issue_critical: &'static str,
    pub issue_warning: &'static str,
    /// The required star.
    pub required: &'static str,
    /// Where words are cut short.
    pub ellipsis: &'static str,
}

pub static UNICODE: Glyphs = Glyphs {
    edge_start: "▏",
    edge_end: "▕",
    ring_start: "┃",
    ring_end: "┃",
    bare_start: "[",
    bare_end: "]",
    spinner: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
    issue_critical: "✖",
    issue_warning: "▲",
    required: "*",
    ellipsis: "…",
};

pub static ASCII: Glyphs = Glyphs {
    edge_start: "|",
    edge_end: "|",
    ring_start: ">",
    ring_end: "<",
    bare_start: "[",
    bare_end: "]",
    spinner: &["|", "/", "-", "\\"],
    issue_critical: "x",
    issue_warning: "!",
    required: "*",
    ellipsis: "~",
};
