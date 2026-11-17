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
