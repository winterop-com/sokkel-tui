//! 05: the command palette and the settings dialog, as the runtime gives them to every app. Ctrl+K
//! opens the palette: the app's commands first (one disabled with its reason, one refused, one that
//! answers late enough to show Stop waiting), then the runtime's own (the settings, the scheme,
//! the density, every theme). `,` opens the settings: the look and the frame, then the app's own
//! section, Notifications.
//!
//! Usage: cargo run --example 05_palette [-- light]
//! Keys: Ctrl+K the palette, Up and Down, Enter, Escape; `,` the settings, Tab, Enter on a choice;
//! `?` the shortcuts. The mouse works on rows, entries and choices. Ctrl+Q quits.

use ratatui::Frame;
use ratatui::layout::Rect;
use sokkel_tui::app::{self, Chrome, Cx, Ran, ShellApp};
use sokkel_tui::{
    Command, CommandGroup, CommandTone, Fact, Look, NoteTone, PageHeader, Rail, RailEntry, RailFoot, RailGroup,
    Setting, SettingsSection, StatusLine, TextRole, TopBar,
};

const TOLD: [&str; 3] = ["Everyone", "The owner", "No one"];

struct Palette {
    said: String,
    /// When the slow rebuild started, while it runs.
    rebuilding: Option<u64>,
    now: u64,
    told: usize,
}

impl ShellApp for Palette {
    fn chrome<'a>(&'a self, look: &'a Look) -> Chrome<'a> {
        Chrome {
            rail: Rail::new(look, "T", "Tidevann")
                .groups([RailGroup::new(None, [RailEntry::new("Runs").icon("▶").active()])])
                .foot(RailFoot {
                    letter: "a".into(),
                    color: "kind-violet".into(),
                    name: "ada".into(),
                }),
            top: TopBar::new(look).trail(["Tidevann", "Runs"]),
            foot: StatusLine::new(look).note(self.said.clone(), NoteTone::Plain).facts([
                Fact::labeled("Ctrl K", "Commands"),
                Fact::labeled(",", "Settings"),
                Fact::labeled("told", TOLD[self.told]),
            ]),
        }
    }

    fn page(&mut self, f: &mut Frame, page: Rect, cx: &mut Cx) {
        let look = cx.look;
        let header = PageHeader::new(look, "Runs").description("Every action is a command: Ctrl+K finds it.");
        let h = header.height(page.width);
        f.render_widget(&header, Rect { height: h, ..page });
        let quiet = TextRole::Help.style(look).fg(look.token("muted-foreground"));
        for (k, line) in [
            "Export runs: done at once.",
            "Retry sync-inventory: refused, and the palette says why.",
            "Rebuild every index: still running after busy-release, so Stop waiting shows.",
            "Pause nightly-export: disabled, with its reason.",
        ]
        .iter()
        .enumerate()
        {
            f.buffer_mut()
                .set_string(page.x, page.y + h + 1 + k as u16, line, quiet);
        }
    }

    fn commands(&self) -> Vec<CommandGroup> {
        vec![CommandGroup::new(
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
        )]
    }

    fn command(&mut self, id: &str) -> Ran {
        match id {
            "export" => {
                self.said = "Exported 1,200 runs.".into();
                Ran::Done
            }
            "retry" => Ran::Refused(Some("sync-inventory is already running".into())),
            "slow" => {
                self.rebuilding = Some(self.now);
                Ran::Running
            }
            _ => Ran::Done,
        }
    }

    fn finished(&self, id: &str) -> bool {
        id != "slow" || self.rebuilding.is_none()
    }

    fn stop_waiting(&mut self, _id: &str) {
        self.said = "Stopped waiting; the rebuild goes on without us.".into();
    }

    fn tick(&mut self, now: u64) {
        self.now = now;
        if let Some(t) = self.rebuilding
            && now.saturating_sub(t) > 9000
        {
            self.rebuilding = None;
            self.said = "Every index rebuilt.".into();
        }
    }

    fn settings(&self, _look: &Look) -> Vec<SettingsSection> {
        vec![
            SettingsSection::new(
                "notify",
                "Notifications",
                [Setting::new("told", "When a run fails", 0)
                    .description("Who is told.")
                    .choice(TOLD, self.told)],
            )
            .icon("●")
            .no_reset(),
        ]
    }

    fn chose(&mut self, id: &str, option: usize) {
        if id == "told" {
            self.told = option;
        }
    }
}

fn main() -> std::io::Result<()> {
    let app = Palette {
        said: "Ctrl+K opens the command palette".into(),
        rebuilding: None,
        now: 0,
        told: 0,
    };
    app::run_shell(app, app::look_from_args("tidevann"))
}
