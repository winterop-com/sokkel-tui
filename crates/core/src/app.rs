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
use crate::palette::{Command, CommandGroup, CommandPalette, PaletteEvent, PaletteState};
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
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::Widget;
use std::time::{Duration, Instant};

/// One screen the runtime drives.
pub trait Program {
    fn look(&self) -> &Look;
    fn draw(&mut self, f: &mut Frame);
    /// A key; true quits.
    fn key(&mut self, key: KeyPress) -> bool;
    fn pointer(&mut self, _p: Pointer) {}
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
            terminal.draw(|f| program.draw(f))?;
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
    ratatui::restore();
    result
}

/// The frame's parts an app gives each frame.
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
    /// The app's name for what it keeps between runs (its look, under [`kept_path`]); None keeps
    /// nothing.
    fn id(&self) -> Option<&str> {
        None
    }
    /// The rail, the top bar and the status line, this frame.
    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a>;
    /// Draw the page in `area`, naming its stops in `cx.focus` as it draws them (after the
    /// frame's: the rail's entries, the search, the palette button).
    fn page(&mut self, f: &mut Frame, area: Rect, cx: &mut Cx);
    /// Whether the app has a layer of its own open (a dialog, a menu): keys and presses go to
    /// it first, and the frame's shortcuts keep quiet.
    fn layer_open(&self) -> bool {
        false
    }
    /// Draw the app's open layer, over the frame and the drawer.
    fn layer(&mut self, _f: &mut Frame, _cx: &mut Cx) {}
    /// The app's commands, shown in the palette before the runtime's own.
    fn commands(&self) -> Vec<CommandGroup> {
        Vec::new()
    }
    /// A command of the app's was run.
    fn command(&mut self, _id: &str) {}
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
    fn tick(&mut self, _now_ms: u64) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    None,
    Palette,
    Settings,
    Account,
    Shortcuts,
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
const THEME: &str = "sokkel.theme:";

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
    window: Rect,
    keys: Shortcuts<Act>,
    now: u64,
    /// Where the look is kept, and the look last kept there.
    kept: Option<std::path::PathBuf>,
    worn: (String, Scheme, Density),
    focus: Focus,
    hover: Option<Position>,
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
    let mut host = Host::new(app, look, kept);
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
    term.draw(|f| host.draw(f)).expect("a frame");
    for (n, key) in keys.iter().enumerate() {
        host.tick(n as u64 * 100);
        host.key(*key);
        term.draw(|f| host.draw(f)).expect("a frame");
    }
    term.backend().buffer().clone()
}

impl<A: ShellApp> Host<A> {
    fn new(app: A, look: Look, kept: Option<std::path::PathBuf>) -> Host<A> {
        let mut host = Host {
            kept,
            worn: (look.theme().to_string(), look.scheme(), look.density()),
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
            window: Rect::default(),
            keys: Shortcuts::new(),
            now: 0,
            focus: Focus::new(),
            hover: None,
        };
        host.keys = host.registry();
        host
    }

    /// The frame and the page, once: the shell, the frame's stops, the page with its own, the
    /// account last, the drawer, the app's layer.
    fn draw_frame(&mut self, f: &mut Frame) {
        let look = &self.look;
        let window = f.area();
        // The ring goes where the keyboard left focus, from the last frame's stops.
        self.shell.rail_focus = self
            .shell
            .entries()
            .iter()
            .map(|(e, _)| *e)
            .find(|&(g, e)| self.focus.ring(entry_stop(g, e)));
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
            Shell::new(look, chrome.rail, chrome.top, chrome.foot).render(window, f.buffer_mut(), &mut self.shell)
        };
        self.focus.begin();
        if !self.shell.drawer {
            for &((g, e), r) in self.shell.entries() {
                self.focus.stop(entry_stop(g, e), r);
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
        self.app.page(f, page, &mut cx);
        if let Some(r) = self.shell.account() {
            self.focus.stop(account_stop(), r);
            if self.focus.ring(account_stop()) {
                let ring = look.color("ring");
                f.buffer_mut()[(r.x, r.y)]
                    .set_symbol(look.glyphs().focus_mark)
                    .set_fg(look.terminal(ring));
            }
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
        if self.app.layer_open() {
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
                self.shell.drawer = false;
                self.app.entry(g, e);
                return true;
            }
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
            Act::App(id) => self.app.command(&id),
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
            Item::new(words.light).check(!dark).into(),
            AccountAct::Scheme(Scheme::Light),
        );
        push(
            &mut rows,
            Item::new(words.dark).check(dark).into(),
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
                    AccountAct::App(id) => self.app.command(&id),
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
            MOUSE => self.mouse = !self.mouse,
            t if t.starts_with(THEME) => self.relook(scheme, density, &t[THEME.len()..]),
            _ => self.app.command(id),
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
            PaletteEvent::Run(id) => {
                self.palette.done();
                self.command(&id);
            }
            PaletteEvent::Close | PaletteEvent::StopWaiting => self.layer = Layer::None,
            _ => {}
        }
    }
}

impl<A: ShellApp> Program for Host<A> {
    fn look(&self) -> &Look {
        &self.look
    }

    fn mouse(&self) -> bool {
        self.mouse
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        self.app.tick(now);
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
                let event = CommandPalette::new(&self.look, self.groups())
                    .window(self.width)
                    .handle(&mut st, key);
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
        // Escape closes the layer on top and never quits (patterns/shell): Ctrl+Q quits.
        self.app.key(key, &mut cx) == Flow::Quit
    }

    fn pointer(&mut self, p: Pointer) {
        match self.layer {
            Layer::Palette => {
                let Some(area) = self.palette_at else { return };
                let mut st = std::mem::take(&mut self.palette);
                let pal = CommandPalette::new(&self.look, self.groups()).window(self.width);
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
            Layer::Account => {
                let Some(anchor) = self.shell.account() else { return };
                let (rows, acts) = self.account_menu();
                let menu = DropdownMenu::new(&self.look, rows);
                let mut st = self.account;
                let window = self.window;
                let event = match (p.kind, menu.hit(window, anchor, p.at, &st)) {
                    (PointerKind::Move, Some(h)) => {
                        menu.hover(&mut st, h);
                        MenuEvent::Ignored
                    }
                    (PointerKind::Press, Some(h)) => menu.press(&mut st, h),
                    (PointerKind::Press, None) => {
                        let (area, _) = menu.areas(window, anchor, &st);
                        if area.contains(p.at) {
                            MenuEvent::Ignored
                        } else {
                            MenuEvent::Closed
                        }
                    }
                    _ => MenuEvent::Ignored,
                };
                drop(menu);
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
                    ShellEvent::Entry(g, e) => self.app.entry(g, e),
                    ShellEvent::Palette => self.open(Layer::Palette),
                    ShellEvent::Search => {
                        self.focus.press(p.at);
                        let query = self.shell.query.clone();
                        self.app.search(&query);
                    }
                    ShellEvent::Changed => {}
                    ShellEvent::Ignored if self.shell.covers(p.at) => {}
                    ShellEvent::Ignored => {
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
        self.draw_frame(f);
        if let Some(pos) = self.shell.cursor_position() {
            f.set_cursor_position(pos);
        }
        self.palette_at = None;
        self.settings_at = None;
        match self.layer {
            Layer::Palette => {
                let mut st = std::mem::take(&mut self.palette);
                let pal = CommandPalette::new(&self.look, self.groups()).window(self.width);
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
