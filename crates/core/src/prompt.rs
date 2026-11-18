//! Prompt: a dialog that asks for one value (a token, a name) in a field, then Submit or Cancel.
//! It is parts/dialog with parts/text-field in its body, put together so an app does not wire the
//! pair itself: Enter submits from the field or Submit, Escape cancels, Tab moves between the field
//! and the verbs, a paste lands in the field, and the pointer works on the field and the verbs.
//! The field can be masked ([`TextField::masked`]).
//!
//! It follows sokkel-design's `patterns/prompt`: Submit is shut while the field is empty (unless
//! [`Prompt::allow_empty`]) or the app refuses the value ([`Prompt::refuse`], shown once something
//! is typed or Submit is tried); busy, the field is held and Submit spins saying "Submitting",
//! Escape waiting until `busy-release`; a failure is said in the dialog's status row, the value
//! kept, Submit trying again.

use crate::cells::Width;
use crate::dialog::{Confirm, Dialog, DialogEvent, DialogFocus, DialogWidth, DialogWords};
use crate::keys::{Key, KeyPress};
use crate::look::Look;
use crate::text_field::{FieldEvent, FieldFocus, FieldState, Issue, TextField};
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
    /// Something was typed or Submit was tried: the app's refusal shows on the field.
    pub touched: bool,
}

/// The words a prompt says, the pattern's English defaults (`promptDialog.*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptWords {
    pub submit: String,
    pub submitting: String,
    pub cancel: String,
    pub empty: String,
    pub failed: String,
}

impl Default for PromptWords {
    fn default() -> Self {
        PromptWords {
            submit: "Submit".into(),
            submitting: "Submitting".into(),
            cancel: "Cancel".into(),
            empty: "Fill in the field first.".into(),
            failed: "Could not submit. Try again.".into(),
        }
    }
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
    submit: Option<String>,
    window: Width,
    words: PromptWords,
    allow_empty: bool,
    refusal: Option<String>,
    busy: Option<u64>,
    failed: Option<Option<String>>,
}

impl<'a> Prompt<'a> {
    /// A prompt titled `title`, its field labelled `label`.
    pub fn new(look: &'a Look, title: impl Into<String>, label: impl Into<String>) -> Prompt<'a> {
        Prompt {
            look,
            title: title.into(),
            description: None,
            field: TextField::new(look, label),
            submit: None,
            window: Width::Wide,
            words: PromptWords::default(),
            allow_empty: false,
            refusal: None,
            busy: None,
            failed: None,
        }
    }

    pub fn words(mut self, words: PromptWords) -> Self {
        self.words = words;
        self
    }

    /// An empty value may be submitted; without it Submit is shut while the field is empty.
    pub fn allow_empty(mut self, on: bool) -> Self {
        self.allow_empty = on;
        self
    }

    /// The app's check refuses the value, saying why: the field shows it as a critical issue
    /// once something is typed or Submit is tried, and Submit is shut with it as its reason.
    pub fn refuse(mut self, why: Option<impl Into<String>>) -> Self {
        self.refusal = why.map(Into::into);
        self
    }

    /// The value is being taken, for `elapsed_ms` so far: the field held, Submit spinning and
    /// saying "Submitting", Escape and Cancel waiting until `busy-release`.
    pub fn busy(mut self, elapsed_ms: Option<u64>) -> Self {
        self.busy = elapsed_ms;
        self
    }

    /// Taking the value failed: the status row says why (`None`: the pattern's words), the
    /// value kept, Submit trying again.
    pub fn failed(mut self, why: Option<Option<String>>) -> Self {
        self.failed = why;
        self
    }

    /// The field as drawn: held while busy, showing the app's refusal once touched.
    fn shown(&self, state: &PromptState) -> TextField<'a> {
        let mut f = self.field.clone().busy(self.busy.is_some(), self.busy.unwrap_or(0));
        if let Some(why) = self.refusal.as_ref().filter(|_| state.touched) {
            f = f.issues([Issue::critical(why.clone())]);
        }
        f
    }

    /// Whether busy still holds the dialog: past `busy-release`, Escape and Cancel stop waiting.
    fn holding(&self) -> bool {
        self.busy.is_some_and(|ms| ms < self.look.duration_ms("busy-release"))
    }

    /// Why Submit is shut for `value`: the app's refusal, else an empty field; none while busy.
    fn shut(&self, value: &str) -> Option<String> {
        if self.busy.is_some() {
            return None;
        }
        self.refusal
            .clone()
            .or_else(|| (value.is_empty() && !self.allow_empty).then(|| self.words.empty.clone()))
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
        self.submit = Some(words.into());
        self
    }

    /// Kept so callers still compile: a prompt is a box at every width (the pattern's phone fit,
    /// as a confirm dialog), so the window's width no longer changes its form.
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
        let label = self.submit.clone().unwrap_or_else(|| self.words.submit.clone());
        let confirm = Confirm::new(label)
            .why(self.shut(state.field.value()))
            .busy(self.busy.is_some(), Some(self.words.submitting.clone()));
        let words = DialogWords {
            cancel: self.words.cancel.clone(),
            ..DialogWords::default()
        };
        let mut d = Dialog::new(self.look, self.title.clone())
            .width(DialogWidth::Sm)
            .confirm(confirm)
            .focus(focus)
            .words(words)
            .busy_for(self.busy.unwrap_or(0));
        // A box sized to its content at every width (the pattern's phone fit), never a sheet.
        let _ = self.window;
        if let Some(why) = &self.failed {
            d = d.status(why.clone().unwrap_or_else(|| self.words.failed.clone()), true);
        }
        if let Some(words) = &self.description {
            d = d.description(words.clone());
        }
        d
    }

    /// Where it stands in `window`.
    pub fn area(&self, window: Rect, state: &PromptState) -> Rect {
        let d = self.dialog(state);
        let body_w = d.area(window, 2).width.saturating_sub(4);
        d.area(window, self.shown(state).height(body_w))
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
        let field = self.shown(state);
        let h = field.height(body.width).min(body.height);
        StatefulWidget::render(&field, Rect { height: h, ..body }, buf, &mut state.field);
    }

    /// Answer a key: in the field it types, Enter submits; on a verb Enter presses it; Tab moves
    /// round the field and the verbs; Escape cancels.
    pub fn handle(&self, state: &mut PromptState, key: KeyPress) -> PromptEvent {
        match key.key {
            // Busy, Escape waits until busy-release; past it, it stops waiting.
            Key::Escape if self.holding() => return PromptEvent::Ignored,
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
        // Submit, unless it is shut (the refusal then shows) or busy.
        let submit = |state: &mut PromptState| {
            state.touched = true;
            if self.busy.is_some() || self.shut(state.field.value()).is_some() {
                PromptEvent::Changed
            } else {
                PromptEvent::Submit(state.field.value().to_string())
            }
        };
        match state.focus {
            PromptFocus::Field if key.is(Key::Enter) => submit(state),
            PromptFocus::Field => {
                state.field.focus = FieldFocus::Control;
                match state.field.handle(&self.shown(state), key) {
                    FieldEvent::Ignored => PromptEvent::Ignored,
                    _ => {
                        state.touched = true;
                        PromptEvent::Changed
                    }
                }
            }
            PromptFocus::Submit if key.is_press() => submit(state),
            PromptFocus::Cancel if key.is_press() && self.holding() => PromptEvent::Ignored,
            PromptFocus::Cancel if key.is_press() => PromptEvent::Cancel,
            _ => PromptEvent::Ignored,
        }
    }

    /// A paste lands in the field, whatever has the keys.
    pub fn paste(&self, state: &mut PromptState, text: &str) -> PromptEvent {
        state.focus = PromptFocus::Field;
        state.field.focus = FieldFocus::Control;
        state.touched = true;
        match state.field.paste(&self.shown(state), text) {
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
