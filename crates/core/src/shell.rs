//! The shell, from sokkel-design `patterns/shell`: where the frame's parts go.
//!
//! The rail runs the window's full height at its start, its head as tall as the top bar and its
//! foot cell level with the status line, so head and bar, foot cell and status line read as one
//! strip each across the window. Below md (90 columns) the rail is a drawer over the content from
//! the start, drawn expanded. The page sits in the content at the page inset, a blank row on top at
//! regular (the pattern's `cells` note).

use crate::cells::{Width, by_density, page_inset};
use crate::keys::{Key, KeyPress, Pointer, PointerKind};
use crate::look::Look;
use crate::rail::Rail;
use crate::status_line::StatusLine;
use crate::top_bar::TopBar;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Clear, Widget};

/// Where the frame's parts go in a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellLayout {
    /// The rail in the frame; `None` below md, where it is a drawer.
    pub rail: Option<Rect>,
    pub top: Rect,
    pub content: Rect,
    pub foot: Rect,
    pub width: Width,
}

impl ShellLayout {
    /// The frame in `window`, the rail `rail_width` columns wide where it stands in the frame.
    pub fn new(look: &Look, window: Rect, rail_width: u16) -> ShellLayout {
        let width = Width::of(window);
        let rail_w = if width == Width::Narrow {
            0
        } else {
            rail_width.min(window.width / 2)
        };
        let main = Rect {
            x: window.x + rail_w,
            width: window.width - rail_w,
            ..window
        };
        let top_h = TopBar::height(look).min(main.height);
        let foot_h = StatusLine::height(look).min(main.height.saturating_sub(top_h));
        ShellLayout {
            rail: (rail_w > 0).then_some(Rect {
                width: rail_w,
                ..window
            }),
            top: Rect { height: top_h, ..main },
            content: Rect {
                y: main.y + top_h,
                height: main.height - top_h - foot_h,
                ..main
            },
            foot: Rect {
                y: main.bottom() - foot_h,
                height: foot_h,
                ..main
            },
            width,
        }
    }

    /// The drawer below md: from the window's start, the rail's expanded width, at most the
    /// window less four columns.
    pub fn drawer(window: Rect, rail_width: u16) -> Rect {
        Rect {
            width: rail_width.min(window.width.saturating_sub(4)),
            ..window
        }
    }

    /// The page in the content: the page inset across, a blank row on top at regular.
    pub fn page(&self, look: &Look) -> Rect {
        let inset = page_inset(look, self.content);
        let top = by_density(look, 1, 0);
        Rect {
            x: self.content.x + inset,
            y: self.content.y + top,
            width: self.content.width.saturating_sub(2 * inset),
            height: self.content.height.saturating_sub(top),
        }
    }
}

/// The frame an app lives in: the rail, the top bar and the status line, laid out by
/// [`ShellLayout`], with the rail collapsing and, below md, opening as a drawer. Build it each frame
/// and draw it with its [`ShellState`]; it gives back the page's rectangle.
#[derive(Debug, Clone)]
pub struct Shell<'a> {
    look: &'a Look,
    rail: Rail<'a>,
    top: TopBar<'a>,
    foot: StatusLine<'a>,
}

/// What lives between frames: the rail collapsed or not, the drawer open or not, and where the
/// last frame put what a press can hit.
#[derive(Debug, Clone, Default)]
pub struct ShellState {
    pub collapsed: bool,
    /// Below md: the rail open over the page.
    pub drawer: bool,
    window: Rect,
    narrow: bool,
    entries: Vec<((usize, usize), Rect)>,
    collapse: Option<Rect>,
    menu: Option<Rect>,
    search: Option<Rect>,
    palette: Option<Rect>,
    drawer_at: Option<Rect>,
}

/// What a key or a press did to the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellEvent {
    /// Not the frame's: the page's to answer.
    Ignored,
    /// The frame changed (collapsed, expanded, the drawer opened or closed).
    Changed,
    /// A rail entry was pressed, by (group, entry): the app goes there.
    Entry(usize, usize),
    /// The top bar's search was pressed.
    Search,
    /// The top bar's command palette button was pressed.
    Palette,
}

impl<'a> Shell<'a> {
    pub fn new(look: &'a Look, rail: Rail<'a>, top: TopBar<'a>, foot: StatusLine<'a>) -> Shell<'a> {
        Shell { look, rail, top, foot }
    }

    /// Draw the frame in `window` and give back the page, where the app draws.
    pub fn render(&self, window: Rect, buf: &mut Buffer, state: &mut ShellState) -> Rect {
        let look = self.look;
        buf.set_style(
            window,
            Style::new().bg(look.token("background")).fg(look.token("foreground")),
        );
        let rail = self.rail.clone().collapsed(state.collapsed, true);
        let shell = ShellLayout::new(look, window, rail.width());
        state.window = window;
        state.narrow = shell.width == Width::Narrow;
        if !state.narrow {
            state.drawer = false;
        }
        state.entries.clear();
        state.collapse = None;
        state.drawer_at = None;
        if let Some(r) = shell.rail {
            (&rail).render(r, buf);
            let l = rail.layout(r, 0);
            state.entries = l.entries;
            state.collapse = l.collapse;
        }
        let top = self.top.clone().window(shell.width);
        let l = top.layout(shell.top);
        (state.menu, state.search, state.palette) = (l.menu, l.search, l.palette);
        (&top).render(shell.top, buf);
        (&self.foot).render(shell.foot, buf);
        shell.page(look)
    }

    /// The drawer, over what the page drew: call it after the page, before the app's layers.
    pub fn render_drawer(&self, buf: &mut Buffer, state: &mut ShellState) {
        if !(state.drawer && state.narrow) {
            return;
        }
        let rail = self.rail.clone().collapsed(false, false);
        let r = ShellLayout::drawer(state.window, rail.width());
        Clear.render(r, buf);
        (&rail).render(r, buf);
        state.entries = rail.layout(r, 0).entries;
        state.drawer_at = Some(r);
    }
}

impl ShellState {
    /// A key the frame answers: Escape closes the drawer.
    pub fn key(&mut self, key: KeyPress) -> ShellEvent {
        if self.drawer && key.is(Key::Escape) {
            self.drawer = false;
            return ShellEvent::Changed;
        }
        ShellEvent::Ignored
    }

    /// Open or close the drawer (the menu button's press, or an app's key).
    pub fn toggle_drawer(&mut self) {
        self.drawer = self.narrow && !self.drawer;
    }

    /// A pointer event on the frame: the rail's collapse, the menu button, an entry, the search,
    /// the palette button; with the drawer open, a press outside it closes it.
    pub fn pointer(&mut self, p: Pointer) -> ShellEvent {
        if p.kind != PointerKind::Press {
            return ShellEvent::Ignored;
        }
        let at = p.at;
        let hit = |r: Option<Rect>| r.is_some_and(|r| r.contains(at));
        let entry = self.entries.iter().find(|(_, r)| r.contains(at)).map(|(e, _)| *e);
        if let Some(drawer) = self.drawer_at {
            self.drawer = false;
            return match entry {
                Some((g, e)) => ShellEvent::Entry(g, e),
                None if drawer.contains(at) => {
                    self.drawer = true;
                    ShellEvent::Ignored
                }
                None => ShellEvent::Changed,
            };
        }
        if hit(self.collapse) {
            self.collapsed = !self.collapsed;
            return ShellEvent::Changed;
        }
        if hit(self.menu) {
            self.toggle_drawer();
            return ShellEvent::Changed;
        }
        if let Some((g, e)) = entry {
            return ShellEvent::Entry(g, e);
        }
        if hit(self.search) {
            return ShellEvent::Search;
        }
        if hit(self.palette) {
            return ShellEvent::Palette;
        }
        ShellEvent::Ignored
    }

    /// Whether a press at `at` lands on the open drawer, which stands over the page.
    pub fn covers(&self, at: Position) -> bool {
        self.drawer_at.is_some_and(|r| r.contains(at))
    }
}
