//! A picture of a buffer, cell for cell: SVG for the eye and plain text for a diff.
//!
//! What lands in the file is what the buffer holds, so the picture cannot drift from the screen.
//! The SVG is for the side-by-side board; the text is what a test compares.

use crate::look::xterm_256;
use ratatui::buffer::Buffer;
use ratatui::style::{Color, Modifier};

/// One cell in SVG user units: what a 14 px monospace glyph occupies.
const CELL_W: f32 = 8.4;
const CELL_H: f32 = 18.0;
const BASELINE: f32 = 13.5;
const FONT: &str = "JetBrainsMonoNL Nerd Font Mono, JetBrains Mono, IBM Plex Mono, monospace";

fn hex(c: Color) -> Option<String> {
    match c {
        Color::Rgb(r, g, b) => Some(format!("#{r:02x}{g:02x}{b:02x}")),
        Color::Indexed(i) => {
            let c = xterm_256(i);
            Some(format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2))
        }
        _ => None,
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The buffer as SVG. `ground` and `ink` stand for the terminal's own colors, where a cell has none.
pub fn svg(buffer: &Buffer, ground: Color, ink: Color) -> String {
    let (w, h) = (buffer.area.width, buffer.area.height);
    let ground = hex(ground).unwrap_or_else(|| "#000000".into());
    let ink = hex(ink).unwrap_or_else(|| "#ffffff".into());
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" font-family=\"{FONT}\" font-size=\"14\">\n\
         <rect width=\"100%\" height=\"100%\" fill=\"{ground}\"/>\n",
        w as f32 * CELL_W,
        h as f32 * CELL_H,
    );
    for y in 0..h {
        for x in 0..w {
            let cell = &buffer[(x, y)];
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let bg = if reversed {
                hex(cell.fg).or(Some(ink.clone()))
            } else {
                hex(cell.bg)
            };
            if let Some(bg) = bg {
                out += &format!(
                    "<rect x=\"{:.2}\" y=\"{}\" width=\"{:.2}\" height=\"{}\" fill=\"{bg}\"/>",
                    x as f32 * CELL_W,
                    y as f32 * CELL_H,
                    CELL_W + 0.4,
                    CELL_H
                );
            }
        }
        out += "\n";
    }
    for y in 0..h {
        for x in 0..w {
            let cell = &buffer[(x, y)];
            let symbol = cell.symbol();
            if symbol.trim().is_empty() {
                continue;
            }
            let reversed = cell.modifier.contains(Modifier::REVERSED);
            let fg = if reversed {
                hex(cell.bg).unwrap_or(ground.clone())
            } else {
                hex(cell.fg).unwrap_or(ink.clone())
            };
            let mut attrs = String::new();
            if cell.modifier.contains(Modifier::BOLD) {
                attrs += " font-weight=\"700\"";
            }
            if cell.modifier.contains(Modifier::ITALIC) {
                attrs += " font-style=\"italic\"";
            }
            if cell.modifier.contains(Modifier::UNDERLINED) {
                attrs += " text-decoration=\"underline\"";
            }
            if cell.modifier.contains(Modifier::DIM) {
                attrs += " opacity=\"0.6\"";
            }
            out += &format!(
                "<text x=\"{:.2}\" y=\"{:.2}\" fill=\"{fg}\"{attrs}>{}</text>",
                x as f32 * CELL_W,
                y as f32 * CELL_H + BASELINE,
                escape(symbol)
            );
        }
        out += "\n";
    }
    out + "</svg>\n"
}

/// The buffer's symbols as lines of text, trailing spaces kept, so a test compares what is shown.
pub fn text(buffer: &Buffer) -> String {
    let mut out = String::new();
    for y in 0..buffer.area.height {
        let mut skip = 0;
        for x in 0..buffer.area.width {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let symbol = buffer[(x, y)].symbol();
            out += symbol;
            skip = unicode_width(symbol).saturating_sub(1);
        }
        out += "\n";
    }
    out
}

/// Wide glyphs take two cells; ratatui leaves the second one blank.
fn unicode_width(s: &str) -> usize {
    ratatui::text::Span::raw(s).width()
}
