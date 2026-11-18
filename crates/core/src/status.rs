//! Status: a state said by its dot and its word, the same in a table cell, a page header's lead
//! and a detail pane's fact. The colors come from the contract's `statuses` (index.json): a run's
//! states by their own colors, a health check's (`ok`, `warn`, `fail`, `error`) by its tone, a
//! state no map knows in `neutral`. An app names its own states with a word and a tone, which win.
//! Its word reads without color beside the dot (principle 22).

use crate::look::{Look, Rgba};
use crate::table::StatusTone;
use crate::tokens;

/// A state as a status part says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    words: String,
    key: String,
    tone: Option<StatusTone>,
}

impl Status {
    /// A state the contract's maps know: `failed`, `running`, `ok`, `warn`, … (a hyphen or an
    /// underscore alike); its words are its name, underscores as spaces.
    pub fn of(state: impl Into<String>) -> Status {
        let state = state.into();
        Status {
            words: state.replace('_', " "),
            key: state.to_lowercase().replace('-', "_"),
            tone: None,
        }
    }

    /// An app's own state: its words in a tone, over any map.
    pub fn toned(words: impl Into<String>, tone: StatusTone) -> Status {
        let words = words.into();
        Status {
            key: words.to_lowercase(),
            words,
            tone: Some(tone),
        }
    }

    pub fn words(&self) -> &str {
        &self.words
    }

    pub fn tone(&self) -> Option<StatusTone> {
        self.tone
    }

    /// The dot's color and the word's ink.
    pub fn colors(&self, look: &Look) -> (Rgba, Rgba) {
        if let Some(tone) = self.tone {
            let (dot, ink) = tone.tokens();
            return (look.color(dot), look.color(ink));
        }
        let map = &tokens::index()["statuses"];
        let name = |v: &serde_json::Value| v.as_str().map(str::to_string);
        // A run's state names its own color; a health state its tone's; else as named, or neutral.
        let key = name(&map["run"][&self.key])
            .or_else(|| name(&map["health"][&self.key]))
            .filter(|k| map["colors"].get(k).is_some())
            .or_else(|| map["colors"].get(&self.key).map(|_| self.key.clone()))
            .unwrap_or_else(|| "neutral".into());
        let entry = &map["colors"][&key];
        let token = |field: &str, or: &str| {
            let t = entry[field].as_str().unwrap_or(or);
            look.color(if look.has_color(t) { t } else { or })
        };
        (token("color", "faint"), token("ink", "muted-foreground"))
    }
}
