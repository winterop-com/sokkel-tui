//! Keys as the parts read them.
//!
//! A key is matched by the character it types, never by its place on the keyboard, so a layout
//! that types `/` with Shift still reaches what `/` does. Mod is Ctrl in a terminal: Cmd almost
//! never reaches a terminal program. No part answers an Alt chord.

/// A key a part can answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// A typed character, as typed (Shift already applied).
    Char(char),
    Enter,
    Escape,
    Tab,
    /// Shift+Tab.
    BackTab,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
}

/// A key and whether Ctrl (Mod) was held with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub ctrl: bool,
    /// Alt was held. Parts ignore such a press: with Option as Meta, Alt chords fight keyboards
    /// that type their brackets with Option.
    pub alt: bool,
}

impl KeyPress {
    pub fn new(key: Key) -> KeyPress {
        KeyPress {
            key,
            ctrl: false,
            alt: false,
        }
    }

    pub fn ctrl(key: Key) -> KeyPress {
        KeyPress {
            key,
            ctrl: true,
            alt: false,
        }
    }

    /// A plain press of `key`: no Ctrl, no Alt.
    pub fn is(&self, key: Key) -> bool {
        self.key == key && !self.ctrl && !self.alt
    }

    /// Enter or Space, unmodified: what presses a button.
    pub fn is_press(&self) -> bool {
        self.is(Key::Enter) || self.is(Key::Char(' '))
    }
}

impl From<Key> for KeyPress {
    fn from(key: Key) -> KeyPress {
        KeyPress::new(key)
    }
}

impl KeyPress {
    /// A crossterm key event as the parts read it; `None` for a release, a repeat the terminal
    /// reports apart, or a key no part answers.
    pub fn from_crossterm(ev: &ratatui::crossterm::event::KeyEvent) -> Option<KeyPress> {
        use ratatui::crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
        if ev.kind == KeyEventKind::Release {
            return None;
        }
        let key = match ev.code {
            KeyCode::Char(c) => Key::Char(c),
            KeyCode::Enter => Key::Enter,
            KeyCode::Esc => Key::Escape,
            KeyCode::Tab => Key::Tab,
            KeyCode::BackTab => Key::BackTab,
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Delete => Key::Delete,
            KeyCode::Left => Key::Left,
            KeyCode::Right => Key::Right,
            KeyCode::Up => Key::Up,
            KeyCode::Down => Key::Down,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            _ => return None,
        };
        Some(KeyPress {
            key,
            ctrl: ev.modifiers.contains(KeyModifiers::CONTROL),
            alt: ev.modifiers.contains(KeyModifiers::ALT),
        })
    }
}
