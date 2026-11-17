//! Text by role (principle 3), in a terminal.
//!
//! A terminal has one size. A role keeps what `cells.textRoles` in sokkel-design's index gives
//! it: bold or not, italic or not. Size, line height and tracking are not drawn, and no role has an
//! ink of its own: the piece's paint gives it.

use crate::look::Look;
use crate::tokens::index;
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

    /// What a cell shows of the role (`cells.textRoles`): bold, italic.
    pub fn style(self, _look: &Look) -> Style {
        let cell = &index()["cells"]["textRoles"][self.name()];
        let mut st = Style::new();
        if cell["bold"].as_bool().unwrap_or(false) {
            st = st.add_modifier(Modifier::BOLD);
        }
        if cell["italic"].as_bool().unwrap_or(false) {
            st = st.add_modifier(Modifier::ITALIC);
        }
        st
    }

    /// The role's words as the theme sets them: `head` takes `table-head-transform` (upper case in
    /// some themes) where the part asks for it, as the table's heads do.
    pub fn transform(self, look: &Look, words: &str) -> String {
        if self == TextRole::Head && look.word("table-head-transform") == Some("uppercase") {
            words.to_uppercase()
        } else {
            words.to_string()
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
        // Cut at a space, the ellipsis follows the word.
        out.truncate(out.trim_end().len());
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

/// One piece of a line that may hold code: the words and whether they are code.
pub type Piece = (String, bool);

/// `s` in lines of at most `max` cells, its backticked parts kept whole as code (a written
/// diagnostic's `0 2 * * *`), broken between words elsewhere.
pub fn wrap_code(s: &str, max: usize) -> Vec<Vec<Piece>> {
    let max = max.max(1);
    // Atoms: a word, or a code part kept whole; `joined` when no space stood before it.
    let mut atoms: Vec<(String, bool, bool)> = Vec::new();
    for (i, seg) in s.split('`').enumerate() {
        let code = i % 2 == 1;
        if code {
            if !seg.is_empty() {
                let joined = !atoms.is_empty() && !s.split('`').nth(i - 1).unwrap_or("").ends_with(' ');
                atoms.push((seg.to_string(), true, joined));
            }
            continue;
        }
        let mut first = true;
        for w in seg.split(' ') {
            if w.is_empty() {
                first = false;
                continue;
            }
            let joined = first && !atoms.is_empty();
            atoms.push((w.to_string(), false, joined));
            first = false;
        }
    }
    let mut lines: Vec<Vec<Piece>> = Vec::new();
    let mut line: Vec<Piece> = Vec::new();
    let mut used = 0;
    for (words, code, joined) in atoms {
        let w = width(&words);
        let sep = usize::from(used > 0 && !joined);
        if used > 0 && used + sep + w > max {
            // An atom joined to the one before it (a period after code) takes that one along.
            let mut carry = Vec::new();
            if joined && line.len() > 1 {
                carry.push(line.pop().unwrap_or_default());
                if line.last().is_some_and(|(s, c)| !c && s == " ") {
                    line.pop();
                }
            }
            lines.push(std::mem::take(&mut line));
            used = carry.iter().map(|(s, _)| width(s)).sum();
            line = carry;
        }
        if used > 0 && !joined {
            line.push((" ".into(), false));
            used += 1;
        }
        used += w;
        line.push((words, code));
    }
    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

/// Words with a count, in English's two forms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plural {
    pub one: String,
    pub other: String,
}

impl Plural {
    pub fn new(one: &str, other: &str) -> Plural {
        Plural {
            one: one.into(),
            other: other.into(),
        }
    }

    /// The words for `count`, the count written in.
    pub fn say(&self, count: usize) -> String {
        let form = if count == 1 { &self.one } else { &self.other };
        form.replace("{count}", &count.to_string())
    }
}
