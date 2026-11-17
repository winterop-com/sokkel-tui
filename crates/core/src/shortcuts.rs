//! The shortcuts registry: the app's keys, matched by the character they type (principle 17).
//!
//! A shortcut is a typed character (`?`, `/`), Mod and a character (Mod is Ctrl in a terminal:
//! `Ctrl K`), or a sequence of two characters (G then D), the second within a second. Keys keep
//! quiet while the reader types in a field and while a layer is open, unless the binding asks.
//! No shortcut is an Alt chord, and none answers one.

use crate::keys::{Key, KeyPress};

/// How long a sequence waits for its second key: "within a second" (the shell pattern).
pub const SEQUENCE_MS: u64 = 1000;

/// The keys of one shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keys {
    /// A typed character, Shift already in it: `?` is `?` whatever keys typed it.
    Char(char),
    /// Mod and a character: Ctrl in a terminal.
    Mod(char),
    /// One typed character, then another.
    Then(char, char),
}

impl Keys {
    /// How the shortcut is written, in the words given (`Ctrl K`, `?`, `G then D`).
    pub fn label(&self, words: &ShortcutWords) -> String {
        let key = |c: char| match c {
            ' ' => words.space.clone(),
            c => c.to_uppercase().to_string(),
        };
        match *self {
            Keys::Char(c) => key(c),
            Keys::Mod(c) => format!("{} {}", words.mod_key, key(c)),
            Keys::Then(a, b) => format!("{} {} {}", key(a), words.then, key(b)),
        }
    }
}

/// The words a shortcut's label is written in, with English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutWords {
    /// Mod in a terminal.
    pub mod_key: String,
    pub then: String,
    pub space: String,
}

impl Default for ShortcutWords {
    fn default() -> ShortcutWords {
        ShortcutWords {
            mod_key: "Ctrl".into(),
            then: "then".into(),
            space: "Space".into(),
        }
    }
}

/// One shortcut: its keys, what it does, the words the shortcuts dialog says for it, and whether
/// it is heard in a field or under a layer.
#[derive(Debug, Clone)]
pub struct Binding<A> {
    pub keys: Keys,
    pub action: A,
    pub does: String,
    pub in_field: bool,
    pub under_layer: bool,
}

impl<A> Binding<A> {
    pub fn new(keys: Keys, action: A, does: impl Into<String>) -> Binding<A> {
        Binding {
            keys,
            action,
            does: does.into(),
            in_field: false,
            under_layer: false,
        }
    }

    /// Heard while the reader types in a field too.
    pub fn in_field(mut self) -> Self {
        self.in_field = true;
        self
    }

    /// Heard while a layer is open too: the palette's own key, which closes it again.
    pub fn under_layer(mut self) -> Self {
        self.under_layer = true;
        self
    }
}

/// Where a key landed: in a field, under a layer, and when.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyContext {
    pub typing: bool,
    pub under_layer: bool,
    pub now_ms: u64,
}

/// The app's shortcuts.
#[derive(Debug, Clone)]
pub struct Shortcuts<A> {
    bindings: Vec<Binding<A>>,
    /// The first key of a sequence, and when it came.
    pending: Option<(char, u64)>,
}

impl<A> Default for Shortcuts<A> {
    fn default() -> Self {
        Shortcuts {
            bindings: Vec::new(),
            pending: None,
        }
    }
}

impl<A: Clone> Shortcuts<A> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(mut self, binding: Binding<A>) -> Self {
        self.bindings.push(binding);
        self
    }

    pub fn bindings(&self) -> &[Binding<A>] {
        &self.bindings
    }

    /// The first key of a sequence is waiting for its second.
    pub fn pending(&self) -> Option<char> {
        self.pending.map(|(c, _)| c)
    }

    /// The action a key asks for, if any. A key that only starts a sequence is taken and asks
    /// for nothing yet.
    pub fn handle(&mut self, key: KeyPress, cx: KeyContext) -> Option<A> {
        if key.alt {
            self.pending = None;
            return None;
        }
        let heard = |b: &Binding<A>| (!cx.typing || b.in_field) && (!cx.under_layer || b.under_layer);
        let typed = match key.key {
            Key::Char(c) => c,
            _ => {
                self.pending = None;
                return None;
            }
        };

        if key.ctrl {
            self.pending = None;
            let c = typed.to_ascii_lowercase();
            return self
                .bindings
                .iter()
                .find(|b| b.keys == Keys::Mod(c) && heard(b))
                .map(|b| b.action.clone());
        }

        if let Some((first, at)) = self.pending.take()
            && cx.now_ms.saturating_sub(at) <= SEQUENCE_MS
            && let Some(b) = self
                .bindings
                .iter()
                .find(|b| b.keys == Keys::Then(first, typed) && heard(b))
        {
            return Some(b.action.clone());
        }
        if self
            .bindings
            .iter()
            .any(|b| matches!(b.keys, Keys::Then(a, _) if a == typed) && heard(b))
        {
            self.pending = Some((typed, cx.now_ms));
            return None;
        }
        self.bindings
            .iter()
            .find(|b| b.keys == Keys::Char(typed) && heard(b))
            .map(|b| b.action.clone())
    }
}
