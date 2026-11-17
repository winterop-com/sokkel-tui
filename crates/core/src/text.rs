//! Text by role (principle 3), in a terminal.
//!
//! A terminal has one size. A role keeps what a cell can show of it: its weight, as bold from
//! 500 up. Size, line height and tracking are not drawn. The ink is the part's, from its paint.
//! Until the contract holds a role-to-cell table, this rule is it.

use crate::look::Look;
use ratatui::style::{Modifier, Style};

/// The roles the parts set their words in, by their resolved names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextRole {
    PageTitle,
    DetailTitle,
    SectionTitle,
    Head,
    Body,
    Action,
    Label,
    Help,
    Badge,
    Statistic,
    Eyebrow,
}

impl TextRole {
    /// The role's name in the resolved file (`text.<name>`).
    pub fn name(self) -> &'static str {
        match self {
            TextRole::PageTitle => "page-title",
            TextRole::DetailTitle => "detail-title",
            TextRole::SectionTitle => "section-title",
            TextRole::Head => "head",
            TextRole::Body => "body",
            TextRole::Action => "action",
            TextRole::Label => "label",
            TextRole::Help => "help",
            TextRole::Badge => "badge",
            TextRole::Statistic => "statistic",
            TextRole::Eyebrow => "eyebrow",
        }
    }

    /// The role's resolved weight in this look's theme.
    pub fn weight(self, look: &Look) -> u64 {
        look.text_role(self.name())["weight"].as_u64().unwrap_or(400)
    }

    /// What a cell shows of the role: bold from weight 500 up.
    pub fn style(self, look: &Look) -> Style {
        if self.weight(look) >= 500 {
            Style::new().add_modifier(Modifier::BOLD)
        } else {
            Style::new()
        }
    }
}

/// The cells a piece of text takes, measured: a wide glyph takes two (proposal, (e) 5).
pub fn width(s: &str) -> usize {
    ratatui::text::Span::raw(s).width()
}

/// The cells one character takes.
pub fn char_width(c: char) -> usize {
    let mut b = [0u8; 4];
    width(c.encode_utf8(&mut b))
}

/// `s` cut to `max` cells, ending in `ellipsis` when it was cut.
pub fn truncate(s: &str, max: usize, ellipsis: &str) -> String {
    if width(s) <= max {
        return s.to_string();
    }
    let room = max.saturating_sub(width(ellipsis));
    let mut out = String::new();
    let mut used = 0;
    for c in s.chars() {
        let w = char_width(c);
        if used + w > room {
            break;
        }
        out.push(c);
        used += w;
    }
    if max >= width(ellipsis) {
        out += ellipsis;
    }
    out
}

/// `s` in lines of at most `max` cells, broken between words; a word longer than a line is
/// broken where it reaches the end.
pub fn wrap(s: &str, max: usize) -> Vec<String> {
    let max = max.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let mut word = word.to_string();
        loop {
            let lw = width(&line);
            let ww = width(&word);
            let sep = usize::from(lw > 0);
            if lw + sep + ww <= max {
                if sep == 1 {
                    line.push(' ');
                }
                line += &word;
                break;
            }
            if lw > 0 {
                lines.push(std::mem::take(&mut line));
                continue;
            }
            // A word longer than the line: as much as fits, the rest on the next line.
            let mut head = String::new();
            let mut used = 0;
            let mut rest = word.chars().peekable();
            while let Some(&c) = rest.peek() {
                if used + char_width(c) > max {
                    break;
                }
                used += char_width(c);
                head.push(c);
                rest.next();
            }
            lines.push(head);
            word = rest.collect();
            if word.is_empty() {
                break;
            }
        }
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}
