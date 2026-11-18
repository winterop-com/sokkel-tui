//! Focus: which stop has it, how it was reached, and the layers that hold it.
//!
//! Immediate mode, as the parts are: each frame the app names its stops in tab order while it
//! draws ([`Focus::stop`]), and the keys between two frames move along the order the last frame
//! named. A layer (a dialog, the palette, a menu, the drawer) keeps Tab inside it while it is open
//! and gives focus back to what opened it when it closes (the shell pattern, point 5).
//!
//! The focus ring shows on keyboard focus only (`on: keyboard`). Focus reached by a press of the
//! pointer is focus without a ring; the next key brings the ring back.

use ratatui::layout::{Position, Rect};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A stop's name, stable from frame to frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(u64);

impl FocusId {
    pub fn new(name: &str) -> FocusId {
        let mut h = DefaultHasher::new();
        name.hash(&mut h);
        FocusId(h.finish())
    }

    /// A stop inside this one: a row of a table, a field's issue mark.
    pub fn child(self, n: u64) -> FocusId {
        let mut h = DefaultHasher::new();
        (self.0, n).hash(&mut h);
        FocusId(h.finish())
    }
}

/// How focus was last moved: the ring shows for the keyboard only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputKind {
    #[default]
    Keyboard,
    Pointer,
}

/// What a stop takes: keys of its own (a button, a row) or typed text (a field), where the app's
/// keys keep quiet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Control,
    Text,
    /// A control whose arrows it leaves go on to the frame: Left and Right then go back and
    /// forward along the routes (a table with no use for them).
    Passing,
}

#[derive(Debug, Clone, Copy)]
struct Stop {
    id: FocusId,
    layer: usize,
    area: Rect,
    kind: StopKind,
}

#[derive(Debug, Clone, Copy)]
struct Layer {
    id: FocusId,
    opener: Option<FocusId>,
}

/// Focus across frames.
#[derive(Debug, Clone, Default)]
pub struct Focus {
    current: Option<FocusId>,
    kind: InputKind,
    /// The stops the last whole frame named, in tab order.
    stops: Vec<Stop>,
    /// The stops this frame has named so far.
    next: Vec<Stop>,
    /// The layers open now, the top last.
    layers: Vec<Layer>,
    /// The layers this frame is drawing into, by their place in `layers` (0 is the page).
    drawing: Vec<usize>,
}

impl Focus {
    pub fn new() -> Focus {
        Focus::default()
    }

    /// Start naming this frame's stops.
    pub fn begin(&mut self) {
        self.next.clear();
        self.drawing.clear();
    }

    /// Done naming: this frame's order is the one keys move along. A focused stop that was not
    /// drawn this frame falls to the first stop of the top layer.
    pub fn end(&mut self) {
        self.stops = std::mem::take(&mut self.next);
        let top = self.layers.len();
        if let Some(cur) = self.current
            && !self.stops.iter().any(|s| s.id == cur && s.layer == top)
        {
            self.current = self.stops.iter().find(|s| s.layer == top).map(|s| s.id);
        }
    }

    /// Name a stop that takes keys of its own, where it is drawn; true when it has focus.
    pub fn stop(&mut self, id: FocusId, area: Rect) -> bool {
        self.add(id, area, StopKind::Control)
    }

    /// Name a stop whose Left and Right, when it leaves them, go on along the routes (a table:
    /// unless it edits, holds a tree or a detail, or has a head chosen); true when it has focus.
    pub fn stop_passing(&mut self, id: FocusId, area: Rect) -> bool {
        self.add(id, area, StopKind::Passing)
    }

    /// Whether the focused stop lets the arrows it leaves go on.
    pub fn passes_arrows(&self) -> bool {
        self.current
            .is_some_and(|c| self.stops.iter().any(|s| s.id == c && s.kind == StopKind::Passing))
    }

    /// Name a stop that takes typed text.
    pub fn text_stop(&mut self, id: FocusId, area: Rect) -> bool {
        self.add(id, area, StopKind::Text)
    }

    fn add(&mut self, id: FocusId, area: Rect, kind: StopKind) -> bool {
        let layer = self.drawing.last().copied().unwrap_or(0);
        self.next.push(Stop { id, layer, area, kind });
        self.is(id)
    }

    /// Draw the stops that follow into an open layer, until [`Focus::leave_layer`].
    pub fn enter_layer(&mut self, layer: FocusId) {
        if let Some(i) = self.layers.iter().position(|l| l.id == layer) {
            self.drawing.push(i + 1);
        }
    }

    pub fn leave_layer(&mut self) {
        self.drawing.pop();
    }

    /// Open a layer over the page: Tab stays inside it, and `first` takes focus (or the layer's
    /// first stop, once drawn). What had focus gets it back on close.
    pub fn open_layer(&mut self, layer: FocusId, first: Option<FocusId>) {
        self.layers.push(Layer {
            id: layer,
            opener: self.current,
        });
        self.current = first;
    }

    /// Close the top layer; focus goes back to what opened it. Returns the layer closed.
    pub fn close_layer(&mut self) -> Option<FocusId> {
        let layer = self.layers.pop()?;
        self.current = layer.opener;
        Some(layer.id)
    }

    /// Whether a layer is open over the page: the app's keys keep quiet under it.
    pub fn under_layer(&self) -> bool {
        !self.layers.is_empty()
    }

    pub fn top_layer(&self) -> Option<FocusId> {
        self.layers.last().map(|l| l.id)
    }

    pub fn current(&self) -> Option<FocusId> {
        self.current
    }

    pub fn is(&self, id: FocusId) -> bool {
        self.current == Some(id)
    }

    /// Whether `id` has focus reached by the keyboard: the ring shows.
    pub fn ring(&self, id: FocusId) -> bool {
        self.is(id) && self.kind == InputKind::Keyboard
    }

    pub fn input_kind(&self) -> InputKind {
        self.kind
    }

    /// Whether the focused stop takes typed text: the app's plain keys keep quiet.
    pub fn typing(&self) -> bool {
        self.current
            .is_some_and(|c| self.stops.iter().any(|s| s.id == c && s.kind == StopKind::Text))
    }

    /// Where a stop was drawn in the last finished frame.
    pub fn area_of(&self, id: FocusId) -> Option<Rect> {
        self.stops.iter().find(|s| s.id == id).map(|s| s.area)
    }

    /// How many stops this frame has named so far: a mark to find the first one named after it.
    pub fn mark(&self) -> usize {
        self.next.len()
    }

    /// The first stop named after `mark` in the last finished frame, in the top layer.
    pub fn first_after(&self, mark: usize) -> Option<FocusId> {
        let top = self.layers.len();
        self.stops.iter().skip(mark).find(|s| s.layer == top).map(|s| s.id)
    }

    /// Give focus to a stop, as the app does after a route change or an action.
    pub fn set(&mut self, id: FocusId) {
        self.current = Some(id);
    }

    /// A key came: the ring shows again. Call it for every key.
    pub fn saw_key(&mut self) {
        self.kind = InputKind::Keyboard;
    }

    /// A press of the pointer at `at`: the stop under it takes focus, without a ring.
    pub fn press(&mut self, at: Position) -> Option<FocusId> {
        self.kind = InputKind::Pointer;
        let top = self.layers.len();
        let hit = self
            .stops
            .iter()
            .rev()
            .find(|s| s.layer == top && s.area.contains(at))
            .map(|s| s.id)?;
        self.current = Some(hit);
        Some(hit)
    }

    /// Tab: the next stop in the top layer, round to the first.
    pub fn forward(&mut self) -> Option<FocusId> {
        self.step(1)
    }

    /// Shift+Tab: the stop before.
    pub fn back(&mut self) -> Option<FocusId> {
        self.step(-1)
    }

    fn step(&mut self, by: isize) -> Option<FocusId> {
        self.kind = InputKind::Keyboard;
        let top = self.layers.len();
        let order: Vec<FocusId> = self.stops.iter().filter(|s| s.layer == top).map(|s| s.id).collect();
        if order.is_empty() {
            return None;
        }
        let n = order.len() as isize;
        let at = self.current.and_then(|c| order.iter().position(|&o| o == c));
        let to = match at {
            Some(i) => (i as isize + by).rem_euclid(n) as usize,
            None if by > 0 => 0,
            None => order.len() - 1,
        };
        self.current = Some(order[to]);
        self.current
    }

    /// Tab and Shift+Tab. True when the key was taken. Escape is the layer's own: it closes what
    /// the app drew, then [`Focus::close_layer`].
    pub fn handle(&mut self, key: crate::keys::KeyPress) -> bool {
        use crate::keys::Key;
        self.saw_key();
        if key.is(Key::Tab) {
            self.forward();
            true
        } else if key.is(Key::BackTab) {
            self.back();
            true
        } else {
            false
        }
    }
}
