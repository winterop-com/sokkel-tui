//! The app runtime: the terminal taken and given back, a frame each `motion-fast` so spinners turn,
//! keys and the pointer handed on, the mouse captured while the app wants it.
//!
//! Two levels:
//! - [`Program`] and [`run`]: one screen that draws itself and answers keys and the pointer.
//! - [`ShellApp`] and [`run_shell`]: an app in sokkel's frame. The runtime draws the shell (the
//!   rail, the top bar, the status line, the drawer below md) and owns what every app has: the
//!   command palette (Ctrl+K), the settings dialog (`,` or the palette's Settings) with the look
//!   and the frame in it, the rail collapsing, the drawer, and the mouse switched on and off. The
//!   app gives its parts, its page, its commands and settings, and answers what they do.

use crate::cells::Width;
use crate::dialog::{Dialog, DialogFocus, DialogWidth};
use crate::focus::{Focus, FocusId};
use crate::glyphs::Charset;
use crate::keys::{Key, KeyPress, Pointer, PointerKind};
use crate::look::{Depth, Look};
use crate::menu::{DropdownMenu, MenuEvent, MenuState};
use crate::palette::{Command, CommandGroup, CommandPalette, PaletteEvent, PaletteState, PaletteWords};
use crate::popup::{Item, PopupRow};
use crate::rail::Rail;
use crate::settings::{
    Setting, SettingsDialog, SettingsEvent, SettingsGroup, SettingsSection, SettingsState, look_chose, look_section,
};
use crate::shell::{Shell, ShellEvent, ShellState};
use crate::shortcuts::{Binding, KeyContext, Keys, ShortcutWords, Shortcuts};
use crate::status_line::StatusLine;
use crate::text::TextRole;
use crate::tokens::{self, Density, Scheme};
use crate::top_bar::TopBar;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event,
};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::widgets::Widget;
use std::time::{Duration, Instant};

/// One screen the runtime drives.
pub trait Program {
    fn look(&self) -> &Look;
    fn draw(&mut self, f: &mut Frame);
    /// A key; true quits.
    fn key(&mut self, key: KeyPress) -> bool;
    fn pointer(&mut self, _p: Pointer) {}
    /// Text pasted into the terminal (bracketed paste), whole.
    fn paste(&mut self, _text: &str) {}
    fn tick(&mut self, _now_ms: u64) {}
    /// Whether it is done, after a press or a tick (a key says so by returning true).
    fn done(&self) -> bool {
        false
    }
    /// Whether the mouse is captured. Captured, the terminal's own selection is off.
    fn mouse(&self) -> bool {
        true
    }
}

/// The look a program starts in: `theme`, dark unless `light` is among the arguments, the
/// terminal's color depth and character set.
pub fn look_from_args(theme: &str) -> Look {
    let light = std::env::args().any(|a| a == "light");
    Look::new(theme, if light { Scheme::Light } else { Scheme::Dark })
        .unwrap_or_else(|| panic!("the contract has no theme {theme}"))
        .with_depth(Depth::detect())
        .with_charset(Charset::detect())
}

/// Ctrl+Q and Ctrl+C quit every program.
pub fn quits(key: KeyPress) -> bool {
    key.ctrl && matches!(key.key, Key::Char('q') | Key::Char('c'))
}

/// Run `program` in the terminal until it quits, and give the terminal back.
pub fn run(program: &mut impl Program) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut mouse = program.mouse();
    execute!(std::io::stdout(), EnableBracketedPaste)?;
    if mouse {
        execute!(std::io::stdout(), EnableMouseCapture)?;
    }
    let start = Instant::now();
    let result = (|| -> std::io::Result<()> {
        loop {
            program.tick(start.elapsed().as_millis() as u64);
            if program.done() {
                return Ok(());
            }
            terminal.draw(|f| {
                program.draw(f);
                program.look().finish(f.buffer_mut());
            })?;
            let wait = Duration::from_millis(program.look().duration_ms("motion-fast"));
            if event::poll(wait)? {
                match event::read()? {
                    Event::Key(k) => {
                        if let Some(key) = KeyPress::from_crossterm(&k)
                            && (quits(key) || program.key(key))
                        {
                            return Ok(());
                        }
                    }
                    Event::Mouse(m) if mouse => {
                        if let Some(p) = Pointer::from_crossterm(&m) {
                            program.pointer(p);
                        }
                    }
                    Event::Paste(text) => program.paste(&text),
                    _ => {}
                }
            }
            if program.mouse() != mouse {
                mouse = !mouse;
                if mouse {
                    execute!(std::io::stdout(), EnableMouseCapture)?;
                } else {
                    execute!(std::io::stdout(), DisableMouseCapture)?;
                }
            }
        }
    })();
    if mouse {
        execute!(std::io::stdout(), DisableMouseCapture)?;
    }
    execute!(std::io::stdout(), DisableBracketedPaste)?;
    ratatui::restore();
    result
}

/// The frame's parts an app gives each frame.
/// The app's search's answer to one query: its results as groups (a group a kind), or failed.
#[derive(Debug, Clone, Default)]
pub struct SearchAnswer {
    pub query: String,
    pub groups: Vec<CommandGroup>,
    pub failed: bool,
}

pub struct Chrome<'a> {
    pub rail: Rail<'a>,
    pub top: TopBar<'a>,
    pub foot: StatusLine<'a>,
}

/// The rows an app adds to the account menu, each with the id its command answers to
/// ([`ShellApp::command`]), and the row that leaves (Sign out, or Sign in where no one is).
#[derive(Debug, Clone, Default)]
pub struct Account {
    pub rows: Vec<(String, Item)>,
    pub leave: Option<(String, Item)>,
}

/// The account menu's words, with sokkel-ui's English defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountWords {
    pub settings: String,
    pub appearance: String,
    pub shortcuts: String,
    pub scheme: String,
    pub light: String,
    pub dark: String,
}

impl Default for AccountWords {
    fn default() -> Self {
        AccountWords {
            settings: "Settings".into(),
            appearance: "Appearance".into(),
            shortcuts: "Keyboard shortcuts".into(),
            scheme: "Scheme".into(),
            light: "Light".into(),
            dark: "Dark".into(),
        }
    }
}

/// What the runtime hands the app with each frame and event: the look, the window, the one
/// keyboard focus the frame and the page share (the page names its stops in it while it draws),
/// where the pointer is, whether the page has the reader (no layer over it), and the time.
pub struct Cx<'a> {
    pub look: &'a Look,
    pub window: Rect,
    pub focus: &'a mut Focus,
    pub hover: Option<Position>,
    pub active: bool,
    pub now: u64,
}

/// What running one of the app's commands did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ran {
    /// Done: the palette closes.
    Done,
    /// Refused, with why: the palette stays open and says so.
    Refused(Option<String>),
    /// Still going: the palette shows it running (Stop waiting after `busy-release`) until
    /// [`ShellApp::finished`] says it is done.
    Running,
}

/// What an app's key did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Not the app's: the runtime's defaults answer it.
    Ignored,
    Handled,
    Quit,
}

/// An app in sokkel's frame. Only [`ShellApp::chrome`] and [`ShellApp::page`] are required.
pub trait ShellApp {
    /// Whether the app starts full screen (a wall display): the reader leaves it with Escape or the
    /// palette.
    fn kiosk(&self) -> bool {
        false
    }
    /// The app's name for what it keeps between runs (its look, under [`kept_path`]); None keeps
    /// nothing.
    fn id(&self) -> Option<&str> {
        None
    }
    /// Where the app is, as a route it can go back to (`runs`, `jobs/nightly-export`). The runtime
    /// keeps the history: Escape with nothing open goes back (the settings close first, as
    /// Escape closes any layer). Empty keeps none.
    fn route(&self) -> String {
        String::new()
    }
    /// Go to a route the app gave before (Back).
    fn go(&mut self, _route: &str) {}
    /// A banner under the top bar, for a state that holds (the server not answering): its tone and
    /// its words, this frame. None takes no row.
    fn banner(&self) -> Option<(crate::banner::BannerTone, String)> {
        None
    }
    /// The rail, the top bar and the status line, this frame.
    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a>;
    /// Draw the page in `area`, naming its stops in `cx.focus` as it draws them (after the
    /// frame's: the rail's entries, the search, the palette button).
    fn page(&mut self, f: &mut Frame, area: Rect, cx: &mut Cx);
    /// A side panel at the page's end while this says how wide it is, in columns (40, 320 px, is
    /// the usual): the runtime makes the room and [`ShellApp::panel`] draws in it. Below md it
    /// covers the page.
    fn panel_width(&self) -> Option<u16> {
        None
    }
    /// Draw the side panel's content in `area` (a [`crate::DetailPane`], say), after the page,
    /// naming its stops in `cx.focus`.
    fn panel(&mut self, _f: &mut Frame, _area: Rect, _cx: &mut Cx) {}
    /// The rows the page needs at `width` columns, when it is taller than the window can be: the
    /// runtime then scrolls it (the wheel, PageUp and PageDown, focus kept in view) and draws a
    /// scroller at its end. None: the page fits whatever it is given.
    fn page_rows(&self, _look: &Look, _width: u16) -> Option<u16> {
        None
    }
    /// Whether the app has a layer of its own open (a dialog, a menu): keys and presses go to
    /// it first, and the frame's shortcuts keep quiet.
    fn layer_open(&self) -> bool {
        false
    }
    /// Draw the app's open layer, over the frame and the drawer.
    fn layer(&mut self, _f: &mut Frame, _cx: &mut Cx) {}
    /// Whether the app's open layer stands over a scrim (a dialog does; a menu does not): the
    /// runtime lays what is under it under the scrim first.
    fn layer_scrim(&self) -> bool {
        false
    }
    /// The app's commands, shown in the palette before the runtime's own.
    fn commands(&self) -> Vec<CommandGroup> {
        Vec::new()
    }
    /// A command of the app's was run (from the palette, a shortcut or an account row).
    fn command(&mut self, _id: &str) -> Ran {
        Ran::Done
    }
    /// Whether a command that answered [`Ran::Running`] is done.
    fn finished(&self, _id: &str) -> bool {
        true
    }
    /// The reader stopped waiting for a running command: the app lets it go on, or stops it.
    fn stop_waiting(&mut self, _id: &str) {}
    /// The palette's query, for the app's own search (`search`), asked once typing has paused
    /// (`debounce`, 150 ms) and the query is not empty. The app answers through
    /// [`ShellApp::palette_answer`], now or on a later tick.
    fn palette_search(&mut self, _query: &str) {}
    /// What the app's search has answered: the query it answers and its results, by kind, or
    /// that it failed. The runtime shows them under the commands; an earlier query's stay out of
    /// reach while the next one waits.
    fn palette_answer(&self) -> Option<SearchAnswer> {
        None
    }
    /// One of the app's search results was chosen, by its command id: the palette closes.
    fn palette_open(&mut self, _id: &str) {}
    /// The app's settings sections, after the runtime's Look and Frame.
    fn settings(&self, _look: &Look) -> Vec<SettingsSection> {
        Vec::new()
    }
    /// A choice in one of the app's settings turned.
    fn chose(&mut self, _id: &str, _option: usize) {}
    /// A rail entry was pressed, by (group, entry).
    fn entry(&mut self, _group: usize, _entry: usize) {}
    /// The app's shortcuts, each answering to a command id ([`ShellApp::command`]). The runtime's
    /// own come first: Ctrl+K the palette, `?` the shortcuts, `/` the search, `,` the settings.
    fn shortcuts(&self) -> Vec<Binding<String>> {
        Vec::new()
    }
    /// Whether the reader is typing in one of the page's fields: shortcuts are quiet then.
    fn typing(&self) -> bool {
        false
    }
    /// The top bar's search holds `query` now.
    fn search(&mut self, _query: &str) {}
    /// The app's rows in the account menu, and its Sign out or Sign in.
    fn account(&self) -> Account {
        Account::default()
    }
    /// A key for the page, or for the app's open layer: `cx.focus` says which of its stops has
    /// focus. Tab and the frame's own keys are the runtime's.
    fn key(&mut self, _key: KeyPress, _cx: &mut Cx) -> Flow {
        Flow::Ignored
    }
    /// A pointer event on the page or on the app's open layer; a press has already focused the
    /// stop under it.
    fn pointer(&mut self, _p: Pointer, _cx: &mut Cx) {}
    /// Text pasted while the page or the app's open layer has the keys (a field takes it with
    /// [`crate::text_field::FieldState::paste`]).
    fn paste(&mut self, _text: &str, _cx: &mut Cx) {}
    fn tick(&mut self, _now_ms: u64) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    None,
    Palette,
    Settings,
    Account,
    Shortcuts,
    /// A collapsed parent's children, beside the rail: by its group and place.
    Flyout(usize, usize),
}

/// What a shortcut does: the runtime's own, or the app's command.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Act {
    Palette,
    Shortcuts,
    Search,
    Settings,
    App(String),
}

/// The words of the runtime's own shortcuts and the shortcuts dialog.
const WORDS_PALETTE: &str = "Open the command palette";
const WORDS_SHORTCUTS: &str = "Keyboard shortcuts";
const WORDS_SEARCH: &str = "Search";
const WORDS_SETTINGS: &str = "Settings";
const WORDS_TAB: &str = "Move focus";
const WORDS_BACK: &str = "Back";
const WORDS_QUIT: &str = "Quit";

/// What a row of the account menu does.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AccountAct {
    None,
    Settings,
    Appearance,
    Shortcuts,
    Scheme(Scheme),
    App(String),
}

/// The runtime's own commands and settings, by id.
const SETTINGS: &str = "sokkel.settings";
const SCHEME: &str = "sokkel.scheme";
const DENSITY: &str = "sokkel.density";
const RAIL: &str = "sokkel.rail";
const MOUSE: &str = "sokkel.mouse";
const KIOSK: &str = "sokkel.kiosk";
const THEME: &str = "sokkel.theme:";
const SEARCH_RETRY: &str = "sokkel.search.retry";
/// The pause in typing before the palette's query goes to the app's search (`debounce`).
const DEBOUNCE_MS: u64 = 150;

/// The frame's own stops.
fn search_stop() -> FocusId {
    FocusId::new("sokkel.search")
}
fn palette_stop() -> FocusId {
    FocusId::new("sokkel.palette")
}
fn account_stop() -> FocusId {
    FocusId::new("sokkel.account")
}
fn edge_stop() -> FocusId {
    FocusId::new("sokkel.rail-edge")
}
fn entry_stop(g: usize, e: usize) -> FocusId {
    FocusId::new("sokkel.rail").child((g as u64) << 32 | e as u64)
}

/// A [`ShellApp`] with the frame the runtime keeps for it.
struct Host<A> {
    app: A,
    look: Look,
    shell: ShellState,
    layer: Layer,
    palette: PaletteState,
    palette_at: Option<Rect>,
    settings: SettingsState,
    settings_at: Option<Rect>,
    width: Width,
    mouse: bool,
    account: MenuState,
    flyout: MenuState,
    window: Rect,
    keys: Shortcuts<Act>,
    now: u64,
    /// The app's command still running, and since when.
    running: Option<(String, u64)>,
    /// The palette's query as last seen and when it changed, and the query last asked of the
    /// app's search and when.
    typed: (String, u64),
    asked: Option<(String, u64)>,
    /// Where the look is kept, and the look last kept there.
    kept: Option<std::path::PathBuf>,
    worn: (String, Scheme, Density),
    /// The rail's width as last kept (a width its edge left).
    rail_kept: Option<u16>,
    focus: Focus,
    hover: Option<Position>,
    /// The routes behind this one, and the one shown.
    history: Vec<String>,
    route: String,
    going_back: bool,
    /// The page's first stop gets focus after this frame (a route change).
    to_page: bool,
    page_mark: usize,
    /// What had focus when a layer opened, to get it back when it closes.
    opened_from: Option<FocusId>,
    was_open: bool,
    /// The page's scroll, its view on screen, and its rows.
    scroll: u16,
    view: Rect,
    rows: u16,
}

/// Run `app` in sokkel's frame, starting in `look`, or in the look it kept when it has an
/// [`ShellApp::id`] and no `light` or `dark` among the arguments.
pub fn run_shell(app: impl ShellApp, look: Look) -> std::io::Result<()> {
    let kept = app.id().and_then(kept_path);
    let asked = std::env::args().any(|a| a == "light" || a == "dark");
    let look = match &kept {
        Some(path) if !asked => read_look(path, &look).unwrap_or(look),
        _ => look,
    };
    let rail = kept.as_deref().and_then(read_rail_size);
    let mut host = Host::new(app, look, kept);
    host.shell.rail_size = rail;
    host.rail_kept = rail;
    run(&mut host)
}

/// One frame of `app` in a `cols` by `rows` window, after `setup` (the rail collapsed, the
/// drawer open) and the `keys` given, a tenth of a second apart: what a test or a specimen looks
/// at, with nothing kept.
pub fn shot(
    app: impl ShellApp,
    look: Look,
    (cols, rows): (u16, u16),
    setup: impl FnOnce(&mut ShellState),
    keys: &[KeyPress],
) -> Buffer {
    let mut host = Host::new(app, look, None);
    setup(&mut host.shell);
    let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(cols, rows)).expect("a test terminal");
    let frame = |term: &mut ratatui::Terminal<ratatui::backend::TestBackend>, host: &mut Host<_>| {
        term.draw(|f| {
            host.draw(f);
            host.look.finish(f.buffer_mut());
        })
        .expect("a frame");
    };
    frame(&mut term, &mut host);
    for (n, key) in keys.iter().enumerate() {
        host.tick(n as u64 * 100);
        host.key(*key);
        frame(&mut term, &mut host);
    }
    term.backend().buffer().clone()
}

impl<A: ShellApp> Host<A> {
    fn new(app: A, look: Look, kept: Option<std::path::PathBuf>) -> Host<A> {
        let mut host = Host {
            kept,
            worn: (look.theme().to_string(), look.scheme(), look.density()),
            rail_kept: None,
            app,
            look,
            shell: ShellState::default(),
            layer: Layer::None,
            palette: PaletteState::default(),
            palette_at: None,
            settings: SettingsState::default(),
            settings_at: None,
            width: Width::Wide,
            mouse: true,
            account: MenuState::default(),
            flyout: MenuState::default(),
            window: Rect::default(),
            keys: Shortcuts::new(),
            now: 0,
            running: None,
            typed: (String::new(), 0),
            asked: None,
            focus: Focus::new(),
            hover: None,
            history: Vec::new(),
            route: String::new(),
            going_back: false,
            to_page: false,
            page_mark: 0,
            opened_from: None,
            was_open: false,
            scroll: 0,
            view: Rect::default(),
            rows: 0,
        };
        host.route = host.app.route();
        host.shell.kiosk = host.app.kiosk();
        host.keys = host.registry();
        host
    }

    /// A rail entry chosen: a collapsed parent opens its flyout, any other goes to the app.
    fn entry(&mut self, g: usize, e: usize) {
        let rail = self.app.chrome(&self.look).rail;
        let children = rail.children(g, e);
        // A collapsed rail or a strip opens a parent's children in a flyout beside it.
        if (self.shell.collapsed || rail.is_strip()) && !self.shell.drawer && !children.is_empty() {
            self.layer = Layer::Flyout(g, e);
            self.flyout = MenuState {
                open: true,
                highlight: 0,
                sub: None,
            };
            return;
        }
        self.shell.drawer = false;
        self.app.entry(g, e);
    }

    /// The flyout's menu: the parent's children, and the anchor it opens at, beside the rail.
    fn flyout_menu(&self, g: usize, e: usize) -> (DropdownMenu<'_>, Vec<usize>, Rect) {
        let children = self.app.chrome(&self.look).rail.children(g, e);
        let at = self
            .shell
            .entries()
            .iter()
            .find(|(k, _)| *k == (g, e))
            .map_or(Rect::default(), |(_, r)| *r);
        let anchor = Rect::new(at.right() + 1, at.y.saturating_sub(1), 1, 1);
        let places = children.iter().map(|(k, _)| *k).collect();
        let rows = children.into_iter().map(|(_, label)| Item::new(label).into());
        (DropdownMenu::new(&self.look, rows), places, anchor)
    }

    fn flyout_event(&mut self, g: usize, event: MenuEvent, places: &[usize]) {
        match event {
            MenuEvent::Closed => self.layer = Layer::None,
            MenuEvent::Action(i, _) => {
                self.layer = Layer::None;
                if let Some(&k) = places.get(i) {
                    self.app.entry(g, k);
                }
            }
            _ => {}
        }
    }

    /// Back: the settings close first, else the route before this one.
    fn back(&mut self) -> bool {
        if self.layer == Layer::Settings {
            self.layer = Layer::None;
            return true;
        }
        match self.history.pop() {
            Some(route) => {
                self.going_back = true;
                self.app.go(&route);
                true
            }
            None => false,
        }
    }

    /// The frame and the page, once: the shell, the frame's stops, the page with its own, the
    /// account last, the drawer, the app's layer.
    fn draw_frame(&mut self, f: &mut Frame) {
        // A new route: the one left goes on the history (unless Back left it), and focus moves to
        // the new page once it has named its stops.
        let route = self.app.route();
        if route != self.route {
            let left = std::mem::replace(&mut self.route, route);
            if !self.going_back && !left.is_empty() {
                self.history.push(left);
            }
            self.to_page = true;
        }
        self.going_back = false;
        // A layer opening keeps what had focus; closing gives it back.
        let open = self.layer != Layer::None || self.app.layer_open();
        if open && !self.was_open {
            self.opened_from = self.focus.current();
        } else if !open
            && self.was_open
            && let Some(id) = self.opened_from.take()
        {
            self.focus.set(id);
        }
        self.was_open = open;
        let look = &self.look;
        let window = f.area();
        // The ring goes where the keyboard left focus, from the last frame's stops.
        self.shell.rail_focus = self
            .shell
            .entries()
            .iter()
            .map(|(e, _)| *e)
            .find(|&(g, e)| self.focus.ring(entry_stop(g, e)));
        self.shell.edge_focus = self.focus.ring(edge_stop());
        self.shell.top_focus = if self.focus.ring(palette_stop()) {
            crate::top_bar::TopBarFocus::Palette
        } else {
            crate::top_bar::TopBarFocus::None
        };
        self.shell.note = self
            .keys
            .pending()
            .map(|c| format!("{} {} …", c.to_uppercase(), ShortcutWords::default().then));
        let page = {
            let chrome = self.app.chrome(look);
            let banner = self
                .app
                .banner()
                .map(|(tone, words)| crate::banner::Banner::new(look, tone, words));
            Shell::new(look, chrome.rail, chrome.top, chrome.foot)
                .banner(banner)
                .panel(self.app.panel_width())
                .render(window, f.buffer_mut(), &mut self.shell)
        };
        self.focus.begin();
        if !self.shell.drawer {
            for &((g, e), r) in self.shell.entries() {
                self.focus.stop(entry_stop(g, e), r);
            }
            // The rail's edge, after its entries: keys resize the rail.
            if let Some(r) = self.shell.edge_area() {
                self.focus.stop(edge_stop(), r);
            }
        }
        if let Some(r) = self.shell.search_area() {
            self.focus.text_stop(search_stop(), r);
        }
        if let Some(r) = self.shell.palette_area() {
            self.focus.stop(palette_stop(), r);
        }
        let active = self.layer == Layer::None && !self.shell.searching && !self.shell.drawer;
        let mut cx = Cx {
            look,
            window,
            focus: &mut self.focus,
            hover: self.hover.filter(|_| active),
            active,
            now: self.now,
        };
        self.page_mark = cx.focus.mark();
        // A page taller than its view is drawn whole off screen, its stops named where they stand
        // in it, and shown from the scroll; presses are moved by the scroll to match.
        let rows = self
            .app
            .page_rows(look, page.width)
            .unwrap_or(page.height)
            .max(page.height);
        // A page with no room (below md the side panel covers it) is not drawn, nor its scroller.
        let rows = if page.height == 0 || page.width == 0 { 0 } else { rows };
        self.rows = rows;
        self.view = page;
        if rows == 0 {
            self.scroll = 0;
        } else if rows > page.height {
            self.scroll = self.scroll.min(rows - page.height);
            cx.hover = cx
                .hover
                .filter(|p| page.contains(*p))
                .map(|p| Position::new(p.x, p.y + self.scroll));
            let tall = Rect { height: rows, ..page };
            let whole = Rect::new(0, 0, window.width, page.y + rows);
            let mut off = ratatui::Terminal::new(ratatui::backend::TestBackend::new(whole.width, whole.height))
                .expect("an off-screen page");
            let ground = Style::new().bg(look.token("background")).fg(look.token("foreground"));
            let app = &mut self.app;
            off.draw(|g| {
                g.buffer_mut().set_style(tall, ground);
                app.page(g, tall, &mut cx);
            })
            .expect("the page");
            let src = off.backend().buffer();
            for y in 0..page.height {
                for x in page.x..page.right() {
                    f.buffer_mut()[(x, page.y + y)] = src[(x, page.y + y + self.scroll)].clone();
                }
            }
            // The scroller, in the column after the page.
            let x = page.right().min(window.right().saturating_sub(1));
            let glyphs = look.glyphs();
            let len = ((page.height as u32 * page.height as u32) / rows as u32).max(1) as u16;
            let end = rows - page.height;
            let at = (self.scroll as u32 * (page.height - len) as u32 / end.max(1) as u32) as u16;
            for k in 0..page.height {
                let (g, c) = if (at..at + len).contains(&k) {
                    (glyphs.scroll_thumb, look.color("muted-foreground"))
                } else {
                    (glyphs.scroll_track, look.color("border"))
                };
                f.buffer_mut()[(x, page.y + k)].set_symbol(g).set_fg(look.terminal(c));
            }
        } else {
            self.scroll = 0;
            self.app.page(f, page, &mut cx);
        }
        // The side panel, after the page: its stops follow the page's.
        if let Some(r) = self.shell.panel_area() {
            self.app.panel(f, r, &mut cx);
        }
        if let Some(r) = self.shell.account() {
            self.focus.stop(account_stop(), r);
            if self.focus.ring(account_stop()) {
                let ring = look.color("ring");
                f.buffer_mut()[(r.x, r.y)]
                    .set_symbol(look.glyphs().focus_mark)
                    .set_fg(look.terminal(ring));
            }
        }
        if self.shell.drawer {
            look.scrim(f.buffer_mut(), window);
        }
        {
            let chrome = self.app.chrome(look);
            Shell::new(look, chrome.rail, chrome.top, chrome.foot).render_drawer(f.buffer_mut(), &mut self.shell);
        }
        if self.shell.drawer {
            for &((g, e), r) in self.shell.entries() {
                self.focus.stop(entry_stop(g, e), r);
            }
        }
        self.focus.end();
        // Keyboard focus on a page stop out of view: the page scrolls to it.
        if self.rows > self.view.height
            && self.focus.input_kind() == crate::focus::InputKind::Keyboard
            && let Some(r) = self.focus.current().and_then(|id| self.focus.area_of(id))
            && r.y >= self.view.y
            && r.x >= self.view.x
            && r.x < self.view.right()
        {
            let top = self.view.y + self.scroll;
            if r.y < top {
                self.scroll -= top - r.y;
            } else if r.bottom() > top + self.view.height {
                self.scroll += r.bottom() - (top + self.view.height);
            }
        }
        if self.to_page {
            self.to_page = false;
            if let Some(id) = self.focus.first_after(self.page_mark) {
                self.focus.set(id);
            }
        }
        if self.app.layer_open() {
            if self.app.layer_scrim() {
                look.scrim(f.buffer_mut(), window);
            }
            let mut cx = Cx {
                look,
                window,
                focus: &mut self.focus,
                hover: self.hover,
                active: self.layer == Layer::None,
                now: self.now,
            };
            self.app.layer(f, &mut cx);
        }
    }

    /// A key on one of the frame's own stops: Enter or Space acts, the arrows move along the
    /// rail. True when the frame took it.
    fn frame_key(&mut self, key: KeyPress) -> bool {
        let entries: Vec<(usize, usize)> = self.shell.entries().iter().map(|(e, _)| *e).collect();
        let on = entries.iter().position(|&(g, e)| self.focus.is(entry_stop(g, e)));
        if let Some(i) = on {
            let to = match key.key {
                Key::Down => Some((i + 1).min(entries.len() - 1)),
                Key::Up => Some(i.saturating_sub(1)),
                Key::Home => Some(0),
                Key::End => Some(entries.len() - 1),
                _ => None,
            };
            if let Some(to) = to {
                self.focus.saw_key();
                let (g, e) = entries[to];
                self.focus.set(entry_stop(g, e));
                return true;
            }
            if key.is_press() {
                let (g, e) = entries[i];
                self.entry(g, e);
                return true;
            }
        }
        if self.focus.is(edge_stop()) && self.shell.edge_key(key) {
            self.focus.saw_key();
            return true;
        }
        if key.is_press() && self.focus.is(palette_stop()) {
            self.open(Layer::Palette);
            return true;
        }
        if key.is_press() && self.focus.is(account_stop()) {
            self.open(Layer::Account);
            return true;
        }
        false
    }

    /// The runtime's shortcuts, then the app's.
    fn registry(&self) -> Shortcuts<Act> {
        let mut keys = Shortcuts::new()
            .bind(Binding::new(Keys::Mod('k'), Act::Palette, WORDS_PALETTE).under_layer())
            .bind(Binding::new(Keys::Char('?'), Act::Shortcuts, WORDS_SHORTCUTS))
            .bind(Binding::new(Keys::Char('/'), Act::Search, WORDS_SEARCH))
            .bind(Binding::new(Keys::Char(','), Act::Settings, WORDS_SETTINGS));
        for b in self.app.shortcuts() {
            let mut bound = Binding::new(b.keys, Act::App(b.action), b.does);
            bound.in_field = b.in_field;
            bound.under_layer = b.under_layer;
            keys = keys.bind(bound);
        }
        keys
    }

    fn act(&mut self, act: Act) {
        match act {
            Act::Palette => self.open(Layer::Palette),
            Act::Shortcuts => self.open(Layer::Shortcuts),
            Act::Settings => self.open(Layer::Settings),
            Act::Search => {
                if self.shell.focus_search() {
                    self.focus.saw_key();
                    self.focus.set(search_stop());
                    self.app.search(&self.shell.query.clone());
                } else {
                    // Below md the search gives way to the palette's button.
                    self.open(Layer::Palette);
                }
            }
            Act::App(id) => {
                self.app.command(&id);
            }
        }
    }

    /// The shortcuts dialog's lines: the keys as written, and what they do.
    fn shortcut_lines(&self) -> Vec<(String, String)> {
        let words = ShortcutWords::default();
        self.keys
            .bindings()
            .iter()
            .map(|b| (b.keys.label(&words), b.does.clone()))
            .chain([
                ("Tab".to_string(), WORDS_TAB.to_string()),
                ("Esc".to_string(), WORDS_BACK.to_string()),
                (Keys::Mod('q').label(&words), WORDS_QUIT.to_string()),
            ])
            .collect()
    }

    fn shortcuts_dialog(&self) -> Dialog<'_> {
        Dialog::new(&self.look, WORDS_SHORTCUTS)
            .width(DialogWidth::Sm)
            .focus(DialogFocus::Close)
            .window(self.width)
    }

    /// The palette as the runtime shows it: the app's commands and its own, its foot saying the
    /// app's name, Enter's verb and the keys that open it.
    fn palette_part(&self, query: &str) -> CommandPalette<'_> {
        let name = self.app.chrome(&self.look).rail.name().to_string();
        let (results, waiting, failed) = self.search_results(query);
        let mut groups = self.groups();
        groups.extend(results);
        CommandPalette::new(&self.look, groups)
            .window(self.width)
            .foot(name, Some(Keys::Mod('k')))
            .search_status(waiting, failed)
    }

    /// The app's search under the commands: its answer's groups (held when they answer an earlier
    /// query), how long the query has waited for its own answer, and whether that answer failed.
    fn search_results(&self, query: &str) -> (Vec<CommandGroup>, Option<u64>, bool) {
        if query.is_empty() || self.layer != Layer::Palette {
            return (Vec::new(), None, false);
        }
        let answer = self.app.palette_answer();
        let fresh = answer.as_ref().is_some_and(|a| a.query == query);
        // The wait runs from the change of the query, through the pause, to its answer.
        let waiting = (!fresh).then(|| self.now.saturating_sub(self.typed.1));
        let Some(answer) = answer else {
            return (Vec::new(), waiting, false);
        };
        if fresh && answer.failed {
            let retry = Command::new(SEARCH_RETRY, PaletteWords::default().retry).icon("↻");
            return (vec![CommandGroup::new("", [retry]).results()], None, true);
        }
        if answer.failed {
            return (Vec::new(), waiting, false);
        }
        let groups = answer.groups.into_iter().map(|g| g.results().held(!fresh)).collect();
        (groups, waiting, false)
    }

    fn groups(&self) -> Vec<CommandGroup> {
        let look = &self.look;
        let mut groups = self.app.commands();
        let themes = tokens::themes().into_iter().map(|t| {
            let c = Command::new(format!("{THEME}{t}"), format!("Theme: {t}")).icon("◐");
            if t == look.theme() { c.meta("now", None) } else { c }
        });
        let mut general = vec![
            Command::new(SETTINGS, "Settings").icon("⚙").hint(","),
            Command::new(SCHEME, "Switch light and dark").icon("◐"),
            Command::new(
                DENSITY,
                if look.density() == Density::Regular {
                    "Compact rows"
                } else {
                    "Regular rows"
                },
            )
            .icon("≡"),
            Command::new(
                RAIL,
                if self.shell.collapsed {
                    "Expand the rail"
                } else {
                    "Collapse the rail"
                },
            )
            .icon("▣"),
            Command::new(
                KIOSK,
                if self.shell.kiosk {
                    "Leave full screen"
                } else {
                    "Full screen"
                },
            )
            .icon("□")
            .hint("the page alone, Escape leaves"),
            Command::new(
                MOUSE,
                if self.mouse {
                    "Turn the mouse off"
                } else {
                    "Turn the mouse on"
                },
            )
            .icon("▲")
            .hint(if self.mouse {
                "the terminal selects text again"
            } else {
                "presses and the wheel act again"
            }),
        ];
        general.extend(themes);
        groups.push(CommandGroup::new("General", general));
        groups
    }

    fn frame_section(&self) -> SettingsSection {
        SettingsSection::new(
            "frame",
            "Frame",
            [
                Setting::new(RAIL, "Rail", 0)
                    .description("Expanded, or collapsed to its icons.")
                    .choice(["Expanded", "Collapsed"], usize::from(self.shell.collapsed)),
                Setting::new(KIOSK, "Full screen", 0)
                    .description("The page alone, for a wall display: no rail, no top bar.")
                    .choice(["Off", "On"], usize::from(self.shell.kiosk)),
                Setting::new(MOUSE, "Mouse", 0)
                    .description("Off, the terminal selects text as it does without an app.")
                    .choice(["On", "Off"], usize::from(!self.mouse)),
            ],
        )
        .icon("▣")
        .no_reset()
    }

    fn dialog(&self) -> SettingsDialog<'_> {
        let mut sections = vec![look_section(&self.look), self.frame_section()];
        sections.extend(self.app.settings(&self.look));
        SettingsDialog::new(&self.look, [SettingsGroup::new(None, sections)]).window(self.width)
    }

    /// The account menu, as sokkel-ui's: the name, Settings, Appearance, the app's rows, the
    /// scheme, then Sign out or Sign in.
    fn account_menu(&self) -> (Vec<PopupRow>, Vec<AccountAct>) {
        let words = AccountWords::default();
        let chrome = self.app.chrome(&self.look);
        let name = chrome.rail.account_name().unwrap_or_default();
        let app = self.app.account();
        let mut rows = vec![PopupRow::Label(name), PopupRow::Separator];
        let mut acts = vec![AccountAct::None, AccountAct::None];
        let mut push = |rows: &mut Vec<PopupRow>, row: PopupRow, act: AccountAct| {
            rows.push(row);
            acts.push(act);
        };
        push(
            &mut rows,
            Item::new(words.settings).icon("⚙").shortcut(",").into(),
            AccountAct::Settings,
        );
        push(
            &mut rows,
            Item::new(words.appearance).icon("◐").into(),
            AccountAct::Appearance,
        );
        // Not below md, where there is no keyboard to speak of (sokkel-ui's account menu).
        if self.width != Width::Narrow {
            push(
                &mut rows,
                Item::new(words.shortcuts).icon("⌨").shortcut("?").into(),
                AccountAct::Shortcuts,
            );
        }
        for (id, item) in app.rows {
            push(&mut rows, item.into(), AccountAct::App(id));
        }
        push(&mut rows, PopupRow::Separator, AccountAct::None);
        push(&mut rows, PopupRow::Label(words.scheme), AccountAct::None);
        let dark = self.look.scheme() == Scheme::Dark;
        push(
            &mut rows,
            Item::new(words.light).radio(!dark).into(),
            AccountAct::Scheme(Scheme::Light),
        );
        push(
            &mut rows,
            Item::new(words.dark).radio(dark).into(),
            AccountAct::Scheme(Scheme::Dark),
        );
        if let Some((id, item)) = app.leave {
            push(&mut rows, PopupRow::Separator, AccountAct::None);
            push(&mut rows, item.into(), AccountAct::App(id));
        }
        (rows, acts)
    }

    fn account_event(&mut self, event: MenuEvent, acts: &[AccountAct]) {
        match event {
            MenuEvent::Closed => self.layer = Layer::None,
            MenuEvent::Action(i, _) => {
                let (theme, density) = (self.look.theme().to_string(), self.look.density());
                self.layer = Layer::None;
                match acts.get(i).cloned().unwrap_or(AccountAct::None) {
                    AccountAct::Settings => {
                        self.open(Layer::Settings);
                        // The app's first section, past the look and the frame, where it has one.
                        if !self.app.settings(&self.look).is_empty() {
                            self.settings.shown = (0, 2);
                        }
                    }
                    AccountAct::Appearance => self.open(Layer::Settings),
                    AccountAct::Shortcuts => self.open(Layer::Shortcuts),
                    AccountAct::Scheme(s) => {
                        self.relook(s, density, &theme);
                        self.layer = Layer::Account;
                    }
                    AccountAct::App(id) => {
                        self.app.command(&id);
                    }
                    AccountAct::None => {}
                }
            }
            _ => {}
        }
    }

    fn open(&mut self, layer: Layer) {
        self.layer = layer;
        self.palette = PaletteState::default();
        self.settings = SettingsState::default();
        if layer == Layer::Account {
            let (rows, _) = self.account_menu();
            self.account = MenuState {
                open: true,
                highlight: crate::popup::stops(&rows).first().copied().unwrap_or(0),
                sub: None,
            };
        } else {
            self.shell.drawer = false;
        }
    }

    fn relook(&mut self, scheme: Scheme, density: Density, theme: &str) {
        if let Some(look) = Look::new(theme, scheme) {
            self.look = look
                .with_density(density)
                .with_depth(self.look.depth())
                .with_charset(self.look.charset());
        }
    }

    /// Run a command from the palette: the runtime's own, else the app's.
    fn command(&mut self, id: &str) {
        let (theme, scheme, density) = (self.look.theme().to_string(), self.look.scheme(), self.look.density());
        self.layer = Layer::None;
        match id {
            SETTINGS => self.open(Layer::Settings),
            SCHEME => self.relook(
                if scheme == Scheme::Dark {
                    Scheme::Light
                } else {
                    Scheme::Dark
                },
                density,
                &theme,
            ),
            DENSITY => self.relook(
                scheme,
                if density == Density::Regular {
                    Density::Compact
                } else {
                    Density::Regular
                },
                &theme,
            ),
            RAIL => self.shell.collapsed = !self.shell.collapsed,
            KIOSK => self.shell.kiosk = !self.shell.kiosk,
            MOUSE => self.mouse = !self.mouse,
            t if t.starts_with(THEME) => self.relook(scheme, density, &t[THEME.len()..]),
            _ => match self.app.command(id) {
                Ran::Done => self.palette.done(),
                Ran::Refused(why) => {
                    self.layer = Layer::Palette;
                    self.palette.refused(why, &crate::palette::PaletteWords::default());
                }
                Ran::Running => {
                    self.layer = Layer::Palette;
                    self.palette.running(id, 0);
                    self.running = Some((id.to_string(), self.now));
                }
            },
        }
    }

    /// What a settings event does: a choice turns the look, the frame, or the app's setting.
    fn settings_event(&mut self, event: SettingsEvent) {
        match event {
            SettingsEvent::Close => self.layer = Layer::None,
            SettingsEvent::Chose(k, option) => {
                let Some(id) = self.dialog().setting(&self.settings, k).map(|s| s.id.clone()) else {
                    return;
                };
                if let Some(look) = look_chose(&self.look, &id, option) {
                    self.look = look;
                } else {
                    match id.as_str() {
                        RAIL => self.shell.collapsed = option == 1,
                        KIOSK => self.shell.kiosk = option == 1,
                        MOUSE => self.mouse = option == 0,
                        _ => self.app.chose(&id, option),
                    }
                }
            }
            _ => {}
        }
    }

    fn palette_event(&mut self, event: PaletteEvent) {
        match event {
            // The app's search: Retry asks again; a result opens and the palette closes.
            PaletteEvent::Run(id) if id == SEARCH_RETRY => {
                let query = self.palette.query.clone();
                self.app.palette_search(&query);
                self.asked = Some((query, self.now));
                self.typed.1 = self.now;
            }
            PaletteEvent::Run(id)
                if self
                    .app
                    .palette_answer()
                    .is_some_and(|a| a.groups.iter().any(|g| g.commands.iter().any(|c| c.id == id))) =>
            {
                self.layer = Layer::None;
                self.app.palette_open(&id);
            }
            PaletteEvent::Run(id) => self.command(&id),
            PaletteEvent::StopWaiting => {
                if let Some((id, _)) = self.running.take() {
                    self.app.stop_waiting(&id);
                }
                self.palette.done();
                self.layer = Layer::None;
            }
            PaletteEvent::Close if self.running.is_none() => self.layer = Layer::None,
            _ => {}
        }
    }
}

impl<A: ShellApp> Program for Host<A> {
    fn look(&self) -> &Look {
        &self.look
    }

    /// A paste goes where the keys go: the palette's query, the settings' search, the top bar's
    /// search, else the app (its open layer, or the page).
    fn paste(&mut self, text: &str) {
        let typed = || {
            text.chars()
                .filter(|c| !c.is_control())
                .map(|c| KeyPress::new(Key::Char(c)))
        };
        match self.layer {
            Layer::Settings if self.settings.focus != crate::settings::SettingsFocus::Search => {}
            Layer::Palette | Layer::Settings => {
                for k in typed().collect::<Vec<_>>() {
                    self.key(k);
                }
            }
            Layer::Account | Layer::Shortcuts | Layer::Flyout(..) => {}
            Layer::None if self.shell.searching => {
                for k in typed().collect::<Vec<_>>() {
                    self.shell.key(k);
                }
                let query = self.shell.query.clone();
                self.app.search(&query);
            }
            Layer::None => {
                let mut cx = Cx {
                    look: &self.look,
                    window: self.window,
                    focus: &mut self.focus,
                    hover: self.hover,
                    active: true,
                    now: self.now,
                };
                self.app.paste(text, &mut cx);
            }
        }
    }

    fn mouse(&self) -> bool {
        self.mouse
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        self.app.tick(now);
        // The palette's query, to the app's search once typing has paused.
        if self.layer == Layer::Palette {
            let query = self.palette.query.clone();
            if query != self.typed.0 {
                self.typed = (query.clone(), now);
            }
            let asked = self.asked.as_ref().is_some_and(|(q, _)| *q == query);
            if !query.is_empty() && !asked && now.saturating_sub(self.typed.1) >= DEBOUNCE_MS {
                self.app.palette_search(&query);
                self.asked = Some((query, now));
            }
        } else if self.asked.is_some() {
            self.asked = None;
            self.typed = (String::new(), now);
        }
        if let Some((id, since)) = self.running.clone() {
            if self.app.finished(&id) {
                self.running = None;
                self.palette.done();
                self.layer = Layer::None;
            } else {
                self.palette.running(id, now.saturating_sub(since));
            }
        }
    }

    fn key(&mut self, key: KeyPress) -> bool {
        match self.layer {
            Layer::Palette if key.ctrl && key.key == Key::Char('k') => {
                self.layer = Layer::None;
                return false;
            }
            Layer::Shortcuts => {
                if key.is(Key::Escape) || key.is_press() || key.is(Key::Char('?')) {
                    self.layer = Layer::None;
                }
                return false;
            }
            Layer::Palette => {
                let mut st = std::mem::take(&mut self.palette);
                let event = self.palette_part(&st.query).handle(&mut st, key);
                self.palette = st;
                self.palette_event(event);
                return false;
            }
            Layer::Settings => {
                let mut st = std::mem::take(&mut self.settings);
                let event = self.dialog().handle(&mut st, key);
                self.settings = st;
                self.settings_event(event);
                return false;
            }
            Layer::Flyout(g, e) => {
                let (menu, places, _) = self.flyout_menu(g, e);
                let mut st = self.flyout;
                let event = menu.handle(&mut st, key);
                drop(menu);
                self.flyout = st;
                self.flyout_event(g, event, &places);
                return false;
            }
            Layer::Account => {
                let (rows, acts) = self.account_menu();
                let mut st = self.account;
                let event = DropdownMenu::new(&self.look, rows).handle(&mut st, key);
                self.account = st;
                self.account_event(event, &acts);
                return false;
            }
            Layer::None => {}
        }
        if self.app.layer_open() {
            let mut cx = Cx {
                look: &self.look,
                window: self.window,
                focus: &mut self.focus,
                hover: self.hover,
                active: true,
                now: self.now,
            };
            return self.app.key(key, &mut cx) == Flow::Quit;
        }
        match self.shell.key(key) {
            ShellEvent::Search => {
                let query = self.shell.query.clone();
                self.app.search(&query);
                return false;
            }
            ShellEvent::Ignored if !self.shell.searching => {}
            _ => {
                // Left the search: Tab goes on from it, Enter and Escape stay on it.
                if matches!(key.key, Key::Tab | Key::BackTab) {
                    self.focus.handle(key);
                }
                return false;
            }
        }
        let cx = KeyContext {
            typing: self.focus.typing() || self.app.typing(),
            under_layer: false,
            now_ms: self.now,
        };
        if let Some(act) = self.keys.handle(key, cx) {
            self.act(act);
            return false;
        }
        if self.keys.pending().is_some() {
            return false;
        }
        if self.focus.handle(key) {
            // Tab onto the search starts typing in it.
            if self.focus.is(search_stop()) {
                self.shell.focus_search();
            }
            return false;
        }
        if self.frame_key(key) {
            return false;
        }
        let mut cx = Cx {
            look: &self.look,
            window: self.window,
            focus: &mut self.focus,
            hover: self.hover,
            active: true,
            now: self.now,
        };
        // Escape closes the layer on top and never quits (patterns/shell): Ctrl+Q quits. With
        // nothing open and the page not taking it, Escape goes back.
        let flow = self.app.key(key, &mut cx);
        // Keys the page leaves scroll a page that scrolls.
        if flow == Flow::Ignored && self.rows > self.view.height {
            let end = self.rows - self.view.height;
            let page = self.view.height.saturating_sub(1).max(1);
            let to = match key.key {
                Key::PageDown => Some((self.scroll + page).min(end)),
                Key::PageUp => Some(self.scroll.saturating_sub(page)),
                Key::Down if !self.focus.typing() => Some((self.scroll + 1).min(end)),
                Key::Up if !self.focus.typing() => Some(self.scroll.saturating_sub(1)),
                Key::Home => Some(0),
                Key::End => Some(end),
                _ => None,
            };
            if let Some(to) = to {
                self.scroll = to;
                return false;
            }
        }
        match flow {
            Flow::Quit => true,
            Flow::Ignored if key.is(Key::Escape) => {
                self.back();
                false
            }
            _ => false,
        }
    }

    fn pointer(&mut self, p: Pointer) {
        match self.layer {
            Layer::Palette => {
                let Some(area) = self.palette_at else { return };
                let mut st = std::mem::take(&mut self.palette);
                let pal = self.palette_part(&st.query);
                let hit = pal.hit(area, p.at, &st);
                let event = match (p.kind, hit) {
                    (PointerKind::Move, Some(h)) => {
                        pal.hover(&mut st, h);
                        PaletteEvent::Ignored
                    }
                    (PointerKind::Press, Some(h)) => pal.press(&mut st, h),
                    (PointerKind::Press, None) if !area.contains(p.at) => PaletteEvent::Close,
                    (PointerKind::ScrollUp, _) => pal.handle(&mut st, KeyPress::new(Key::Up)),
                    (PointerKind::ScrollDown, _) => pal.handle(&mut st, KeyPress::new(Key::Down)),
                    _ => PaletteEvent::Ignored,
                };
                drop(pal);
                self.palette = st;
                self.palette_event(event);
            }
            Layer::Settings => {
                let (Some(area), PointerKind::Press) = (self.settings_at, p.kind) else {
                    return;
                };
                let mut st = std::mem::take(&mut self.settings);
                let event = self.dialog().press(&mut st, area, p.at);
                self.settings = st;
                self.settings_event(event);
            }
            Layer::Shortcuts => {
                if p.kind == PointerKind::Press {
                    self.layer = Layer::None;
                }
            }
            Layer::Flyout(g, e) => {
                let (menu, places, anchor) = self.flyout_menu(g, e);
                let mut st = self.flyout;
                let event = menu.pointer(&mut st, self.window, anchor, p);
                drop(menu);
                self.flyout = st;
                self.flyout_event(g, event, &places);
            }
            Layer::Account => {
                let Some(anchor) = self.shell.account() else { return };
                let (rows, acts) = self.account_menu();
                let mut st = self.account;
                let event = DropdownMenu::new(&self.look, rows).pointer(&mut st, self.window, anchor, p);
                self.account = st;
                self.account_event(event, &acts);
            }
            Layer::None => {
                if matches!(p.kind, PointerKind::Move) {
                    self.hover = Some(p.at);
                }
                let mut cx = Cx {
                    look: &self.look,
                    window: self.window,
                    focus: &mut self.focus,
                    hover: self.hover,
                    active: true,
                    now: self.now,
                };
                if self.app.layer_open() {
                    self.app.pointer(p, &mut cx);
                    return;
                }
                match self.shell.pointer(p) {
                    ShellEvent::Account => self.open(Layer::Account),
                    ShellEvent::Entry(g, e) => self.entry(g, e),
                    ShellEvent::Palette => self.open(Layer::Palette),
                    ShellEvent::Search => {
                        self.focus.press(p.at);
                        let query = self.shell.query.clone();
                        self.app.search(&query);
                    }
                    ShellEvent::Changed => {}
                    ShellEvent::Ignored if self.shell.covers(p.at) => {}
                    ShellEvent::Ignored => {
                        // A page that scrolls: the wheel scrolls it; a press lands where the page
                        // drew, moved by the scroll.
                        let tall = self.rows > self.view.height;
                        if tall && matches!(p.kind, PointerKind::ScrollUp | PointerKind::ScrollDown) {
                            let end = self.rows - self.view.height;
                            self.scroll = if p.kind == PointerKind::ScrollUp {
                                self.scroll.saturating_sub(3)
                            } else {
                                (self.scroll + 3).min(end)
                            };
                            return;
                        }
                        let p = if tall && self.view.contains(p.at) {
                            Pointer {
                                at: Position::new(p.at.x, p.at.y + self.scroll),
                                ..p
                            }
                        } else {
                            p
                        };
                        if p.kind == PointerKind::Press {
                            cx.focus.press(p.at);
                        }
                        self.app.pointer(p, &mut cx);
                    }
                }
            }
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let window = f.area();
        self.width = Width::of(window);
        self.window = window;
        // A look changed by any way (the palette, the settings, the account menu) is kept.
        let worn = (self.look.theme().to_string(), self.look.scheme(), self.look.density());
        if worn != self.worn {
            if let Some(path) = &self.kept {
                let _ = write_look(path, &self.look);
            }
            self.worn = worn;
        }
        // A width the rail's edge left is kept too, once the drag lets go.
        if self.shell.rail_size != self.rail_kept && !self.shell.dragging() {
            if let Some(path) = &self.kept {
                let _ = write_rail_size(path, self.shell.rail_size);
            }
            self.rail_kept = self.shell.rail_size;
        }
        self.draw_frame(f);
        if let Some(pos) = self.shell.cursor_position() {
            f.set_cursor_position(pos);
        }
        self.palette_at = None;
        self.settings_at = None;
        // The palette, the settings and the shortcuts stand over a scrim; the account menu does not.
        if matches!(self.layer, Layer::Palette | Layer::Settings | Layer::Shortcuts) {
            self.look.scrim(f.buffer_mut(), window);
        }
        match self.layer {
            Layer::Palette => {
                let mut st = std::mem::take(&mut self.palette);
                let pal = self.palette_part(&st.query);
                let r = pal.area(window, &st);
                pal.render(r, f.buffer_mut(), &mut st);
                if let Some(pos) = st.cursor_position() {
                    f.set_cursor_position(pos);
                }
                drop(pal);
                self.palette = st;
                self.palette_at = Some(r);
            }
            Layer::Settings => {
                let mut st = std::mem::take(&mut self.settings);
                let d = self.dialog();
                let r = d.area(window);
                d.render(r, f.buffer_mut(), &mut st);
                if let Some(pos) = st.cursor_position() {
                    f.set_cursor_position(pos);
                }
                drop(d);
                self.settings = st;
                self.settings_at = Some(r);
            }
            Layer::Shortcuts => {
                let lines = self.shortcut_lines();
                let d = self.shortcuts_dialog();
                let area = d.area(window, lines.len() as u16);
                (&d).render(area, f.buffer_mut());
                let body = d.layout(area).body;
                let look = &self.look;
                let key_style = TextRole::Badge
                    .style(look)
                    .fg(look.token("muted-foreground"))
                    .bg(look.token("muted"));
                let does_style = TextRole::Body.style(look).fg(look.token("popover-foreground"));
                let key_w = lines
                    .iter()
                    .map(|(k, _)| crate::text::width(k) as u16)
                    .max()
                    .unwrap_or(0)
                    + 2;
                for (k, (keys, does)) in lines.iter().enumerate() {
                    let y = body.y + k as u16;
                    if y >= body.bottom() {
                        break;
                    }
                    f.buffer_mut().set_string(body.x, y, format!(" {keys} "), key_style);
                    f.buffer_mut().set_stringn(
                        body.x + key_w + 2,
                        y,
                        does,
                        body.width.saturating_sub(key_w + 2) as usize,
                        does_style,
                    );
                }
            }
            Layer::Flyout(g, e) => {
                let (menu, _, anchor) = self.flyout_menu(g, e);
                menu.render(window, anchor, f.buffer_mut(), &self.flyout);
            }
            Layer::Account => {
                if let Some(anchor) = self.shell.account() {
                    let (rows, _) = self.account_menu();
                    DropdownMenu::new(&self.look, rows).render(window, anchor, f.buffer_mut(), &self.account);
                }
            }
            Layer::None => {}
        }
    }
}

/// The keys the look is kept under, the web's own (sokkel-ui's `SCHEME_KEY` and `APPEARANCE_KEY`),
/// so an app's look reads the same wherever it is kept.
pub const SCHEME_KEY: &str = "sokkel-ui-scheme";
pub const APPEARANCE_KEY: &str = "sokkel-ui-appearance";

/// Where an app named `id` keeps its look: `appearance.json` in its folder under the user's config
/// directory (`$XDG_CONFIG_HOME`, else `~/.config`, else `%APPDATA%`).
pub fn kept_path(id: &str) -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(std::path::PathBuf::from))?;
    Some(base.join(id).join("appearance.json"))
}

/// The look kept at `path`, worn over `base` (its depth and character set kept); None where
/// nothing is kept or it cannot be read.
pub fn read_look(path: &std::path::Path, base: &Look) -> Option<Look> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
    let scheme = match v.get(SCHEME_KEY).and_then(|s| s.as_str()) {
        Some("light") => Scheme::Light,
        Some("dark") => Scheme::Dark,
        _ => base.scheme(),
    };
    let look = v.get(APPEARANCE_KEY);
    let theme = look
        .and_then(|a| a.get("theme"))
        .and_then(|t| t.as_str())
        .unwrap_or(base.theme());
    let density = match look.and_then(|a| a.get("density")).and_then(|d| d.as_str()) {
        Some("compact") => Density::Compact,
        Some("regular") => Density::Regular,
        _ => base.density(),
    };
    Some(
        Look::new(theme, scheme)?
            .with_density(density)
            .with_depth(base.depth())
            .with_charset(base.charset()),
    )
}

/// Where the rail's kept width lives: `layout.json` beside the kept look.
fn layout_path(look_path: &std::path::Path) -> std::path::PathBuf {
    look_path.with_file_name("layout.json")
}

/// The rail's width its edge left in an earlier run, kept beside the look at `look_path`.
pub fn read_rail_size(look_path: &std::path::Path) -> Option<u16> {
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(layout_path(look_path)).ok()?).ok()?;
    v.get("railSize").and_then(|n| n.as_u64()).map(|n| n as u16)
}

/// Keep the rail's width (`None`: its own) beside the look at `look_path`.
pub fn write_rail_size(look_path: &std::path::Path, size: Option<u16>) -> std::io::Result<()> {
    let path = layout_path(look_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let v = serde_json::json!({ "railSize": size });
    std::fs::write(path, serde_json::to_string_pretty(&v)? + "\n")
}

/// Keep `look` at `path`, under the web's keys.
pub fn write_look(path: &std::path::Path, look: &Look) -> std::io::Result<()> {
    let v = serde_json::json!({
        SCHEME_KEY: if look.scheme() == Scheme::Dark { "dark" } else { "light" },
        APPEARANCE_KEY: {
            "theme": look.theme(),
            "density": if look.density() == Density::Compact { "compact" } else { "regular" },
        },
    });
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&v)? + "\n")
}
