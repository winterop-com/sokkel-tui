//! CommandPalette, from sokkel-design `parts/command-palette`.
//!
//! The box Mod+K opens (Ctrl+K in a terminal): a search over the app's commands in groups, the
//! active row run by Enter. A command that answers later holds the palette open, a spinner in its
//! row, until `busy-release`, when a Stop waiting row lets it go. A refused command says why under
//! the rows.
//!
//! In cells (the part carries no `cells` note yet; sokkel-tui's reading, in its proposal) the box is
//! the dialog's, 46 columns (384 px; `md` 61, `lg` 80) a third of the way down and centered; below
//! md it takes the window's width less 4, a row from the top. The search is a row on its tint with
//! the search icon at its start; a group's title a row in the help role; every command one row: the
//! icon, the label, the hint or code after it, and at its end the meta, the kind and the keys as
//! caps. The list shows at most 60 percent of the window, the active row kept in view. What lies
//! under the palette is drawn as it is (open-questions T2).
//!
//! Not yet: the app's own search and its results, two-line rows with artwork, the foot, a chooser's
//! description, the touch screen's Close.

use crate::cells::{Width, cols};
use crate::keys::{Key, KeyPress};
use crate::look::{Look, Rgba};
use crate::shortcuts::{Keys, ShortcutWords};
use crate::text::{self, TextRole};
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Clear, Widget};

/// A status ink for a command's meta or hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTone {
    Critical,
    Warning,
    Good,
    Info,
}

impl CommandTone {
    fn key(self) -> &'static str {
        match self {
            CommandTone::Critical => "critical",
            CommandTone::Warning => "warning",
            CommandTone::Good => "good",
            CommandTone::Info => "info",
        }
    }
}

/// One command.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Command {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub hint: Option<String>,
    pub code: Option<String>,
    pub meta: Option<String>,
    pub meta_tone: Option<CommandTone>,
    pub kind: Option<String>,
    pub keys: Option<Keys>,
    pub keywords: Vec<String>,
    /// Disabled, and why: the reason stands in the hint's place.
    pub disabled: Option<String>,
    /// A second line under the label: a two-line row, drawn as a search result.
    pub detail: Option<String>,
}

impl Command {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Command {
        Command {
            id: id.into(),
            label: label.into(),
            ..Command::default()
        }
    }

    pub fn icon(mut self, glyph: impl Into<String>) -> Self {
        self.icon = Some(glyph.into());
        self
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn meta(mut self, meta: impl Into<String>, tone: Option<CommandTone>) -> Self {
        self.meta = Some(meta.into());
        self.meta_tone = tone;
        self
    }

    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn keys(mut self, keys: Keys) -> Self {
        self.keys = Some(keys);
        self
    }

    pub fn keywords(mut self, words: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.keywords = words.into_iter().map(Into::into).collect();
        self
    }

    /// A second line under the label, making it a two-line row (a search result).
    pub fn detail(mut self, words: impl Into<String>) -> Self {
        self.detail = Some(words.into());
        self
    }

    pub fn disabled(mut self, why: impl Into<String>) -> Self {
        self.disabled = Some(why.into());
        self
    }
}

/// Commands under a title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandGroup {
    pub title: String,
    pub commands: Vec<Command>,
}

impl CommandGroup {
    pub fn new(title: impl Into<String>, commands: impl IntoIterator<Item = Command>) -> CommandGroup {
        CommandGroup {
            title: title.into(),
            commands: commands.into_iter().collect(),
        }
    }
}

/// The words a palette says, with the spec's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteWords {
    pub placeholder: String,
    pub empty: String,
    pub title: String,
    pub close: String,
    pub running: String,
    pub refused: String,
    pub stop_waiting: String,
    /// The foot's word for the active row's verb, beside Enter.
    pub run: String,
    pub enter: String,
    pub keys: ShortcutWords,
}

impl Default for PaletteWords {
    fn default() -> Self {
        PaletteWords {
            placeholder: "Search or run a command".into(),
            empty: "No command matches.".into(),
            title: "Command palette".into(),
            close: "Close".into(),
            running: "Running".into(),
            refused: "The command did not run.".into(),
            stop_waiting: "Stop waiting".into(),
            run: "Run".into(),
            enter: "Enter".into(),
            keys: ShortcutWords::default(),
        }
    }
}

/// What a key did to the palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteEvent {
    Ignored,
    /// The query changed or the active row moved.
    Changed,
    /// Run the command with this id. The app answers with [`PaletteState::done`],
    /// [`PaletteState::running`] or [`PaletteState::refused`].
    Run(String),
    /// Closed: focus back where it was.
    Close,
    /// Let a running command go after `busy-release`: the app stops the work if it can.
    StopWaiting,
}

/// The query, the active row and a running command: what lives between frames.
#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    pub query: String,
    /// The active row, as a place among the rows that match.
    pub active: usize,
    /// A command that answers later, by id, and how long it has run.
    pub running: Option<(String, u64)>,
    /// Why the last command was refused.
    pub refusal: Option<String>,
    scroll: usize,
    cursor_at: Option<Position>,
}

impl PaletteState {
    /// The chosen command answered later: the palette holds, its row busy.
    pub fn running(&mut self, id: impl Into<String>, elapsed_ms: u64) {
        self.running = Some((id.into(), elapsed_ms));
    }

    /// It settled: the app closes the palette.
    pub fn done(&mut self) {
        *self = PaletteState::default();
    }

    /// It was refused: the palette stays open and says why (`None`: the palette's own words).
    pub fn refused(&mut self, why: Option<String>, words: &PaletteWords) {
        self.running = None;
        self.refusal = Some(why.unwrap_or_else(|| words.refused.clone()));
    }

    /// Where the terminal's cursor goes: the query's end.
    pub fn cursor_position(&self) -> Option<Position> {
        self.cursor_at
    }
}

/// How well `query` matches `words`: none, or a score where higher is better. A prefix beats the
/// start of a word, which beats letters in order (`fuzzy`).
pub fn score(query: &str, words: &str) -> Option<u32> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some(1);
    }
    let w = words.to_lowercase();
    if w.starts_with(&q) {
        return Some(300);
    }
    if w.split(|c: char| !c.is_alphanumeric()).any(|part| part.starts_with(&q)) {
        return Some(200);
    }
    if w.contains(&q) {
        return Some(150);
    }
    let mut chars = w.chars();
    let mut gaps = 0u32;
    for c in q.chars() {
        let mut skipped = 0;
        loop {
            match chars.next() {
                Some(x) if x == c => break,
                Some(_) => skipped += 1,
                None => return None,
            }
        }
        gaps += skipped;
    }
    Some(100u32.saturating_sub(gaps))
}

/// One drawn line of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Line {
    StopWaiting,
    Title(usize),
    Separator,
    Command(usize, usize),
    /// A two-line row's second line.
    Detail(usize, usize),
}

/// The command palette. Build it each frame; draw it last, over the frame.
#[derive(Debug, Clone)]
pub struct CommandPalette<'a> {
    look: &'a Look,
    groups: Vec<CommandGroup>,
    sheet: bool,
    width_px: f64,
    words: PaletteWords,
    /// The foot: the app's name, and the keys that open the palette.
    foot: Option<(String, Option<Keys>)>,
}

impl<'a> CommandPalette<'a> {
    pub fn new(look: &'a Look, groups: impl IntoIterator<Item = CommandGroup>) -> CommandPalette<'a> {
        CommandPalette {
            look,
            groups: groups.into_iter().collect(),
            sheet: false,
            width_px: 384.0,
            words: PaletteWords::default(),
            foot: None,
        }
    }

    /// The window's width decides its form: below md it takes the window's width.
    pub fn window(mut self, width: Width) -> Self {
        self.sheet = width == Width::Narrow;
        self
    }

    /// From md: 384 px (`sm`, the default), 512 (`md`) or 672 (`lg`).
    pub fn width_px(mut self, px: f64) -> Self {
        self.width_px = px;
        self
    }

    /// The strip at its foot: the app's `name`, the active row's verb beside Enter, and
    /// "Command palette" beside the `keys` that open it.
    pub fn foot(mut self, name: impl Into<String>, keys: Option<Keys>) -> Self {
        self.foot = Some((name.into(), keys));
        self
    }

    fn foot_rows(&self) -> u16 {
        if self.foot.is_some() { 2 } else { 0 }
    }

    pub fn words(mut self, words: PaletteWords) -> Self {
        self.words = words;
        self
    }

    /// The commands that match the query, by group, best first within a group.
    pub fn matches(&self, query: &str) -> Vec<(usize, Vec<usize>)> {
        self.groups
            .iter()
            .enumerate()
            .filter_map(|(g, group)| {
                let mut hits: Vec<(u32, usize)> = group
                    .commands
                    .iter()
                    .enumerate()
                    .filter_map(|(c, cmd)| {
                        let words = std::iter::once(cmd.label.as_str())
                            .chain(cmd.keywords.iter().map(String::as_str))
                            .chain(cmd.code.as_deref());
                        words.filter_map(|w| score(query, w)).max().map(|s| (s, c))
                    })
                    .collect();
                hits.sort_by_key(|&(s, c)| (std::cmp::Reverse(s), c));
                (!hits.is_empty()).then(|| (g, hits.into_iter().map(|(_, c)| c).collect()))
            })
            .collect()
    }

    /// The rows that match, in order, as (group, command).
    fn rows(&self, query: &str) -> Vec<(usize, usize)> {
        self.matches(query)
            .into_iter()
            .flat_map(|(g, cs)| cs.into_iter().map(move |c| (g, c)))
            .collect()
    }

    fn command(&self, at: (usize, usize)) -> &Command {
        &self.groups[at.0].commands[at.1]
    }

    fn released(&self, state: &PaletteState) -> bool {
        state
            .running
            .as_ref()
            .is_some_and(|(_, ms)| *ms >= self.look.duration_ms("busy-release"))
    }

    /// Answer a key while the palette is open.
    pub fn handle(&self, state: &mut PaletteState, key: KeyPress) -> PaletteEvent {
        if key.alt {
            return PaletteEvent::Ignored;
        }
        let busy = state.running.is_some();
        let released = self.released(state);
        // Mod+K closes it again, not while a command runs.
        if key.ctrl && matches!(key.key, Key::Char('k') | Key::Char('K')) {
            return if busy {
                PaletteEvent::Ignored
            } else {
                PaletteEvent::Close
            };
        }
        if key.ctrl {
            return PaletteEvent::Ignored;
        }
        if key.key == Key::Escape {
            return match (busy, released) {
                (false, _) => PaletteEvent::Close,
                (true, true) => PaletteEvent::StopWaiting,
                (true, false) => PaletteEvent::Ignored,
            };
        }
        if busy {
            // Another choice is ignored while a command runs; past release, Stop waiting answers.
            return if released && key.key == Key::Enter && state.active == 0 {
                PaletteEvent::StopWaiting
            } else if released && matches!(key.key, Key::Up | Key::Down) {
                state.active = 0;
                PaletteEvent::Changed
            } else {
                PaletteEvent::Ignored
            };
        }
        let rows = self.rows(&state.query);
        let enabled: Vec<usize> = (0..rows.len())
            .filter(|&k| self.command(rows[k]).disabled.is_none())
            .collect();
        let at = enabled.iter().position(|&k| k == state.active).unwrap_or(0);
        match key.key {
            Key::Down if !enabled.is_empty() => {
                state.active = enabled[(at + 1).min(enabled.len() - 1)];
                PaletteEvent::Changed
            }
            Key::Up if !enabled.is_empty() => {
                state.active = enabled[at.saturating_sub(1)];
                PaletteEvent::Changed
            }
            Key::Home if !enabled.is_empty() => {
                state.active = enabled[0];
                PaletteEvent::Changed
            }
            Key::End if !enabled.is_empty() => {
                state.active = enabled[enabled.len() - 1];
                PaletteEvent::Changed
            }
            Key::Enter => match enabled.get(at).map(|&k| rows[k]) {
                Some(r) if enabled.contains(&state.active) => {
                    state.refusal = None;
                    PaletteEvent::Run(self.command(r).id.clone())
                }
                _ => PaletteEvent::Ignored,
            },
            Key::Char(c) => {
                state.query.push(c);
                self.retarget(state);
                PaletteEvent::Changed
            }
            Key::Backspace => {
                state.query.pop();
                self.retarget(state);
                PaletteEvent::Changed
            }
            _ => PaletteEvent::Ignored,
        }
    }

    /// After the query changed: the first enabled row active, the refusal cleared.
    fn retarget(&self, state: &mut PaletteState) {
        let rows = self.rows(&state.query);
        state.active = (0..rows.len())
            .find(|&k| self.command(rows[k]).disabled.is_none())
            .unwrap_or(0);
        state.scroll = 0;
        state.refusal = None;
    }

    /// What the status line says while a row is active: its hint, when it has one.
    pub fn tip(&self, state: &PaletteState) -> Option<String> {
        let rows = self.rows(&state.query);
        rows.get(state.active).and_then(|&r| self.command(r).hint.clone())
    }

    fn lines(&self, state: &PaletteState) -> Vec<Line> {
        let mut out = Vec::new();
        if self.released(state) {
            out.push(Line::StopWaiting);
        }
        for (k, (g, cs)) in self.matches(&state.query).into_iter().enumerate() {
            if k > 0 {
                out.push(Line::Separator);
            }
            out.push(Line::Title(g));
            for c in cs {
                out.push(Line::Command(g, c));
                if self.groups[g].commands[c].detail.is_some() {
                    out.push(Line::Detail(g, c));
                }
            }
        }
        out
    }

    /// Where it stands in `window`: a third of the way down and centered; below md the window's
    /// width less 4, a row from the top.
    pub fn area(&self, window: Rect, state: &PaletteState) -> Rect {
        let w = if self.sheet {
            window.width.saturating_sub(4)
        } else {
            cols(self.width_px).min(window.width.saturating_sub(4))
        };
        let list_max = (window.height as usize * 6 / 10).max(3);
        let lines = self.lines(state).len().max(1).min(list_max) as u16;
        let refusal = if state.refusal.is_some() { 2 } else { 0 };
        let h = (2 + 1 + 1 + lines + refusal + self.foot_rows()).min(window.height);
        let x = window.x + (window.width - w) / 2;
        let y = if self.sheet {
            window.y + 1
        } else {
            window.y + (window.height.saturating_sub(h)) / 3
        };
        Rect::new(x, y.min(window.bottom().saturating_sub(h)), w, h)
    }
}

/// What a pointer hits in the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteHit {
    /// A command, by its place among the rows that match.
    Command(usize),
    StopWaiting,
}

impl CommandPalette<'_> {
    /// What a pointer at `at` hits in the palette drawn in `area` with `state`.
    pub fn hit(&self, area: Rect, at: Position, state: &PaletteState) -> Option<PaletteHit> {
        let inner = Rect::new(
            area.x + 1,
            area.y + 1,
            area.width.saturating_sub(2),
            area.height.saturating_sub(2),
        );
        let refusal = if state.refusal.is_some() { 2 } else { 0 };
        let list = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(2 + refusal + self.foot_rows()),
            ..inner
        };
        if !list.contains(at) {
            return None;
        }
        let k = state.scroll + (at.y - list.y) as usize;
        let rows = self.rows(&state.query);
        match self.lines(state).get(k)? {
            Line::StopWaiting => Some(PaletteHit::StopWaiting),
            Line::Command(g, c) | Line::Detail(g, c) => {
                rows.iter().position(|r| *r == (*g, *c)).map(PaletteHit::Command)
            }
            _ => None,
        }
    }

    /// A press on what [`CommandPalette::hit`] found: a command runs (not a disabled one, nor any
    /// while another runs); Stop waiting lets the running one go.
    pub fn press(&self, state: &mut PaletteState, hit: PaletteHit) -> PaletteEvent {
        match hit {
            PaletteHit::StopWaiting => PaletteEvent::StopWaiting,
            PaletteHit::Command(_) if state.running.is_some() => PaletteEvent::Ignored,
            PaletteHit::Command(place) => {
                let rows = self.rows(&state.query);
                match rows.get(place).map(|&r| self.command(r)) {
                    Some(cmd) if cmd.disabled.is_none() => {
                        state.active = place;
                        state.refusal = None;
                        PaletteEvent::Run(cmd.id.clone())
                    }
                    _ => PaletteEvent::Ignored,
                }
            }
        }
    }

    /// The pointer over a command: it becomes the active row.
    pub fn hover(&self, state: &mut PaletteState, hit: PaletteHit) {
        if let PaletteHit::Command(place) = hit
            && state.running.is_none()
        {
            let rows = self.rows(&state.query);
            if rows.get(place).is_some_and(|&r| self.command(r).disabled.is_none()) {
                state.active = place;
            }
        }
    }
}

impl CommandPalette<'_> {
    /// Draw it in `area` (from [`CommandPalette::area`]), keeping the active row in view.
    pub fn render(&self, area: Rect, buf: &mut Buffer, state: &mut PaletteState) {
        let area = area.intersection(buf.area);
        state.cursor_at = None;
        if area.width < 12 || area.height < 4 {
            return;
        }
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let p = |piece: &str, field: &str| look.paint(&format!("command-palette.base.{piece}"), field);
        let ground = p("rest.box", "ground").unwrap_or_else(|| look.color("popover"));
        let ink = p("rest.box", "ink").unwrap_or_else(|| look.color("popover-foreground"));
        let edge = p("rest.box", "edge").unwrap_or(ground);
        let quiet = look.color("muted-foreground");
        Clear.render(area, buf);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(look.border_set("radius-surface"))
            .border_style(Style::new().fg(t(edge)).bg(t(look.color("background"))));
        let inner = block.inner(area);
        block.render(area, buf);
        buf.set_style(inner, Style::new().bg(t(ground)).fg(t(ink)));

        // The search: its tint, the icon at its start, the query or the placeholder.
        let sg = p("rest.search", "ground").unwrap_or(ground);
        let search = Rect::new(inner.x, inner.y, inner.width, 1);
        buf.set_style(search, Style::new().bg(t(sg)));
        let icon_ink = p("rest.searchIcon", "ink").unwrap_or(quiet);
        let icon_ink = look
            .faded("command-palette.base.rest.searchIcon", "ink")
            .unwrap_or(icon_ink);
        buf.set_string(
            search.x + 1,
            search.y,
            glyphs.search,
            Style::new().fg(t(icon_ink)).bg(t(sg)),
        );
        let room = search.width.saturating_sub(4) as usize;
        let body = TextRole::Body.style(look).bg(t(sg));
        if state.query.is_empty() {
            buf.set_string(
                search.x + 3,
                search.y,
                text::truncate(&self.words.placeholder, room, glyphs.ellipsis),
                body.fg(t(quiet)),
            );
            state.cursor_at = Some(Position::new(search.x + 3, search.y));
        } else {
            let shown = text::truncate(&state.query, room, glyphs.ellipsis);
            buf.set_string(search.x + 3, search.y, &shown, body.fg(t(ink)));
            state.cursor_at = Some(Position::new(search.x + 3 + text::width(&shown) as u16, search.y));
        }

        // The list: the active row kept in view.
        let refusal_rows = if state.refusal.is_some() { 2 } else { 0 };
        let list = Rect {
            y: inner.y + 2,
            height: inner.height.saturating_sub(2 + refusal_rows + self.foot_rows()),
            ..inner
        };
        let lines = self.lines(state);
        let rows = self.rows(&state.query);
        if rows.is_empty() && !self.released(state) {
            let ei = p("rest.empty", "ink").unwrap_or(ink);
            let w = text::width(&self.words.empty) as u16;
            buf.set_string(
                list.x + list.width.saturating_sub(w) / 2,
                list.y,
                &self.words.empty,
                TextRole::Body.style(look).fg(t(ei)),
            );
        } else {
            let active_line = lines
                .iter()
                .position(|l| match l {
                    Line::Command(g, c) => rows.get(state.active) == Some(&(*g, *c)),
                    Line::StopWaiting => self.released(state) && state.active == 0,
                    _ => false,
                })
                .unwrap_or(0);
            let view = list.height as usize;
            let active_end = active_line + usize::from(matches!(lines.get(active_line + 1), Some(Line::Detail(..))));
            if active_line < state.scroll {
                state.scroll = active_line;
            } else if active_end >= state.scroll + view {
                state.scroll = active_end + 1 - view;
            }
            for (k, line) in lines.iter().enumerate().skip(state.scroll).take(view) {
                let y = list.y + (k - state.scroll) as u16;
                let active = k == active_line || (k == active_line + 1 && matches!(line, Line::Detail(..)));
                self.draw_line(buf, list, y, *line, active, state);
            }
        }

        // The refusal, under the list across the box.
        if let Some(why) = &state.refusal {
            let ri = p("refused.refusal", "ink").unwrap_or_else(|| look.color("critical-ink"));
            let rule = p("refused.refusal", "rule").unwrap_or(edge);
            let ry = inner.bottom() - 2 - self.foot_rows();
            for x in inner.x..inner.right() {
                buf[(x, ry)].set_symbol(glyphs.rule).set_fg(t(rule));
            }
            buf.set_string(inner.x + 1, ry + 1, glyphs.issue_critical, Style::new().fg(t(ri)));
            buf.set_string(
                inner.x + 3,
                ry + 1,
                text::truncate(why, inner.width.saturating_sub(4) as usize, glyphs.ellipsis),
                TextRole::Body.style(look).fg(t(ri)),
            );
        }
        // The foot: a rule, the app's name, the active row's verb beside Enter, the keys.
        if let Some((name, keys)) = &self.foot {
            let rule = p("rest.foot", "rule").unwrap_or(edge);
            let fi = p("rest.foot", "ink").unwrap_or(quiet);
            let fy = inner.bottom() - 2;
            for x in inner.x..inner.right() {
                buf[(x, fy)].set_symbol(glyphs.rule).set_fg(t(rule));
            }
            let help = TextRole::Help.style(look).fg(t(fi));
            let kg = p("rest.keys", "ground").unwrap_or(ground);
            let cap = TextRole::Badge.style(look).fg(t(fi)).bg(t(kg));
            let name = text::truncate(name, inner.width as usize / 3, glyphs.ellipsis);
            buf.set_string(inner.x + 1, fy + 1, &name, help);
            let name_end = inner.x + 1 + text::width(&name) as u16 + 2;
            let mut pieces = vec![(self.words.enter.clone(), self.words.run.clone())];
            if let Some(k) = keys {
                pieces.push((k.label(&self.words.keys), self.words.title.clone()));
            }
            // The pieces kept from the first (Enter's verb) while they fit after the name.
            let width = |(k, w): &(String, String)| text::width(k) as u16 + 2 + 1 + text::width(w) as u16 + 2;
            let mut keep = 0;
            let mut used = 0;
            for piece in &pieces {
                if name_end + used + width(piece) > inner.right() {
                    break;
                }
                used += width(piece);
                keep += 1;
            }
            let mut x = inner.right().saturating_sub(used);
            for (key, words) in pieces.iter().take(keep) {
                buf.set_string(x, fy + 1, format!(" {key} "), cap);
                x += text::width(key) as u16 + 3;
                buf.set_string(x, fy + 1, words, help);
                x += text::width(words) as u16 + 2;
            }
        }
    }

    fn draw_line(&self, buf: &mut Buffer, list: Rect, y: u16, line: Line, active: bool, state: &PaletteState) {
        let look = self.look;
        let glyphs = look.glyphs();
        let t = |c: Rgba| look.terminal(c);
        let p = |piece: &str, field: &str| look.paint(&format!("command-palette.base.{piece}"), field);
        let ground = p("rest.box", "ground").unwrap_or_else(|| look.color("popover"));
        let quiet = look.color("muted-foreground");
        match line {
            Line::Separator => {
                let rule = p("rest.separator", "ground").unwrap_or(quiet);
                for x in list.x..list.right() {
                    buf[(x, y)].set_symbol(glyphs.rule).set_fg(t(rule));
                }
            }
            Line::Title(g) => {
                let ti = p("rest.groupTitle", "ink").unwrap_or(quiet);
                buf.set_string(
                    list.x + 1,
                    y,
                    text::truncate(
                        &self.groups[g].title,
                        list.width.saturating_sub(2) as usize,
                        glyphs.ellipsis,
                    ),
                    TextRole::Badge.style(look).fg(t(ti)),
                );
            }
            Line::StopWaiting => {
                let state_key = if active { "active" } else { "rest" };
                let rg = p(&format!("{state_key}.row"), "ground").unwrap_or(ground);
                let ri = p(&format!("{state_key}.row"), "ink").unwrap_or(quiet);
                buf.set_style(Rect::new(list.x, y, list.width, 1), Style::new().bg(t(rg)));
                buf.set_string(
                    list.x + 3,
                    y,
                    &self.words.stop_waiting,
                    TextRole::Body.style(look).fg(t(ri)),
                );
            }
            Line::Detail(g, c) => {
                let cmd = &self.groups[g].commands[c];
                let st = if active { "active" } else { "rest" };
                let rg = p(&format!("{st}.row"), "ground").unwrap_or(ground);
                let hi = p(&format!("{st}.hint"), "ink").unwrap_or(quiet);
                buf.set_style(Rect::new(list.x, y, list.width, 1), Style::new().bg(t(rg)));
                let x = list.x + 1 + cmd.icon.as_ref().map_or(2, |i| text::width(i) as u16 + 1);
                let room = list.right().saturating_sub(x + 1) as usize;
                if let Some(d) = &cmd.detail {
                    buf.set_string(
                        x,
                        y,
                        text::truncate(d, room, glyphs.ellipsis),
                        TextRole::Help.style(look).fg(t(hi)).bg(t(rg)),
                    );
                }
            }
            Line::Command(g, c) => {
                let cmd = &self.groups[g].commands[c];
                let disabled = cmd.disabled.is_some();
                let st = if disabled {
                    "disabled"
                } else if active {
                    "active"
                } else {
                    "rest"
                };
                let piece = |name: &str| {
                    let key = format!("{}.{name}", if active { "active" } else { "rest" });
                    p(&key, "ink").unwrap_or(quiet)
                };
                let rg = p(&format!("{st}.row"), "ground").unwrap_or(ground);
                let mut ri = p(&format!("{st}.row"), "ink").unwrap_or(quiet);
                if disabled {
                    ri = look.faded("command-palette.base.disabled.row", "ink").unwrap_or(ri);
                }
                let fade = |c: Rgba| if disabled { ri } else { c };
                buf.set_style(Rect::new(list.x, y, list.width, 1), Style::new().bg(t(rg)));
                let row_style = Style::new().bg(t(rg));
                let mut x = list.x + 1;
                if let Some(icon) = &cmd.icon {
                    buf.set_string(x, y, icon, row_style.fg(t(fade(piece("icon")))));
                    x += text::width(icon) as u16 + 1;
                } else {
                    x += 2;
                }
                // The row's end: a spinner while it runs, else the meta, the kind and the keys.
                let mut end = list.right().saturating_sub(1);
                let running = state.running.as_ref().is_some_and(|(id, _)| *id == cmd.id);
                if running {
                    let si = p("busy.spinner", "ink").unwrap_or(quiet);
                    let ms = state.running.as_ref().map_or(0, |r| r.1);
                    end = end.saturating_sub(1);
                    buf.set_string(end, y, look.spinner_frame(ms), row_style.fg(t(si)));
                    end = end.saturating_sub(1);
                } else if let Some(keys) = &cmd.keys {
                    let kg = p("rest.keys", "ground").unwrap_or(rg);
                    let ki = p("rest.keys", "ink").unwrap_or(quiet);
                    let label = keys.label(&self.words.keys);
                    let w = text::width(&label) as u16 + 2;
                    end = end.saturating_sub(w);
                    buf.set_string(
                        end,
                        y,
                        format!(" {label} "),
                        TextRole::Badge.style(look).fg(t(ki)).bg(t(kg)),
                    );
                    end = end.saturating_sub(1);
                }
                if let Some(kind) = &cmd.kind {
                    let w = text::width(kind) as u16;
                    end = end.saturating_sub(w);
                    buf.set_string(
                        end,
                        y,
                        kind,
                        TextRole::Help.style(look).patch(row_style).fg(t(fade(piece("kind")))),
                    );
                    end = end.saturating_sub(1);
                }
                if let Some(meta) = &cmd.meta {
                    let mi = match (cmd.meta_tone, active) {
                        (Some(tone), _) => p(&format!("{}.meta", tone.key()), "ink").unwrap_or(quiet),
                        (None, _) => piece("meta"),
                    };
                    let w = text::width(meta) as u16;
                    end = end.saturating_sub(w);
                    buf.set_string(
                        end,
                        y,
                        meta,
                        TextRole::Help.style(look).patch(row_style).fg(t(fade(mi))),
                    );
                    end = end.saturating_sub(2);
                }
                // The label, then the hint, the code, or a disabled command's reason.
                let room = end.saturating_sub(x) as usize;
                let label = text::truncate(&cmd.label, room, glyphs.ellipsis);
                buf.set_string(x, y, &label, TextRole::Body.style(look).patch(row_style).fg(t(ri)));
                x += text::width(&label) as u16 + 1;
                let after = cmd.disabled.as_ref().or(cmd.hint.as_ref()).or(cmd.code.as_ref());
                if let Some(words) = after {
                    let room = end.saturating_sub(x) as usize;
                    if room > 2 {
                        buf.set_string(
                            x,
                            y,
                            text::truncate(words, room, glyphs.ellipsis),
                            TextRole::Help.style(look).patch(row_style).fg(t(fade(piece("hint")))),
                        );
                    }
                }
            }
        }
    }
}
