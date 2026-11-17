//! TextField, from sokkel-design `parts/text-field`.
//!
//! A labeled text input: the field frame every field shares (label, hint, description, control,
//! help, fallback, issue mark) around a one-line input. A problem is said by a mark at the
//! control's end, never by a line inserted under it, so a form never changes height.
//!
//! In cells (the part's `cells` note) the label, the control and the help are one row each, with no
//! blank rows between them. The control's edge is drawn in its pad cells as a button's is. The
//! issue mark is one column, one from the control's end, and a stop of its own after the control;
//! the value's room ends three columns before the end while it shows. Its issues are said in the
//! status line while the field has focus ([`TextField::tip`]); the popover the mark opens is drawn
//! last, over what lies below, with [`TextField::popover`]. The terminal shows one face, so `mono`
//! and `numeric` draw as the rest do.

use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::text::{self, Plural, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, StatefulWidget, Widget};

/// How bad an issue is: critical stops the submit, a warning does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tone {
    Warning,
    Critical,
}

/// One issue, in sokkel-api's issue shape: its text, a detail, and the actions that put it right.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub tone: Tone,
    pub text: String,
    pub detail: Option<String>,
    pub actions: Vec<String>,
}

impl Issue {
    pub fn critical(text: impl Into<String>) -> Issue {
        Issue {
            tone: Tone::Critical,
            text: text.into(),
            detail: None,
            actions: Vec::new(),
        }
    }

    pub fn warning(text: impl Into<String>) -> Issue {
        Issue {
            tone: Tone::Warning,
            ..Issue::critical(text)
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Issue {
        self.detail = Some(detail.into());
        self
    }

    pub fn action(mut self, label: impl Into<String>) -> Issue {
        self.actions.push(label.into());
        self
    }
}

/// The words a field says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldWords {
    pub required: String,
    pub issue_critical: String,
    pub issue_warning: String,
    pub errors: Plural,
    pub warnings: Plural,
}

impl Default for FieldWords {
    fn default() -> FieldWords {
        FieldWords {
            required: "required".into(),
            issue_critical: "Error: ".into(),
            issue_warning: "Warning: ".into(),
            errors: Plural::new("{count} error", "{count} errors"),
            warnings: Plural::new("{count} warning", "{count} warnings"),
        }
    }
}

/// How required is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequiredMark {
    #[default]
    Star,
    Word,
}

/// Which of the field's stops has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FieldFocus {
    #[default]
    None,
    Control,
    /// The issue mark, the stop after the control while an issue shows.
    Mark,
}

/// What a key did to the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldEvent {
    /// The field does not answer this key; the app may.
    Ignored,
    /// The cursor moved.
    Moved,
    /// The value changed.
    Edited,
    /// Enter in the control: the app's `onEnter`.
    Enter,
    /// Escape in the control: the app's `onEscape`.
    Escape,
    IssuesOpened,
    IssuesClosed,
}

/// The value, the cursor and the focus: what lives between frames.
#[derive(Debug, Clone, Default)]
pub struct FieldState {
    value: String,
    /// The cursor, a byte offset on a char boundary.
    cursor: usize,
    /// The first column of the value the control shows.
    scroll: usize,
    pub focus: FieldFocus,
    pub issue_open: bool,
    cursor_at: Option<Position>,
    mark_at: Option<Rect>,
    /// A text area's first line in view, and the columns its lines wrap at (from the last frame).
    top: usize,
    wrap: usize,
}

/// A text area's lines as drawn: each a byte range of the value, a paragraph broken where it
/// reaches `width` columns. A line ends before its newline.
fn visual_lines(value: &str, width: usize) -> Vec<(usize, usize)> {
    let width = width.max(1);
    let mut out = Vec::new();
    let mut start = 0;
    let mut used = 0;
    for (i, c) in value.char_indices() {
        if c == '\n' {
            out.push((start, i));
            start = i + 1;
            used = 0;
            continue;
        }
        let w = text::char_width(c);
        if used + w > width {
            out.push((start, i));
            start = i;
            used = 0;
        }
        used += w;
    }
    out.push((start, value.len()));
    out
}

/// The line the cursor at byte `at` stands on: at a wrapped line's end it is the next line's start.
fn line_of(lines: &[(usize, usize)], at: usize) -> usize {
    let mut found = 0;
    for (k, &(a, b)) in lines.iter().enumerate() {
        if at >= a && (at < b || (at == b && lines.get(k + 1).is_none_or(|n| n.0 > b))) {
            return k;
        }
        if at >= a {
            found = k;
        }
    }
    found
}

impl FieldState {
    /// A state holding `value`, the cursor at its end.
    pub fn new(value: impl Into<String>) -> FieldState {
        let value = value.into();
        FieldState {
            cursor: value.len(),
            value,
            ..FieldState::default()
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// The cursor, a byte offset into the value on a char boundary.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.cursor = self.value.len();
    }

    pub fn focused(mut self, focus: FieldFocus) -> FieldState {
        self.focus = focus;
        self
    }

    /// The issue popover open (held from outside, as `issueOpen` is).
    pub fn issue_open(mut self, open: bool) -> FieldState {
        self.issue_open = open;
        self
    }

    /// Where the terminal's cursor goes after the field is drawn, while the control has focus.
    pub fn cursor_position(&self) -> Option<Position> {
        self.cursor_at
    }

    /// Where the issue mark was drawn, while an issue shows.
    pub fn mark_area(&self) -> Option<Rect> {
        self.mark_at
    }

    fn prev_boundary(&self) -> usize {
        self.value[..self.cursor].char_indices().last().map_or(0, |(i, _)| i)
    }

    fn next_boundary(&self) -> usize {
        self.value[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |c| self.cursor + c.len_utf8())
    }

    /// Answer a key the field's focused stop received.
    pub fn handle(&mut self, field: &TextField, key: KeyPress) -> FieldEvent {
        if key.alt {
            return FieldEvent::Ignored;
        }
        if self.issue_open && key.is(Key::Escape) {
            self.issue_open = false;
            return FieldEvent::IssuesClosed;
        }
        match self.focus {
            FieldFocus::None => FieldEvent::Ignored,
            FieldFocus::Mark if key.is_press() && field.has_issues() => {
                self.issue_open = !self.issue_open;
                if self.issue_open {
                    FieldEvent::IssuesOpened
                } else {
                    FieldEvent::IssuesClosed
                }
            }
            FieldFocus::Mark => FieldEvent::Ignored,
            FieldFocus::Control if field.disabled || key.ctrl => FieldEvent::Ignored,
            FieldFocus::Control if field.multiline.is_some() => self.handle_area(field, key),
            FieldFocus::Control => match key.key {
                Key::Enter => FieldEvent::Enter,
                Key::Escape => FieldEvent::Escape,
                Key::Left => {
                    self.cursor = self.prev_boundary();
                    FieldEvent::Moved
                }
                Key::Right => {
                    self.cursor = self.next_boundary();
                    FieldEvent::Moved
                }
                Key::Home => {
                    self.cursor = 0;
                    FieldEvent::Moved
                }
                Key::End => {
                    self.cursor = self.value.len();
                    FieldEvent::Moved
                }
                _ if field.holds() => FieldEvent::Ignored,
                Key::Char(c) => {
                    self.value.insert(self.cursor, c);
                    self.cursor += c.len_utf8();
                    FieldEvent::Edited
                }
                Key::Backspace if self.cursor > 0 => {
                    let from = self.prev_boundary();
                    self.value.replace_range(from..self.cursor, "");
                    self.cursor = from;
                    FieldEvent::Edited
                }
                Key::Delete if self.cursor < self.value.len() => {
                    let to = self.next_boundary();
                    self.value.replace_range(self.cursor..to, "");
                    FieldEvent::Edited
                }
                _ => FieldEvent::Ignored,
            },
        }
    }
}

impl FieldState {
    /// Text pasted at the cursor: control characters left out, and newlines too in a one-line
    /// input (a text area keeps them).
    pub fn paste(&mut self, field: &TextField, text: &str) -> FieldEvent {
        if field.disabled || field.holds() || self.focus != FieldFocus::Control {
            return FieldEvent::Ignored;
        }
        let keep = |c: char| !c.is_control() || (c == '\n' && field.multiline.is_some());
        let text: String = text.replace("\r\n", "\n").chars().filter(|&c| keep(c)).collect();
        if text.is_empty() {
            return FieldEvent::Ignored;
        }
        self.value.insert_str(self.cursor, &text);
        self.cursor += text.len();
        FieldEvent::Edited
    }

    /// A key in a text area: Enter starts a line, Up and Down move between lines, Home and End go
    /// to a line's ends; the rest as in a one-line input.
    fn handle_area(&mut self, field: &TextField, key: KeyPress) -> FieldEvent {
        let lines = visual_lines(&self.value, self.wrap.max(1));
        let at = line_of(&lines, self.cursor);
        let col = text::width(&self.value[lines[at].0..self.cursor]);
        let to_col = |s: &FieldState, line: (usize, usize), col: usize| {
            let mut used = 0;
            for (i, c) in s.value[line.0..line.1].char_indices() {
                let w = text::char_width(c);
                if used + w > col {
                    return line.0 + i;
                }
                used += w;
            }
            line.1
        };
        match key.key {
            Key::Escape => FieldEvent::Escape,
            Key::Up if at > 0 => {
                self.cursor = to_col(self, lines[at - 1], col);
                FieldEvent::Moved
            }
            Key::Down if at + 1 < lines.len() => {
                self.cursor = to_col(self, lines[at + 1], col);
                FieldEvent::Moved
            }
            Key::Up | Key::Down => FieldEvent::Ignored,
            Key::Home => {
                self.cursor = lines[at].0;
                FieldEvent::Moved
            }
            Key::End => {
                self.cursor = lines[at].1;
                FieldEvent::Moved
            }
            Key::Enter if !field.holds() => {
                self.value.insert(self.cursor, '\n');
                self.cursor += 1;
                FieldEvent::Edited
            }
            Key::Left => {
                self.cursor = self.prev_boundary();
                FieldEvent::Moved
            }
            Key::Right => {
                self.cursor = self.next_boundary();
                FieldEvent::Moved
            }
            _ if field.holds() => FieldEvent::Ignored,
            Key::Char(c) => {
                self.value.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                FieldEvent::Edited
            }
            Key::Backspace if self.cursor > 0 => {
                let from = self.prev_boundary();
                self.value.replace_range(from..self.cursor, "");
                self.cursor = from;
                FieldEvent::Edited
            }
            Key::Delete if self.cursor < self.value.len() => {
                let to = self.next_boundary();
                self.value.replace_range(self.cursor..to, "");
                FieldEvent::Edited
            }
            _ => FieldEvent::Ignored,
        }
    }
}

/// A labeled text input. Build it each frame and draw it with its [`FieldState`].
#[derive(Debug, Clone)]
pub struct TextField<'a> {
    look: &'a Look,
    label: String,
    hint: Option<String>,
    key_hint: bool,
    description: Option<String>,
    help: Option<String>,
    fallback: Option<String>,
    required: bool,
    required_mark: RequiredMark,
    issues: Vec<Issue>,
    placeholder: Option<String>,
    read_only: bool,
    /// Work on the value under way, for this many ms: its text held, a spinner at the end.
    busy: Option<u64>,
    disabled: bool,
    words: FieldWords,
    multiline: Option<(u16, u16)>,
    masked: bool,
    /// In a row of fields: the description rows the row gives each, so their controls stand level.
    in_row: Option<u16>,
}

impl<'a> TextField<'a> {
    pub fn new(look: &'a Look, label: impl Into<String>) -> TextField<'a> {
        TextField {
            look,
            label: label.into(),
            hint: None,
            key_hint: false,
            description: None,
            help: None,
            fallback: None,
            required: false,
            required_mark: RequiredMark::Star,
            issues: Vec::new(),
            placeholder: None,
            read_only: false,
            busy: None,
            disabled: false,
            words: FieldWords::default(),
            multiline: None,
            masked: false,
            in_row: None,
        }
    }

    /// Beside the label: a unit, a format, a default.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// The hint is a setting's key, never translated.
    pub fn key_hint(mut self, key: impl Into<String>) -> Self {
        self.hint = Some(key.into());
        self.key_hint = true;
        self
    }

    /// Between label and control: what the question asks.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Under the control: what a right answer looks like.
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Under the help: what applies when the field is left empty.
    pub fn fallback(mut self, fallback: impl Into<String>) -> Self {
        self.fallback = Some(fallback.into());
        self
    }

    pub fn required(mut self, mark: RequiredMark) -> Self {
        self.required = true;
        self.required_mark = mark;
        self
    }

    /// One critical issue: the value is refused.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        if let Some(e) = error {
            self.issues.push(Issue::critical(e));
        }
        self
    }

    /// Several issues; the worst sets the control's look.
    pub fn issues(mut self, issues: impl IntoIterator<Item = Issue>) -> Self {
        self.issues.extend(issues);
        self
    }

    /// An example in the empty control; never instead of a label.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Work on the value under way (a save, a check), for `elapsed_ms` so far (`busy`, proposed
    /// in the spec): its text held as read only holds it, a spinner at the control's end, focus
    /// kept.
    pub fn busy(mut self, busy: bool, elapsed_ms: u64) -> Self {
        self.busy = busy.then_some(elapsed_ms);
        self
    }

    /// Whether the value is held from edits: read only, or busy.
    fn holds(&self) -> bool {
        self.read_only || self.busy.is_some()
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// A secret (a token, a password): each character drawn as the bullet, one column each, the
    /// value itself never on screen. One line. sokkel-tui's reading (proposal c6): the contract's
    /// text field has no masked input yet.
    pub fn masked(mut self, on: bool) -> Self {
        self.masked = on;
        self
    }

    /// In a row of fields (`inRow`): the field takes `description_rows` rows for its description,
    /// the most any field in the row needs ([`TextField::level`]), so the controls stand level.
    pub fn in_row(mut self, description_rows: u16) -> Self {
        self.in_row = Some(description_rows);
        self
    }

    /// The rows its description wraps to at `width` columns.
    pub fn description_rows(&self, width: u16) -> u16 {
        self.description
            .as_ref()
            .map_or(0, |t| text::wrap(t, width as usize).len() as u16)
    }

    /// The description rows a row of fields shares: the most any of them needs at its width.
    pub fn level<'f>(fields: impl IntoIterator<Item = (&'f TextField<'f>, u16)>) -> u16 {
        fields
            .into_iter()
            .map(|(f, w)| f.description_rows(w))
            .max()
            .unwrap_or(0)
    }

    /// A text area: at least `rows` lines, growing with its text up to `max_rows`, then
    /// scrolling. Enter starts a new line.
    pub fn multiline(mut self, rows: u16, max_rows: u16) -> Self {
        self.multiline = Some((rows.max(1), max_rows.max(rows.max(1))));
        self
    }

    pub fn words(mut self, words: FieldWords) -> Self {
        self.words = words;
        self
    }

    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }

    /// The worst issue's tone.
    pub fn worst(&self) -> Option<Tone> {
        self.issues.iter().map(|i| i.tone).max()
    }

    /// How many stops Tab makes in it: none disabled, the control, and the mark with an issue.
    pub fn stops(&self) -> usize {
        if self.disabled {
            0
        } else {
            1 + usize::from(self.has_issues())
        }
    }

    /// Every issue said as one sentence, each after its tone's word: the mark's name.
    pub fn mark_name(&self) -> Option<String> {
        if self.issues.is_empty() {
            return None;
        }
        let said: Vec<String> = self
            .issues
            .iter()
            .map(|i| {
                let lead = match i.tone {
                    Tone::Critical => &self.words.issue_critical,
                    Tone::Warning => &self.words.issue_warning,
                };
                format!("{lead}{}", i.text)
            })
            .collect();
        Some(said.join(" "))
    }

    /// What the status line says while the field has focus: its issues, on the control or the
    /// mark (principle 23).
    pub fn tip(&self, state: &FieldState) -> Option<String> {
        (state.focus != FieldFocus::None).then(|| self.mark_name()).flatten()
    }

    /// The rows it takes at `width` columns, a text area at its fewest.
    pub fn height(&self, width: u16) -> u16 {
        let w = width as usize;
        let lines = |t: &Option<String>| t.as_ref().map_or(0, |t| text::wrap(t, w).len());
        let control = self.multiline.map_or(1, |(rows, _)| rows);
        let description = self.in_row.map_or(lines(&self.description), |r| r as usize);
        (1 + control as usize + description + lines(&self.help) + lines(&self.fallback)) as u16
    }

    /// The rows it takes at `width` columns holding `state`'s value: a text area grows with it.
    pub fn height_with(&self, width: u16, state: &FieldState) -> u16 {
        self.height(width) - self.multiline.map_or(1, |(r, _)| r) + self.control_rows(width, state)
    }

    fn area_room(&self, width: u16) -> usize {
        width.saturating_sub(2 + if self.has_issues() { 2 } else { 0 }) as usize
    }

    fn control_rows(&self, width: u16, state: &FieldState) -> u16 {
        match self.multiline {
            None => 1,
            Some((rows, max)) => {
                let n = visual_lines(&state.value, self.area_room(width)).len() as u16;
                n.clamp(rows, max)
            }
        }
    }

    /// The issue popover, while it is open: draw it after everything else.
    pub fn popover<'f>(&'f self, state: &FieldState) -> Option<IssuePopover<'f, 'a>> {
        let anchor = state.mark_at?;
        (state.issue_open && self.has_issues()).then_some(IssuePopover { field: self, anchor })
    }

    fn control_state(&self, focus: bool) -> &'static str {
        if self.disabled {
            return "disabled";
        }
        match (self.worst(), focus) {
            (Some(Tone::Critical), true) => "invalid+focus",
            (Some(Tone::Critical), false) => "invalid",
            (Some(Tone::Warning), true) => "warning+focus",
            (Some(Tone::Warning), false) => "warning",
            (None, true) => "focus",
            (None, false) => "rest",
        }
    }

    /// A piece's color at rest, or faded while the field is disabled.
    fn piece(&self, piece: &str, field: &str) -> Rgba {
        let key = format!("text-field.base.rest.{piece}");
        let c = if self.disabled {
            self.look.faded(&key, field).or_else(|| self.look.paint(&key, field))
        } else {
            self.look.paint(&key, field)
        };
        c.unwrap_or_else(|| panic!("the contract paints no {field} for the text field's {piece}"))
    }

    /// The columns `value` takes as drawn: a masked one a column a character.
    fn shown_width(&self, value: &str) -> usize {
        if self.masked {
            value.chars().count()
        } else {
            text::width(value)
        }
    }

    /// A text area: each row with the edge at its ends, the value's lines wrapped at its room, the
    /// cursor's line kept in view.
    fn draw_area(
        &self,
        buf: &mut Buffer,
        rect: Rect,
        state: &mut FieldState,
        focus: bool,
        c: (Rgba, Rgba, Rgba, Rgba),
    ) {
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let (ground, edge, ink, quiet) = c;
        buf.set_style(rect, Style::new().bg(t(ground)));
        let edge_style = Style::new().bg(t(ground)).fg(t(edge));
        for y in rect.y..rect.bottom() {
            buf[(rect.x, y)].set_symbol(glyphs.edge_start).set_style(edge_style);
            buf[(rect.right() - 1, y)]
                .set_symbol(glyphs.edge_end)
                .set_style(edge_style);
        }
        let room = self.area_room(rect.width);
        state.wrap = room;
        let body = TextRole::Body.style(look);
        let start = rect.x + 1;
        if state.value.is_empty() {
            if let Some(p) = &self.placeholder {
                for (k, line) in text::wrap(p, room).into_iter().take(rect.height as usize).enumerate() {
                    buf.set_stringn(start, rect.y + k as u16, line, room, body.fg(t(quiet)));
                }
            }
            state.top = 0;
        }
        let lines = visual_lines(&state.value, room);
        let at = line_of(&lines, state.cursor);
        let view = rect.height as usize;
        if at < state.top {
            state.top = at;
        } else if at >= state.top + view {
            state.top = at + 1 - view;
        }
        if !state.value.is_empty() {
            for (k, &(a, b)) in lines.iter().enumerate().skip(state.top).take(view) {
                let y = rect.y + (k - state.top) as u16;
                buf.set_stringn(start, y, &state.value[a..b], room, body.fg(t(ink)));
            }
        }
        if focus {
            let col = text::width(&state.value[lines[at].0..state.cursor]) as u16;
            state.cursor_at = Some(Position::new(start + col, rect.y + (at - state.top) as u16));
        }
        // The issue mark, in the first row's end.
        if let Some(worst) = self.worst() {
            let mx = rect.right() - 3;
            let key = if worst == Tone::Warning { "warning" } else { "rest" };
            let mi = look
                .paint(&format!("text-field.base.{key}.issueMark"), "ink")
                .unwrap_or(ink);
            let g = match worst {
                Tone::Critical => glyphs.issue_critical,
                Tone::Warning => glyphs.issue_warning,
            };
            buf[(mx + 1, rect.y)]
                .set_symbol(g)
                .set_style(Style::new().fg(t(mi)).bg(t(ground)).add_modifier(Modifier::BOLD));
            state.mark_at = Some(Rect::new(mx, rect.y, 3, 1));
        }
    }

    fn put_lines(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: &mut u16,
        words: &Option<String>,
        piece: &str,
        role: TextRole,
    ) {
        let Some(words) = words else { return };
        let look = self.look;
        let style = role.style(look).fg(look.terminal(self.piece(piece, "ink")));
        for line in text::wrap(words, area.width as usize) {
            if *y >= area.bottom() {
                return;
            }
            buf.set_stringn(area.x, *y, line, area.width as usize, style);
            *y += 1;
        }
    }
}

impl StatefulWidget for &TextField<'_> {
    type State = FieldState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut FieldState) {
        let area = area.intersection(buf.area);
        state.cursor_at = None;
        state.mark_at = None;
        if area.width < 6 || area.height == 0 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let mut y = area.y;

        // The label row: label, the required mark, the hint.
        let label_style = TextRole::Label.style(look).fg(t(self.piece("label", "ink")));
        let mut spans = vec![Span::styled(self.label.clone(), label_style)];
        if self.required {
            spans.push(match self.required_mark {
                RequiredMark::Star => {
                    Span::styled(glyphs.required, Style::new().fg(t(self.piece("requiredStar", "ink"))))
                }
                RequiredMark::Word => Span::styled(
                    format!(" {}", self.words.required),
                    TextRole::Help.style(look).fg(t(self.piece("requiredWord", "ink"))),
                ),
            });
        }
        if let Some(hint) = &self.hint {
            // A key hint is the code face in muted-foreground: the description's ink.
            let ink = self.piece(if self.key_hint { "description" } else { "hint" }, "ink");
            spans.push(Span::raw(" "));
            spans.push(Span::styled(hint.clone(), TextRole::Help.style(look).fg(t(ink))));
        }
        buf.set_line(area.x, y, &Line::from(spans), area.width);
        y += 1;

        let from = y;
        self.put_lines(buf, area, &mut y, &self.description, "description", TextRole::Body);
        if let Some(rows) = self.in_row {
            y = (from + rows).min(area.bottom());
        }

        // The control.
        if y < area.bottom() {
            let focus = state.focus == FieldFocus::Control && !self.disabled;
            let key = format!("text-field.base.{}.control", self.control_state(focus));
            let page = look.color("background");
            let field = |f: &str| {
                let c = if self.disabled {
                    look.faded(&key, f)
                } else {
                    look.paint(&key, f)
                };
                c.unwrap_or(page)
            };
            let (ground, edge, ink, quiet) = (field("ground"), field("edge"), field("ink"), field("placeholder"));
            if self.multiline.is_some() {
                let rows = self.control_rows(area.width, state).min(area.bottom() - y);
                let rect = Rect {
                    y,
                    height: rows,
                    ..area
                };
                self.draw_area(buf, rect, state, focus, (ground, edge, ink, quiet));
                y += rows;
                self.put_lines(buf, area, &mut y, &self.help, "help", TextRole::Help);
                self.put_lines(buf, area, &mut y, &self.fallback, "fallback", TextRole::Help);
                return;
            }
            let row = Rect { y, height: 1, ..area };
            buf.set_style(row, Style::new().bg(t(ground)));
            let edge_style = Style::new().bg(t(ground)).fg(t(edge));
            buf[(row.x, y)].set_symbol(glyphs.edge_start).set_style(edge_style);
            buf[(row.right() - 1, y)]
                .set_symbol(glyphs.edge_end)
                .set_style(edge_style);

            // The mark: one column, one from the control's end; the value's room ends three
            // columns before the end while an issue shows.
            let mark_w: u16 = if self.has_issues() { 2 } else { 0 };
            let busy_w: u16 = if self.busy.is_some() { 2 } else { 0 };
            let start = row.x + 1;
            let room = row.width.saturating_sub(2 + mark_w + busy_w) as usize;
            let body = TextRole::Body.style(look);
            if state.value.is_empty() {
                if let Some(p) = &self.placeholder {
                    let p = text::truncate(p, room, glyphs.ellipsis);
                    buf.set_stringn(start, y, p, room, body.fg(t(quiet)));
                }
                state.scroll = 0;
            } else {
                // Keep the cursor in view: the window follows it. A masked value is drawn as one
                // bullet a character.
                let shown: String = if self.masked {
                    glyphs.bullet.repeat(state.value.chars().count())
                } else {
                    state.value.clone()
                };
                let at = self.shown_width(&state.value[..state.cursor]);
                if at < state.scroll {
                    state.scroll = at;
                } else if room > 0 && at >= state.scroll + room {
                    state.scroll = at + 1 - room;
                }
                let mut col = 0;
                let mut x = start;
                for c in shown.chars() {
                    let w = text::char_width(c);
                    if col >= state.scroll && col + w <= state.scroll + room {
                        let mut b = [0u8; 4];
                        buf[(x, y)].set_symbol(c.encode_utf8(&mut b)).set_style(body.fg(t(ink)));
                        x += w as u16;
                    }
                    col += w;
                }
            }
            if focus {
                let at = self.shown_width(&state.value[..state.cursor]);
                state.cursor_at = Some(Position::new(start + (at - state.scroll) as u16, y));
            }

            // Busy: the spinner at the control's end, before the issue mark where one shows.
            if let Some(ms) = self.busy {
                let sx = row.right().saturating_sub(2 + if mark_w > 0 { 3 } else { 0 });
                let quiet = look.color("muted-foreground");
                buf.set_string(sx, y, look.spinner_frame(ms), Style::new().bg(t(ground)).fg(t(quiet)));
            }

            // The issue mark, inside the control's end.
            if let Some(worst) = self.worst() {
                // The glyph one column in from the end; the cells either side are its ring's.
                let mx = row.right() - 3;
                let mark_rest = if worst == Tone::Warning { "warning" } else { "rest" };
                let mark_ground = if state.issue_open {
                    look.paint("text-field.base.issueOpen.issueMark", "ground")
                        .unwrap_or(ground)
                } else {
                    ground
                };
                let mark_ink = look
                    .paint(&format!("text-field.base.{mark_rest}.issueMark"), "ink")
                    .unwrap_or(ink);
                let glyph = match worst {
                    Tone::Critical => glyphs.issue_critical,
                    Tone::Warning => glyphs.issue_warning,
                };
                let mark_style = Style::new().bg(t(mark_ground)).fg(t(mark_ink));
                buf[(mx + 1, y)]
                    .set_symbol(glyph)
                    .set_style(mark_style.add_modifier(Modifier::BOLD));
                if state.focus == FieldFocus::Mark {
                    let ring = look.ring("text-field.base.focus.issueMark", "ring").unwrap_or(mark_ink);
                    let ring_style = Style::new().bg(t(ground)).fg(t(ring));
                    buf[(mx, y)].set_symbol(glyphs.edge_end).set_style(ring_style);
                    buf[(mx + 2, y)].set_symbol(glyphs.edge_start).set_style(ring_style);
                }
                state.mark_at = Some(Rect::new(mx, y, 3, 1));
            }
            y += 1;
        }

        self.put_lines(buf, area, &mut y, &self.help, "help", TextRole::Help);
        self.put_lines(buf, area, &mut y, &self.fallback, "fallback", TextRole::Help);
    }
}

/// The issue mark's popover: each issue's text, its detail, and its actions. Drawn last, over
/// what lies below the field, below the mark and aligned to its end, or above it when there is
/// no room below.
#[derive(Debug, Clone)]
pub struct IssuePopover<'f, 'a> {
    field: &'f TextField<'a>,
    anchor: Rect,
}

impl IssuePopover<'_, '_> {
    /// Where it lands on a screen of `screen`.
    pub fn area(&self, screen: Rect) -> Rect {
        let w = crate::cells::cols(320.0)
            .min(screen.width.saturating_sub(crate::cells::cols(32.0)))
            .max(12);
        let h = self.lines(w).len() as u16 + 2;
        let x = self.anchor.right().saturating_sub(w).max(screen.x);
        let below = self.anchor.bottom();
        let y = if below + h <= screen.bottom() {
            below
        } else {
            self.anchor.y.saturating_sub(h).max(screen.y)
        };
        Rect::new(x, y, w, h).intersection(screen)
    }

    fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let field = self.field;
        let look = field.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let pop = "text-field.base.rest.issuePopover";
        let ink = look
            .paint(pop, "ink")
            .unwrap_or_else(|| look.color("popover-foreground"));
        let quiet = look.color("muted-foreground");
        let room = width.saturating_sub(4) as usize;
        let mut out = Vec::new();
        for (n, issue) in field.issues.iter().enumerate() {
            if n > 0 {
                out.push(Line::raw(""));
            }
            let (glyph, tone_ink) = match issue.tone {
                Tone::Critical => (glyphs.issue_critical, "critical-ink"),
                Tone::Warning => (glyphs.issue_warning, "warning-ink"),
            };
            let lead = Style::new().fg(t(look.color(tone_ink)));
            let words = TextRole::Label.style(look).fg(t(ink));
            for (i, line) in text::wrap(&issue.text, room.saturating_sub(2)).into_iter().enumerate() {
                out.push(Line::from(vec![
                    Span::styled(if i == 0 { glyph } else { " " }, lead),
                    Span::raw(" "),
                    Span::styled(line, words),
                ]));
            }
            let help = TextRole::Help.style(look);
            if let Some(detail) = &issue.detail {
                for line in text::wrap(detail, room.saturating_sub(2)) {
                    out.push(Line::from(vec![Span::raw("  "), Span::styled(line, help.fg(t(quiet)))]));
                }
            }
            if !issue.actions.is_empty() {
                let mut spans = vec![Span::raw("  ")];
                for (i, a) in issue.actions.iter().enumerate() {
                    if i > 0 {
                        spans.push(Span::raw("  "));
                    }
                    spans.push(Span::styled(
                        a.clone(),
                        help.fg(t(ink)).add_modifier(Modifier::UNDERLINED),
                    ));
                }
                out.push(Line::from(spans));
            }
        }
        out
    }
}

impl Widget for IssuePopover<'_, '_> {
    fn render(self, screen: Rect, buf: &mut Buffer) {
        let area = self.area(screen.intersection(buf.area));
        if area.width < 4 || area.height < 3 {
            return;
        }
        let look = self.field.look;
        let t = |c: Rgba| look.terminal(c);
        let pop = "text-field.base.rest.issuePopover";
        let ground = look.paint(pop, "ground").unwrap_or_else(|| look.color("popover"));
        let edge = look.paint(pop, "edge").unwrap_or(ground);
        // The border's cells stand on the page; only the inside wears the popover's ground.
        let page = look.color("background");
        Clear.render(area, buf);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(look.border_set("radius-surface"))
            .border_style(Style::new().fg(t(edge)).bg(t(page)));
        let inner = block.inner(area);
        block.render(area, buf);
        buf.set_style(inner, Style::new().bg(t(ground)));
        let inner = Rect {
            x: inner.x + 1,
            width: inner.width.saturating_sub(2),
            ..inner
        };
        for (i, line) in self.lines(area.width).into_iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.bottom() {
                break;
            }
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}
