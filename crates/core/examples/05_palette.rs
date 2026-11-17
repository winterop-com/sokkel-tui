//! 05: the command palette and the settings dialog. Ctrl+K opens the palette: type to narrow it,
//! Enter runs. Its commands switch the theme (all thirteen the contract has), the scheme and the
//! density, and open Settings; one is disabled with its reason, one is refused, and one answers late
//! enough to show Stop waiting.
//!
//! Usage: cargo run --example 05_palette [-- light]
//! Keys: Ctrl+K the palette; in it Up and Down, Enter, Escape. In Settings Tab moves, Enter on an
//! entry or a choice, Escape closes. The mouse works on rows, entries and verbs. Ctrl+Q quits.

#[path = "common/mod.rs"]
mod common;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Button, ButtonVariant, Command, CommandGroup, CommandPalette, CommandTone, Density, Fact, Key, KeyPress, Keys,
    Look, NoteTone, PageHeader, PaletteEvent, PaletteState, PaletteWords, Pointer, PointerKind, Scheme, Setting,
    SettingsDialog, SettingsEvent, SettingsFocus, SettingsGroup, SettingsLayout, SettingsSection, SettingsState,
    StatusLine, TextRole, tokens,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    None,
    Palette,
    Settings,
}

struct App {
    look: Look,
    layer: Layer,
    palette: PaletteState,
    settings: SettingsState,
    said: String,
    now: u64,
    late_since: Option<u64>,
    palette_at: Option<Rect>,
    settings_at: Option<(Rect, SettingsLayout)>,
}

impl App {
    fn relook(&mut self, theme: &str, scheme: Scheme, density: Density) {
        self.look = Look::new(theme, scheme)
            .expect("a theme the contract has")
            .with_density(density)
            .with_depth(self.look.depth())
            .with_charset(self.look.charset());
    }

    fn groups(&self) -> Vec<CommandGroup> {
        let themes = tokens::themes().into_iter().map(|t| {
            let c = Command::new(format!("theme:{t}"), format!("Theme: {t}")).icon("◐");
            if t == self.look.theme() { c.meta("now", None) } else { c }
        });
        vec![
            CommandGroup::new(
                "Look",
                [
                    Command::new("scheme", "Switch light and dark")
                        .icon("◐")
                        .keys(Keys::Char('s')),
                    Command::new("density", "Switch density")
                        .icon("≡")
                        .hint(match self.look.density() {
                            Density::Regular => "regular now",
                            Density::Compact => "compact now",
                        }),
                    Command::new("settings", "Settings").icon(self.look.glyphs().search),
                ],
            ),
            CommandGroup::new("Themes", themes),
            CommandGroup::new(
                "Runs",
                [
                    Command::new("export", "Export runs")
                        .icon("↓")
                        .meta("1,200", None)
                        .keywords(["download"]),
                    Command::new("retry", "Retry sync-inventory")
                        .icon("↻")
                        .meta("failed", Some(CommandTone::Critical)),
                    Command::new("slow", "Rebuild every index")
                        .icon("⟳")
                        .hint("takes a while"),
                    Command::new("pause", "Pause nightly-export")
                        .icon("‖")
                        .disabled("It is not running"),
                ],
            ),
        ]
    }

    fn run(&mut self, id: &str) {
        let (scheme, density) = (self.look.scheme(), self.look.density());
        let theme = self.look.theme().to_string();
        match id {
            "scheme" => self.relook(
                &theme,
                if scheme == Scheme::Dark {
                    Scheme::Light
                } else {
                    Scheme::Dark
                },
                density,
            ),
            "density" => self.relook(
                &theme,
                scheme,
                if density == Density::Regular {
                    Density::Compact
                } else {
                    Density::Regular
                },
            ),
            "settings" => {
                self.layer = Layer::Settings;
                self.settings = SettingsState::default();
                return;
            }
            "export" => self.said = "Exported 1,200 runs.".into(),
            "retry" => {
                // Refused: the palette stays open and says why.
                self.palette.refused(
                    Some("sync-inventory is already running".into()),
                    &PaletteWords::default(),
                );
                return;
            }
            "slow" => {
                self.palette.running("slow", 0);
                self.late_since = Some(self.now);
                return;
            }
            t if t.starts_with("theme:") => self.relook(&t["theme:".len()..], scheme, density),
            _ => {}
        }
        self.layer = Layer::None;
    }

    fn settings_dialog(&self) -> SettingsDialog<'_> {
        SettingsDialog::new(
            &self.look,
            [SettingsGroup::new(
                None,
                [
                    SettingsSection::new(
                        "look",
                        "Look",
                        [
                            Setting::new("scheme", "Scheme", 10).description("Light or dark, on this machine."),
                            Setting::new("density", "Density", 10).description("Regular, or compact rows and padding."),
                        ],
                    )
                    .icon("◐")
                    .no_reset(),
                    SettingsSection::new(
                        "notify",
                        "Notifications",
                        [Setting::new("fail", "When a run fails", 10).description("Who is told, and how.")],
                    )
                    .icon("●")
                    .no_reset(),
                ],
            )],
        )
    }

    fn settings_key(&mut self, key: KeyPress) {
        let order = [
            SettingsFocus::Search,
            SettingsFocus::Entry(0, 0),
            SettingsFocus::Entry(0, 1),
            SettingsFocus::Control(0),
            SettingsFocus::Control(1),
            SettingsFocus::Done,
        ];
        if key.is(Key::Tab) || key.is(Key::BackTab) {
            let at = order.iter().position(|f| *f == self.settings.focus).unwrap_or(0);
            let n = order.len();
            self.settings.focus = order[if key.is(Key::Tab) {
                (at + 1) % n
            } else {
                (at + n - 1) % n
            }];
            return;
        }
        if key.is_press() {
            match self.settings.focus {
                SettingsFocus::Control(0) => return self.run("scheme"),
                SettingsFocus::Control(1) => return self.run("density"),
                _ => {}
            }
        }
        let mut st = std::mem::take(&mut self.settings);
        let event = self.settings_dialog().handle(&mut st, key);
        self.settings = st;
        if event == SettingsEvent::Close {
            self.layer = Layer::None;
        }
    }
}

impl common::App for App {
    fn look(&self) -> &Look {
        &self.look
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        if let Some(t) = self.late_since {
            if now - t > 9000 {
                self.late_since = None;
                self.palette.done();
                self.layer = Layer::None;
                self.said = "Every index rebuilt.".into();
            } else {
                self.palette.running("slow", now - t);
            }
        }
    }

    fn key(&mut self, key: KeyPress) -> bool {
        let ctrl_k = key.ctrl && matches!(key.key, Key::Char('k') | Key::Char('K'));
        match self.layer {
            Layer::None if ctrl_k => {
                self.palette = PaletteState::default();
                self.layer = Layer::Palette;
            }
            Layer::None => return key.is(Key::Escape),
            Layer::Palette => {
                let p = CommandPalette::new(&self.look, self.groups());
                match p.handle(&mut self.palette, key) {
                    PaletteEvent::Run(id) => self.run(&id),
                    PaletteEvent::Close => self.layer = Layer::None,
                    PaletteEvent::StopWaiting => {
                        self.late_since = None;
                        self.palette.done();
                        self.layer = Layer::None;
                        self.said = "Stopped waiting; the rebuild goes on without us.".into();
                    }
                    _ => {}
                }
            }
            Layer::Settings => self.settings_key(key),
        }
        false
    }

    fn pointer(&mut self, p: Pointer) {
        match (self.layer, p.kind) {
            (Layer::Palette, PointerKind::Move | PointerKind::Press) => {
                let pal = CommandPalette::new(&self.look, self.groups());
                let Some(area) = self.palette_at else { return };
                match pal.hit(area, p.at, &self.palette) {
                    Some(hit) if p.kind == PointerKind::Move => pal.hover(&mut self.palette, hit),
                    Some(hit) => match pal.press(&mut self.palette, hit) {
                        PaletteEvent::Run(id) => self.run(&id),
                        PaletteEvent::StopWaiting => {
                            self.late_since = None;
                            self.palette.done();
                            self.layer = Layer::None;
                            self.said = "Stopped waiting; the rebuild goes on without us.".into();
                        }
                        _ => {}
                    },
                    None if p.kind == PointerKind::Press && !area.contains(p.at) && self.late_since.is_none() => {
                        self.layer = Layer::None
                    }
                    None => {}
                }
            }
            (Layer::Palette, PointerKind::ScrollDown) => {
                self.key(KeyPress::new(Key::Down));
            }
            (Layer::Palette, PointerKind::ScrollUp) => {
                self.key(KeyPress::new(Key::Up));
            }
            (Layer::Settings, PointerKind::Press) => {
                let Some((area, l)) = self.settings_at.clone() else {
                    return;
                };
                let at = p.at;
                let hit = |r: Option<Rect>| r.is_some_and(|r| r.contains(at));
                if hit(l.close) || hit(l.done) || !area.contains(at) {
                    self.layer = Layer::None;
                } else if let Some(&((g, e), _)) = l.entries.iter().find(|(_, r)| r.contains(at)) {
                    self.settings.focus = SettingsFocus::Entry(g, e);
                    self.settings.shown = (g, e);
                } else if let Some(&(k, _)) = l.controls.iter().find(|(_, r)| r.contains(at)) {
                    self.settings.focus = SettingsFocus::Control(k);
                    self.run(if k == 0 { "scheme" } else { "density" });
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let look = &self.look;
        let area = f.area();
        f.buffer_mut().set_style(
            area,
            Style::new().bg(look.token("background")).fg(look.token("foreground")),
        );
        let foot_h = StatusLine::height(look);
        let page = Rect::new(
            area.x + 2,
            area.y + 1,
            area.width.saturating_sub(4),
            area.height.saturating_sub(foot_h + 1),
        );
        let header = PageHeader::new(look, "Runs")
            .count("1,200 runs")
            .actions([Button::new(look, "New").variant(ButtonVariant::Default)]);
        f.render_widget(
            &header,
            Rect {
                height: header.height(page.width),
                ..page
            },
        );
        let hint = TextRole::Body.style(look).fg(look.token("muted-foreground"));
        f.buffer_mut()
            .set_string(page.x, page.y + 3, "Ctrl+K opens the command palette.", hint);

        self.palette_at = None;
        self.settings_at = None;
        let width = Width::of(area);
        match self.layer {
            Layer::Palette => {
                let p = CommandPalette::new(look, self.groups()).window(width);
                let r = p.area(area, &self.palette);
                p.render(r, f.buffer_mut(), &mut self.palette);
                if let Some(pos) = self.palette.cursor_position() {
                    f.set_cursor_position(pos);
                }
                self.palette_at = Some(r);
            }
            Layer::Settings => {
                let mut st = std::mem::take(&mut self.settings);
                let d = self.settings_dialog().window(width);
                let r = d.area(area);
                d.render(r, f.buffer_mut(), &mut st);
                let l = d.layout(r, &st);
                for (k, c) in &l.controls {
                    let words = match (k, look.scheme(), look.density()) {
                        (0, Scheme::Dark, _) => "Dark",
                        (0, Scheme::Light, _) => "Light",
                        (1, _, Density::Regular) => "Regular",
                        (1, _, Density::Compact) => "Compact",
                        _ => "Everyone",
                    };
                    let b = Button::new(look, words)
                        .variant(ButtonVariant::Outline)
                        .focused(st.focus == SettingsFocus::Control(*k));
                    let w = b.width().min(c.width);
                    f.render_widget(&b, Rect { width: w, ..*c });
                }
                self.settings = st;
                self.settings_at = Some((r, l));
            }
            Layer::None => {}
        }
        let note = if self.said.is_empty() {
            "Ctrl K commands"
        } else {
            &self.said
        };
        let foot = StatusLine::new(look).note(note, NoteTone::Plain).facts([
            Fact::labeled("theme", look.theme()).code(),
            Fact::labeled("Ctrl Q", "quit"),
        ]);
        f.render_widget(
            &foot,
            Rect::new(area.x, area.bottom().saturating_sub(foot_h), area.width, foot_h),
        );
    }
}

fn main() -> std::io::Result<()> {
    let mut app = App {
        look: common::look(),
        layer: Layer::None,
        palette: PaletteState::default(),
        settings: SettingsState::default(),
        said: String::new(),
        now: 0,
        late_since: None,
        palette_at: None,
        settings_at: None,
    };
    common::run(&mut app)
}
