//! The palette sheet: the command palette at rest, filtered, running past busy-release and refused,
//! and the settings dialog with a search narrowing it.

use ratatui::{Frame, layout::Rect, style::Style};
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Command, CommandGroup, CommandPalette, CommandTone, Keys, Look, PaletteState, PaletteWords, Setting,
    SettingsDialog, SettingsFocus, SettingsGroup, SettingsSection, SettingsState, TextRole,
};

fn note(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    if y < f.area().bottom() {
        let style = TextRole::Help.style(look).fg(look.token("muted-foreground"));
        f.buffer_mut().set_string(x, y, words, style);
    }
}

fn heading(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    if y < f.area().bottom() {
        let style = TextRole::SectionTitle.style(look).fg(look.token("foreground"));
        f.buffer_mut().set_string(x, y, words, style);
    }
}

fn groups() -> Vec<CommandGroup> {
    vec![
        CommandGroup::new(
            "Go to",
            [
                Command::new("go-runs", "Runs").icon("▶").keys(Keys::Then('g', 'r')),
                Command::new("go-jobs", "Jobs").icon("◆").keys(Keys::Then('g', 'j')),
                Command::new("go-hosts", "Hosts")
                    .icon("▣")
                    .hint("2 not answering")
                    .kind("Place"),
            ],
        ),
        CommandGroup::new(
            "Runs",
            [
                Command::new("new-run", "New run").icon("+").keys(Keys::Mod('n')),
                Command::new("export", "Export runs")
                    .icon("↓")
                    .meta("1,200", None)
                    .keywords(["download", "csv"]),
                Command::new("retry", "Retry sync-inventory")
                    .icon("↻")
                    .meta("failed", Some(CommandTone::Critical)),
                Command::new("pause", "Pause nightly-export")
                    .icon("‖")
                    .disabled("It is not running"),
            ],
        ),
    ]
}

fn settings(look: &Look) -> SettingsDialog<'_> {
    SettingsDialog::new(
        look,
        [
            SettingsGroup::new(
                None,
                [
                    SettingsSection::new(
                        "general",
                        "General",
                        [
                            Setting::new("theme", "Theme", 16).description("How every page looks."),
                            Setting::new("density", "Density", 14).description("Regular, or compact rows and padding."),
                            Setting::new("motion", "Reduce motion", 5)
                                .description("Spinners stand still; nothing slides.")
                                .note("Follows the system setting until you change it."),
                        ],
                    )
                    .icon("✱")
                    .lead("Applies on this machine."),
                    SettingsSection::new(
                        "notify",
                        "Notifications",
                        [Setting::new("fail", "When a run fails", 12).description("Who is told, and how.")],
                    )
                    .icon("●"),
                ],
            ),
            SettingsGroup::new(
                Some("Workspace"),
                [SettingsSection::new(
                    "hosts",
                    "Hosts",
                    [Setting::new("timeout", "Timeout", 10).description("How long a host may take to answer.")],
                )
                .icon("▣")
                .no_reset()],
            ),
        ],
    )
}

pub fn draw(f: &mut Frame, look: &Look) {
    let area = f.area();
    f.buffer_mut().set_style(
        area,
        Style::new().bg(look.token("background")).fg(look.token("foreground")),
    );
    let window = Width::of(area);
    let x0 = area.x + 2;
    let mut y = area.y + 1;
    heading(f, look, x0, y, "CommandPalette");
    y += 2;
    let two = area.width >= 100;
    let room_w = if two { (area.width - 6) / 2 } else { area.width - 4 };
    let room_h = 16;
    let words = PaletteWords::default();
    let palette = CommandPalette::new(look, groups()).window(window);
    let mut states = [
        ("at rest, the second row active", PaletteState::default()),
        ("filtered by what was typed", PaletteState::default()),
        ("a command running past busy-release", PaletteState::default()),
        ("a command refused", PaletteState::default()),
    ];
    states[0].1.active = 1;
    states[1].1.query = "ex".into();
    states[2].1.running("export", 6000);
    states[3].1.active = 4;
    states[3]
        .1
        .refused(Some("api.example.org refused the export: too many runs".into()), &words);
    for (i, (name, state)) in states.iter_mut().enumerate() {
        let (c, r) = if two { (i % 2, i / 2) } else { (0, i) };
        let room = Rect::new(
            x0 + c as u16 * (room_w + 2),
            y + r as u16 * (room_h + 1),
            room_w,
            room_h,
        );
        if room.bottom() > area.bottom() {
            break;
        }
        note(f, look, room.x, room.y, name);
        let inner = Rect {
            y: room.y + 1,
            height: room.height - 1,
            ..room
        };
        let pa = palette.area(inner, state);
        palette.render(pa, f.buffer_mut(), state);
    }
    y += if two { 2 * (room_h + 1) } else { 4 * (room_h + 1) };

    heading(f, look, x0, y, "SettingsDialog");
    y += 1;
    let dialog = settings(look).window(window);
    let room = Rect::new(area.x, y, area.width, area.bottom().saturating_sub(y));
    let mut state = SettingsState::default();
    state.focus = SettingsFocus::Entry(0, 1);
    if window == Width::Narrow {
        state.in_section = true;
    }
    let da = dialog.area(room);
    dialog.render(da, f.buffer_mut(), &mut state);
    // The app's controls, where the layout puts them: here the words a select would show.
    let layout = dialog.layout(da, &state);
    let shows = ["Tidevann ▾", "Regular ▾", "Off"];
    for (k, r) in layout.controls {
        let style = TextRole::Body
            .style(look)
            .fg(look.token("foreground"))
            .bg(look.token("field"));
        f.buffer_mut().set_stringn(
            r.x,
            r.y,
            format!(" {}", shows.get(k).unwrap_or(&"")),
            r.width as usize,
            style,
        );
    }
}
