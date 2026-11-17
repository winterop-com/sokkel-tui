//! The parts sheets: every part the core holds, in its variants and states, drawn from the library
//! alone.
//!
//! `cargo run --example 08_gallery` shows them in the terminal: ← and → (or Tab) move between the
//! sheets, `t` the theme, `s` the scheme, `a` ASCII, `c` no color; ↑ ↓, PgUp PgDn, Home End and the
//! wheel scroll; `m` turns mouse capture off and on (off: the terminal selects text as usual);
//! `q` or Esc quits.
//!
//! With arguments it writes one sheet out as SVG and text, cell for cell, for `make specimens`:
//! cargo run --example 08_gallery -- <controls|page|table|overlays|palette> <theme> <light|dark> <cols> <rows> <out-without-extension> [ascii] [nocolor]
//!
//! `controls` is Button and TextField; `page` is PageState, PageHeader and Section; `table` is the
//! table; `overlays` the dialog, the dropdown menu and the select; `palette` the command
//! palette and the settings dialog.

use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
use sokkel_tui::{
    Button, ButtonSize, ButtonVariant, Charset, Depth, FieldFocus, FieldState, IconPlace, Issue, Look, RequiredMark,
    Scheme, TextField, TextRole, snapshot,
};

mod overlays;
mod page;
mod palette;
mod table;

const STATES: [&str; 7] = ["rest", "focus", "hover", "busy", "reason", "invalid", "open"];

fn state_button<'a>(look: &'a Look, variant: ButtonVariant, state: &str) -> Button<'a> {
    let b = Button::new(look, "Export").variant(variant);
    match state {
        "focus" => b.focused(true),
        "hover" => b.hovered(true),
        "busy" => b.busy(true, 300),
        "reason" => b.reason(Some("Nothing to export yet")),
        "invalid" => b.invalid(true),
        "open" => b.opens_menu(true),
        _ => b,
    }
}

fn heading(f: &mut Frame, look: &Look, x: u16, y: u16, words: &str) {
    let style = TextRole::SectionTitle.style(look).fg(look.token("foreground"));
    f.buffer_mut().set_string(x, y, words, style);
}

fn quiet(look: &Look) -> Style {
    TextRole::Help.style(look).fg(look.token("muted-foreground"))
}

fn draw_controls(f: &mut Frame, look: &Look) {
    let area = f.area();
    f.buffer_mut().set_style(
        area,
        Style::new().bg(look.token("background")).fg(look.token("foreground")),
    );
    let x0 = area.x + 2;
    let mut y = area.y + 1;

    // Buttons: each variant in each state, the states split over rows as the width allows.
    heading(f, look, x0, y, "Button");
    y += 2;
    let lead = 13u16;
    let cell = state_button(look, ButtonVariant::Destructive, "busy").width() + 1;
    let per_row = ((area.width.saturating_sub(lead + 4)) / cell).max(1) as usize;
    let variants = [
        ButtonVariant::Default,
        ButtonVariant::Outline,
        ButtonVariant::Secondary,
        ButtonVariant::Ghost,
        ButtonVariant::Destructive,
        ButtonVariant::Link,
    ];
    for states in STATES.chunks(per_row) {
        for (i, s) in states.iter().enumerate() {
            let x = x0 + lead + i as u16 * cell + 1;
            f.buffer_mut().set_string(x, y, *s, quiet(look));
        }
        y += 1;
        for v in variants {
            f.buffer_mut().set_string(x0, y, v.key(), quiet(look));
            for (i, s) in states.iter().enumerate() {
                let b = state_button(look, v, s);
                let x = x0 + lead + i as u16 * cell;
                f.render_widget(&b, Rect::new(x, y, b.width(), 1));
            }
            y += 1;
        }
        y += 1;
    }

    // Sizes, icons, icon buttons, destructive hover.
    let row: Vec<(&str, Button)> = vec![
        (
            "xs",
            Button::new(look, "Retry")
                .size(ButtonSize::Xs)
                .variant(ButtonVariant::Outline),
        ),
        (
            "sm",
            Button::new(look, "Retry")
                .size(ButtonSize::Sm)
                .variant(ButtonVariant::Outline),
        ),
        ("lg", Button::new(look, "Retry").size(ButtonSize::Lg)),
        (
            "icon",
            Button::new(look, "New run")
                .with_icon("+", IconPlace::Start)
                .variant(ButtonVariant::Secondary),
        ),
        (
            "end",
            Button::new(look, "Open")
                .with_icon("›", IconPlace::End)
                .variant(ButtonVariant::Ghost),
        ),
    ];
    let mut x = x0;
    for (name, b) in &row {
        let w = b.width().max(name.len() as u16 + 2);
        if x + w > area.right() {
            x = x0;
            y += 3;
        }
        f.buffer_mut().set_string(x + 1, y, *name, quiet(look));
        f.render_widget(b, Rect::new(x, y + 1, b.width(), 1));
        x += w + 1;
    }
    y += 3;
    let row: Vec<(&str, Button)> = vec![
        ("icon button", Button::icon(look, "⋯", "More actions")),
        ("focus", Button::icon(look, "⋯", "More actions").focused(true)),
        ("pressed", Button::icon(look, "★", "Pin").pressed(true)),
        ("busy", Button::icon(look, "⟳", "Refresh").busy(true, 600)),
        (
            "destructive hover",
            Button::new(look, "Cancel run")
                .variant(ButtonVariant::Outline)
                .destructive_hover(true)
                .hovered(true),
        ),
    ];
    let mut x = x0;
    for (name, b) in &row {
        let w = b.width().max(name.len() as u16 + 2);
        if x + w > area.right() {
            x = x0;
            y += 3;
        }
        f.buffer_mut().set_string(x + 1, y, *name, quiet(look));
        f.render_widget(b, Rect::new(x, y + 1, b.width(), 1));
        x += w + 1;
    }
    y += 4;

    // Text fields, in two columns where there is room.
    heading(f, look, x0, y, "TextField");
    y += 2;
    let two = area.width >= 100;
    let col_w = if two { (area.width - 4 - 4) / 2 } else { area.width - 4 };
    let schedule_issue = Issue::critical("Not a schedule: the hour is 0 to 23.")
        .detail("cron reads five fields: minute, hour, day, month, weekday.")
        .action("Use 0 2 * * *");
    let fields: Vec<(TextField, FieldState)> = vec![
        (
            TextField::new(look, "Job name").placeholder("nightly-export"),
            FieldState::default(),
        ),
        (
            TextField::new(look, "Host")
                .hint("host name")
                .help("Where the job's runs report to."),
            FieldState::new("api.example.org").focused(FieldFocus::Control),
        ),
        (
            TextField::new(look, "Schedule")
                .required(RequiredMark::Star)
                .description("When the job runs, in cron's five fields.")
                .issues([schedule_issue]),
            FieldState::new("0 25 * * *").issue_open(true),
        ),
        (
            TextField::new(look, "Timeout")
                .key_hint("run.timeout")
                .issues([Issue::warning("Longer than the host allows; runs stop at 60m.")])
                .fallback("Default is 30 minutes."),
            FieldState::new("90m").focused(FieldFocus::Mark),
        ),
        (
            TextField::new(look, "Owner")
                .required(RequiredMark::Word)
                .disabled(true),
            FieldState::new("ada"),
        ),
        (
            TextField::new(look, "Project")
                .read_only(true)
                .help("Set when the job is made."),
            FieldState::new("harbor"),
        ),
        (
            TextField::new(look, "Note")
                .multiline(3, 5)
                .help("Enter starts a line; it grows to five."),
            FieldState::new("sync-inventory failed twice at 11:58.\nAsk grace before the next run."),
        ),
    ];
    let mut states = Vec::new();
    let top = y;
    let mut col_y = [top, top];
    for (i, (field, mut state)) in fields.into_iter().enumerate() {
        let c = if two { i % 2 } else { 0 };
        let x = x0 + c as u16 * (col_w + 4);
        let h = field.height_with(col_w, &state);
        let r = Rect::new(x, col_y[c], col_w, h).intersection(area);
        f.render_stateful_widget(&field, r, &mut state);
        col_y[c] += h + 1;
        states.push((field, state));
    }
    // Overlays last, over what lies below.
    for (field, state) in &states {
        if let Some(p) = field.popover(state) {
            f.render_widget(p, area);
        }
        if let Some(pos) = state.cursor_position() {
            f.set_cursor_position(pos);
        }
    }
    // The tips the status line would say, for the stops that have focus.
    let tips: Vec<String> = states.iter().filter_map(|(field, s)| field.tip(s)).collect();
    if let Some(tip) = tips.first() {
        let y = area.bottom() - 1;
        f.render_widget(
            Line::from(vec![Span::styled(format!("  {tip}"), quiet(look))]),
            Rect::new(area.x, y, area.width, 1),
        );
    }
}

/// The sheets, and the rows each takes at a wide and at a narrow width.
const SHEETS: [(&str, u16, u16); 5] = [
    ("controls", 40, 60),
    ("page", 84, 120),
    ("table", 90, 90),
    ("overlays", 56, 90),
    ("palette", 64, 100),
];

fn draw_sheet(f: &mut Frame, sheet: &str, look: &Look) {
    match sheet {
        "page" => page::draw(f, look),
        "table" => table::draw(f, look),
        "overlays" => overlays::draw(f, look),
        "palette" => palette::draw(f, look),
        _ => draw_controls(f, look),
    }
}

/// The sheets in the terminal, scrolled when taller than it.
fn view() -> std::io::Result<()> {
    use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, MouseEventKind};
    use ratatui::crossterm::execute;
    use sokkel_tui::{Fact, Key, KeyPress, NoteTone, StatusLine, tokens};

    let themes = tokens::themes();
    let (mut sheet, mut theme, mut scheme, mut ascii, mut nocolor) = (0usize, 0usize, Scheme::Dark, false, false);
    let mut scroll: u16 = 0;
    let mut mouse = true;
    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let result = (|| -> std::io::Result<()> {
        loop {
            let mut look = Look::new(themes[theme], scheme).expect("a theme the contract has");
            if ascii {
                look = look.with_charset(Charset::Ascii);
            }
            if nocolor {
                look = look.with_depth(Depth::None);
            }
            let (name, tall, narrow) = SHEETS[sheet];
            let mut max_scroll = 0;
            terminal.draw(|f| {
                let area = f.area();
                let rows = if area.width >= 100 { tall } else { narrow };
                // The sheet, drawn whole off screen at the terminal's width.
                let mut off = Terminal::new(TestBackend::new(area.width, rows)).expect("a buffer");
                off.draw(|g| draw_sheet(g, name, &look)).expect("a sheet");
                let src = off.backend().buffer();
                let view = area.height.saturating_sub(1);
                max_scroll = rows.saturating_sub(view);
                scroll = scroll.min(max_scroll);
                for y in 0..view.min(rows) {
                    for x in 0..area.width {
                        f.buffer_mut()[(x, y)] = src[(x, y + scroll)].clone();
                    }
                }
                let foot = StatusLine::new(&look)
                    .note(
                        "← → sheets  t theme  s scheme  a ascii  c no color  m mouse  q quit",
                        NoteTone::Plain,
                    )
                    .facts([
                        Fact::labeled("mouse", if mouse { "on" } else { "off" }),
                        Fact::labeled("sheet", name).code(),
                        Fact::labeled("theme", themes[theme]).code(),
                        Fact::new(match scheme {
                            Scheme::Dark => "dark",
                            Scheme::Light => "light",
                        }),
                    ]);
                f.render_widget(&foot, Rect::new(0, area.height.saturating_sub(1), area.width, 1));
            })?;
            let page = terminal.size()?.height.saturating_sub(2).max(1);
            match event::read()? {
                Event::Key(k) => {
                    let Some(key) = KeyPress::from_crossterm(&k) else {
                        continue;
                    };
                    if key.ctrl && key.key == Key::Char('c') {
                        return Ok(());
                    }
                    match key.key {
                        Key::Char('q') | Key::Escape => return Ok(()),
                        Key::Right | Key::Tab => {
                            sheet = (sheet + 1) % SHEETS.len();
                            scroll = 0;
                        }
                        Key::Left | Key::BackTab => {
                            sheet = (sheet + SHEETS.len() - 1) % SHEETS.len();
                            scroll = 0;
                        }
                        Key::Char('t') => theme = (theme + 1) % themes.len(),
                        Key::Char('T') => theme = (theme + themes.len() - 1) % themes.len(),
                        Key::Char('s') => {
                            scheme = match scheme {
                                Scheme::Dark => Scheme::Light,
                                Scheme::Light => Scheme::Dark,
                            }
                        }
                        Key::Char('a') => ascii = !ascii,
                        Key::Char('c') => nocolor = !nocolor,
                        Key::Char('m') => {
                            mouse = !mouse;
                            if mouse {
                                execute!(std::io::stdout(), EnableMouseCapture)?;
                            } else {
                                execute!(std::io::stdout(), DisableMouseCapture)?;
                            }
                        }
                        Key::Down => scroll = (scroll + 1).min(max_scroll),
                        Key::Up => scroll = scroll.saturating_sub(1),
                        Key::PageDown | Key::Char(' ') => scroll = (scroll + page).min(max_scroll),
                        Key::PageUp => scroll = scroll.saturating_sub(page),
                        Key::Home => scroll = 0,
                        Key::End => scroll = max_scroll,
                        _ => {}
                    }
                }
                Event::Mouse(m) => match m.kind {
                    MouseEventKind::ScrollDown => scroll = (scroll + 3).min(max_scroll),
                    MouseEventKind::ScrollUp => scroll = scroll.saturating_sub(3),
                    _ => {}
                },
                _ => {}
            }
        }
    })();
    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        if let Err(e) = view() {
            eprintln!("parts: {e}");
            std::process::exit(1);
        }
        return;
    }
    if args.len() < 7 {
        eprintln!(
            "usage: cargo run --example 08_gallery -- <controls|page|table|overlays|palette> <theme> <light|dark> <cols> <rows> <out-without-extension> [ascii] [nocolor]"
        );
        std::process::exit(2);
    }
    let sheet = args[1].as_str();
    let scheme = if args[3] == "light" {
        Scheme::Light
    } else {
        Scheme::Dark
    };
    let mut look = Look::new(&args[2], scheme).unwrap_or_else(|| panic!("the contract has no theme {}", args[2]));
    if args[7..].iter().any(|a| a == "ascii") {
        look = look.with_charset(Charset::Ascii);
    }
    if args[7..].iter().any(|a| a == "nocolor") {
        look = look.with_depth(Depth::None);
    }
    let (cols, rows): (u16, u16) = (args[4].parse().unwrap(), args[5].parse().unwrap());
    let mut term = Terminal::new(TestBackend::new(cols, rows)).unwrap();
    term.draw(|f| draw_sheet(f, sheet, &look)).unwrap();
    let buffer = term.backend().buffer();
    let plain = Look::new(&args[2], scheme).unwrap();
    let ground = plain.token("background");
    let ink = plain.token("foreground");
    std::fs::write(format!("{}.svg", args[6]), snapshot::svg(buffer, ground, ink)).unwrap();
    std::fs::write(format!("{}.txt", args[6]), snapshot::text(buffer)).unwrap();
}
