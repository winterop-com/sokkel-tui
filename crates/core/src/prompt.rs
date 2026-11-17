//! Prompt: a dialog that asks for one value (a token, a name) in a field, then Submit or Cancel.
//! It is parts/dialog with parts/text-field in its body, put together so an app does not wire the
//! pair itself: Enter submits from the field or Submit, Escape cancels, Tab moves between the field
//! and the verbs, a paste lands in the field, and the pointer works on the field and the verbs.
//! The field can be masked ([`TextField::masked`]).

use crate::cells::Width;
use crate::dialog::{Confirm, Dialog, DialogEvent, DialogFocus, DialogWidth};
use crate::keys::{Key, KeyPress};
use crate::look::Look;
use crate::text_field::{FieldEvent, FieldFocus, FieldState, TextField};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::{StatefulWidget, Widget};

/// Which of the prompt's stops has the keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptFocus {
    #[default]
    Field,
    Submit,
    Cancel,
}

/// What lives between frames: the field and which stop has the keys.
#[derive(Debug, Clone, Default)]
pub struct PromptState {
    pub field: FieldState,
    pub focus: PromptFocus,
}

/// What a key, a paste or a press did to the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptEvent {
    Ignored,
    Changed,
    /// Submitted, with the field's value: the app closes the prompt or keeps it open.
    Submit(String),
    Cancel,
}

/// A dialog asking for one value.
#[derive(Debug, Clone)]
pub struct Prompt<'a> {
    look: &'a Look,
    title: String,
    description: Option<String>,
    field: TextField<'a>,
    submit: String,
    window: Width,
}

impl<'a> Prompt<'a> {
    /// A prompt titled `title`, its field labelled `label`.
    pub fn new(look: &'a Look, title: impl Into<String>, label: impl Into<String>) -> Prompt<'a> {
        Prompt {
            look,
            title: title.into(),
            description: None,
            field: TextField::new(look, label),
            submit: "Submit".into(),
            window: Width::Wide,
        }
    }

    pub fn description(mut self, words: impl Into<String>) -> Self {
        self.description = Some(words.into());
        self
    }

    /// The field as the app wants it: masked, a placeholder, help, an issue.
    pub fn field(mut self, f: impl FnOnce(TextField<'a>) -> TextField<'a>) -> Self {
        self.field = f(self.field);
        self
    }

    /// The submit verb's words, in place of "Submit".
    pub fn submit(mut self, words: impl Into<String>) -> Self {
        self.submit = words.into();
        self
    }

    pub fn window(mut self, width: Width) -> Self {
        self.window = width;
        self
    }

    fn dialog(&self, state: &PromptState) -> Dialog<'a> {
        let focus = match state.focus {
            PromptFocus::Field => DialogFocus::None,
            PromptFocus::Submit => DialogFocus::Confirm,
            PromptFocus::Cancel => DialogFocus::Cancel,
        };
        let mut d = Dialog::new(self.look, self.title.clone())
            .width(DialogWidth::Sm)
            .confirm(Confirm::new(self.submit.clone()))
            .focus(focus)
            .window(self.window);
        if let Some(words) = &self.description {
            d = d.description(words.clone());
        }
        d
    }

    /// Where it stands in `window`.
    pub fn area(&self, window: Rect, state: &PromptState) -> Rect {
        let d = self.dialog(state);
        let body_w = d.area(window, 2).width.saturating_sub(4);
        d.area(window, self.field.height(body_w))
    }

    /// Draw it at `area`; the field's cursor is in `state.field` for the app to set.
    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &mut PromptState) {
        let d = self.dialog(state);
        (&d).render(area, buf);
        let body = d.layout(area).body;
        state.field.focus = if state.focus == PromptFocus::Field {
            FieldFocus::Control
        } else {
            FieldFocus::None
        };
        let h = self.field.height(body.width).min(body.height);
        StatefulWidget::render(&self.field, Rect { height: h, ..body }, buf, &mut state.field);
    }

    /// Answer a key: in the field it types, Enter submits; on a verb Enter presses it; Tab moves
    /// round the field and the verbs; Escape cancels.
    pub fn handle(&self, state: &mut PromptState, key: KeyPress) -> PromptEvent {
        match key.key {
            Key::Escape => return PromptEvent::Cancel,
            Key::Tab | Key::BackTab => {
                let order = [PromptFocus::Field, PromptFocus::Cancel, PromptFocus::Submit];
                let at = order.iter().position(|f| *f == state.focus).unwrap_or(0);
                let n = order.len();
                state.focus = order[if key.key == Key::Tab {
                    (at + 1) % n
                } else {
                    (at + n - 1) % n
                }];
                return PromptEvent::Changed;
            }
            _ => {}
        }
        match state.focus {
            PromptFocus::Field if key.is(Key::Enter) => PromptEvent::Submit(state.field.value().to_string()),
            PromptFocus::Field => {
                state.field.focus = FieldFocus::Control;
                match state.field.handle(&self.field, key) {
                    FieldEvent::Ignored => PromptEvent::Ignored,
                    _ => PromptEvent::Changed,
                }
            }
            PromptFocus::Submit if key.is_press() => PromptEvent::Submit(state.field.value().to_string()),
            PromptFocus::Cancel if key.is_press() => PromptEvent::Cancel,
            _ => PromptEvent::Ignored,
        }
    }

    /// A paste lands in the field, whatever has the keys.
    pub fn paste(&self, state: &mut PromptState, text: &str) -> PromptEvent {
        state.focus = PromptFocus::Field;
        state.field.focus = FieldFocus::Control;
        match state.field.paste(&self.field, text) {
            FieldEvent::Ignored => PromptEvent::Ignored,
            _ => PromptEvent::Changed,
        }
    }

    /// A press at `at` on the prompt drawn at `area`: the verbs act, the field takes the keys, a
    /// press outside cancels.
    pub fn press(&self, state: &mut PromptState, area: Rect, at: Position) -> PromptEvent {
        let d = self.dialog(state);
        let body = d.layout(area).body;
        if body.contains(at) {
            state.focus = PromptFocus::Field;
            return PromptEvent::Changed;
        }
        match d.press(area, at) {
            DialogEvent::Confirm => PromptEvent::Submit(state.field.value().to_string()),
            DialogEvent::Close => PromptEvent::Cancel,
            _ => PromptEvent::Ignored,
        }
    }
}
