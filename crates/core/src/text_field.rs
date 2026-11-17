//! TextField, from sokkel-design `parts/text-field`.
//!
//! A labeled text input: the field frame every field shares (label, hint, description, control,
//! help, fallback, issue mark) around a one-line input. A problem is said by a mark at the
//! control's end, never by a line inserted under it, so a form never changes height.
//!
//! In cells the control is one row, its edge drawn in its pad cells as a button's is; the gaps of
//! 6 px between the frame's rows are no rows. The issue mark takes three columns inside the
//! control's end and is a stop of its own after the control. Its popover is drawn last, over what
//! lies below the field, with [`TextField::popover`]. The terminal shows one face, so `mono` and
//! `numeric` draw as the rest do.

use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::text::{self, TextRole};
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
                _ if field.read_only => FieldEvent::Ignored,
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
    disabled: bool,
    words: FieldWords,
    on: &'a str,
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
            disabled: false,
            words: FieldWords::default(),
            on: "background",
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

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn words(mut self, words: FieldWords) -> Self {
        self.words = words;
        self
    }

    /// The surface it stands on, a color token: `background` (default), `card`, `popover`.
    pub fn on(mut self, surface: &'a str) -> Self {
        self.on = surface;
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

    /// What the status line says while the field has focus: the issues, while the mark has it.
    pub fn tip(&self, state: &FieldState) -> Option<String> {
        (state.focus == FieldFocus::Mark).then(|| self.mark_name()).flatten()
    }

    /// The rows it takes at `width` columns.
    pub fn height(&self, width: u16) -> u16 {
        let w = width as usize;
        let lines = |t: &Option<String>| t.as_ref().map_or(0, |t| text::wrap(t, w).len());
        (2 + lines(&self.description) + lines(&self.help) + lines(&self.fallback)) as u16
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

    fn piece(&self, piece: &str, field: &str, under: Rgba) -> Rgba {
        self.look
            .paint_on(&format!("text-field.base.rest.{piece}"), field, under)
            .unwrap_or_else(|| panic!("the contract paints no {field} for the text field's {piece}"))
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
        let under = look.surface(self.on);
        let style = role.style(look).fg(look.terminal(self.piece(piece, "ink", under)));
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
        let under = look.surface(self.on);
        let mut y = area.y;

        // The label row: label, the required mark, the hint.
        let label_style = TextRole::Label.style(look).fg(t(self.piece("label", "ink", under)));
        let mut spans = vec![Span::styled(self.label.clone(), label_style)];
        if self.required {
            spans.push(match self.required_mark {
                RequiredMark::Star => Span::styled(
                    glyphs.required,
                    Style::new().fg(t(self.piece("requiredStar", "ink", under))),
                ),
                RequiredMark::Word => Span::styled(
                    format!(" {}", self.words.required),
                    TextRole::Help
                        .style(look)
                        .fg(t(self.piece("requiredWord", "ink", under))),
                ),
            });
        }
        if let Some(hint) = &self.hint {
            let ink = if self.key_hint {
                look.color_on("muted-foreground", under)
            } else {
                self.piece("hint", "ink", under)
            };
            spans.push(Span::raw(" "));
            spans.push(Span::styled(hint.clone(), TextRole::Help.style(look).fg(t(ink))));
        }
        buf.set_line(area.x, y, &Line::from(spans), area.width);
        y += 1;

        self.put_lines(buf, area, &mut y, &self.description, "description", TextRole::Body);

        // The control.
        if y < area.bottom() {
            let focus = state.focus == FieldFocus::Control && !self.disabled;
            let key = format!("text-field.base.{}.control", self.control_state(focus));
            let ground = look.paint_on(&key, "ground", under).unwrap_or(under);
            let field = |f: &str| look.paint_on(&key, f, ground).unwrap_or(ground);
            let (mut ground, mut edge, mut ink, mut quiet) =
                (ground, field("edge"), field("ink"), field("placeholder"));
            if self.disabled {
                let fade = |c: Rgba| look.faded(c, under);
                (ground, edge, ink, quiet) = (fade(ground), fade(edge), fade(ink), fade(quiet));
            }
            let row = Rect { y, height: 1, ..area };
            buf.set_style(row, Style::new().bg(t(ground)));
            let edge_style = Style::new().bg(t(ground)).fg(t(edge));
            buf[(row.x, y)].set_symbol(glyphs.edge_start).set_style(edge_style);
            buf[(row.right() - 1, y)]
                .set_symbol(glyphs.edge_end)
                .set_style(edge_style);

            let mark_w: u16 = if self.has_issues() { 3 } else { 0 };
            let start = row.x + 1;
            let room = row.width.saturating_sub(2 + mark_w) as usize;
            let body = TextRole::Body.style(look);
            if state.value.is_empty() {
                if let Some(p) = &self.placeholder {
                    let p = text::truncate(p, room, glyphs.ellipsis);
                    buf.set_stringn(start, y, p, room, body.fg(t(quiet)));
                }
                state.scroll = 0;
            } else {
                // Keep the cursor in view: the window follows it.
                let at = text::width(&state.value[..state.cursor]);
                if at < state.scroll {
                    state.scroll = at;
                } else if room > 0 && at >= state.scroll + room {
                    state.scroll = at + 1 - room;
                }
                let mut col = 0;
                let mut x = start;
                for c in state.value.chars() {
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
                let at = text::width(&state.value[..state.cursor]);
                state.cursor_at = Some(Position::new(start + (at - state.scroll) as u16, y));
            }

            // The issue mark, inside the control's end.
            if let Some(worst) = self.worst() {
                let mx = row.right() - 1 - mark_w;
                let mark_rest = if worst == Tone::Warning { "warning" } else { "rest" };
                let mut mark_ground = ground;
                if state.issue_open {
                    mark_ground = look
                        .paint_on("text-field.base.issueOpen.issueMark", "ground", ground)
                        .unwrap_or(ground);
                }
                let mark_ink = look
                    .paint_on(&format!("text-field.base.{mark_rest}.issueMark"), "ink", mark_ground)
                    .unwrap_or(ink);
                let glyph = match worst {
                    Tone::Critical => glyphs.issue_critical,
                    Tone::Warning => glyphs.issue_warning,
                };
                let mark_style = Style::new().bg(t(mark_ground)).fg(t(mark_ink));
                buf.set_style(Rect::new(mx, y, mark_w, 1), mark_style);
                buf[(mx + 1, y)]
                    .set_symbol(glyph)
                    .set_style(mark_style.add_modifier(Modifier::BOLD));
                if state.focus == FieldFocus::Mark {
                    let ring = look
                        .paint_ring_on("text-field.base.focus.issueMark", "ring", mark_ground)
                        .unwrap_or(mark_ink);
                    let ring_style = Style::new().bg(t(mark_ground)).fg(t(ring));
                    buf[(mx, y)].set_symbol(glyphs.edge_end).set_style(ring_style);
                    buf[(mx + 2, y)].set_symbol(glyphs.edge_start).set_style(ring_style);
                }
                state.mark_at = Some(Rect::new(mx, y, mark_w, 1));
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
        let under = look.surface("popover");
        let ground = look.paint_on(pop, "ground", under).unwrap_or(under);
        let ink = look.paint_on(pop, "ink", ground).unwrap_or(ground);
        let quiet = look.color_on("muted-foreground", ground);
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
            let lead = Style::new().fg(t(look.color_on(tone_ink, ground)));
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
        let under = look.surface("popover");
        let ground = look.paint_on(pop, "ground", under).unwrap_or(under);
        let edge = look.paint_on(pop, "edge", ground).unwrap_or(ground);
        // The border's cells stand on the page; only the inside wears the popover's ground.
        let page = look.surface("background");
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
