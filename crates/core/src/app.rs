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
use crate::glyphs::Charset;
use crate::keys::{Key, KeyPress, Pointer, PointerKind};
use crate::look::{Depth, Look};
use crate::palette::{Command, CommandGroup, CommandPalette, PaletteEvent, PaletteState};
use crate::rail::Rail;
use crate::settings::{
    Setting, SettingsDialog, SettingsEvent, SettingsGroup, SettingsSection, SettingsState, look_chose, look_section,
};
use crate::shell::{Shell, ShellEvent, ShellState};
use crate::status_line::StatusLine;
use crate::tokens::{self, Density, Scheme};
use crate::top_bar::TopBar;
use ratatui::Frame;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use ratatui::layout::Rect;
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
    /// The rail, the top bar and the status line, this frame.
    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a>;
    /// Draw the page in `area`. `active` is false while a layer (the palette, the settings, the
    /// drawer) stands over it: keyboard focus is there.
    fn page(&mut self, look: &Look, f: &mut Frame, area: Rect, active: bool);
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
    /// A key while no layer is open.
    fn key(&mut self, _key: KeyPress) -> Flow {
        Flow::Ignored
    }
    /// A pointer event on the page.
    fn pointer(&mut self, _p: Pointer) {}
    fn tick(&mut self, _now_ms: u64) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    None,
    Palette,
    Settings,
}

/// The runtime's own commands and settings, by id.
const SETTINGS: &str = "sokkel.settings";
const SCHEME: &str = "sokkel.scheme";
const DENSITY: &str = "sokkel.density";
const RAIL: &str = "sokkel.rail";
const MOUSE: &str = "sokkel.mouse";
const THEME: &str = "sokkel.theme:";

/// Draw `app` in its frame once: the shell, the page, the drawer over it. [`run_shell`] does this
/// each frame; a program that writes a frame out (a specimen) calls it on a test backend.
pub fn draw_shell(app: &mut impl ShellApp, look: &Look, f: &mut Frame, shell: &mut ShellState, active: bool) {
    let window = f.area();
    let page = {
        let chrome = app.chrome(look);
        Shell::new(look, chrome.rail, chrome.top, chrome.foot).render(window, f.buffer_mut(), shell)
    };
    let drawer = shell.drawer;
    app.page(look, f, page, active && !drawer);
    let chrome = app.chrome(look);
    Shell::new(look, chrome.rail, chrome.top, chrome.foot).render_drawer(f.buffer_mut(), shell);
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
}

/// Run `app` in sokkel's frame, starting in `look`.
pub fn run_shell(app: impl ShellApp, look: Look) -> std::io::Result<()> {
    let mut host = Host {
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
    };
    run(&mut host)
}

impl<A: ShellApp> Host<A> {
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

    fn open(&mut self, layer: Layer) {
        self.layer = layer;
        self.palette = PaletteState::default();
        self.settings = SettingsState::default();
        self.shell.drawer = false;
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
        self.app.tick(now);
    }

    fn key(&mut self, key: KeyPress) -> bool {
        match self.layer {
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
            Layer::None => {}
        }
        if self.shell.key(key) != ShellEvent::Ignored {
            return false;
        }
        if key.ctrl && key.key == Key::Char('k') {
            self.open(Layer::Palette);
            return false;
        }
        match self.app.key(key) {
            Flow::Quit => true,
            Flow::Handled => false,
            Flow::Ignored => match key.key {
                Key::Char(',') if !key.ctrl => {
                    self.open(Layer::Settings);
                    false
                }
                // Escape closes the layer on top and never quits (patterns/shell): Ctrl+Q quits.
                _ => false,
            },
        }
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
            Layer::None => match self.shell.pointer(p) {
                ShellEvent::Entry(g, e) => self.app.entry(g, e),
                ShellEvent::Palette | ShellEvent::Search => self.open(Layer::Palette),
                ShellEvent::Changed => {}
                ShellEvent::Ignored if self.shell.covers(p.at) => {}
                ShellEvent::Ignored => self.app.pointer(p),
            },
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let window = f.area();
        self.width = Width::of(window);
        let active = self.layer == Layer::None;
        draw_shell(&mut self.app, &self.look, f, &mut self.shell, active);
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
            Layer::None => {}
        }
    }
}
