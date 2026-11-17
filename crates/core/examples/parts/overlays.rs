//! The overlays sheet: a dropdown menu open with a submenu, selects closed, open and quiet, and
//! dialogs confirming, busy past release, shut, and closing from the corner. Below md the dialogs
//! are sheets filling their room.

use ratatui::{Frame, layout::Rect, style::Style, widgets::Widget};
use sokkel_tui::cells::Width;
use sokkel_tui::{
    Button, ButtonVariant, Confirm, Dialog, DialogFocus, DialogWidth, DropdownMenu, Item, Look, MenuState, PopupRow,
    Select, SelectOption, SelectState, SelectVariant, TextRole,
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

fn body(f: &mut Frame, look: &Look, r: Rect, words: &str) {
    let style = TextRole::Body.style(look).fg(look.token("popover-foreground"));
    for (i, line) in sokkel_tui::text::wrap(words, r.width as usize).into_iter().enumerate() {
        let y = r.y + i as u16;
        if y < r.bottom() {
            f.buffer_mut().set_string(r.x, y, line, style);
        }
    }
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

    // The menu, open, its submenu too.
    heading(f, look, x0, y, "DropdownMenu");
    heading(f, look, x0 + 40, y, "Select");
    y += 2;
    let trigger = Button::new(look, "Actions")
        .variant(ButtonVariant::Outline)
        .opens_menu(true);
    let tr = Rect::new(x0 + 1, y, trigger.width(), 1);
    f.render_widget(&trigger, tr);
    let menu = DropdownMenu::new(
        look,
        [
            PopupRow::Label("Run".into()),
            Item::new("Run now").icon("▶").shortcut("Ctrl R").into(),
            Item::new("Run with").icon("▷").into(),
            Item::new("Pause").icon("‖").disabled(true).into(),
            PopupRow::Separator,
            Item::new("Notify on failure").check(true).into(),
            Item::new("Notify on success").check(false).into(),
            PopupRow::Separator,
            Item::new("Delete").icon("×").destructive().into(),
        ],
    )
    .submenu(
        2,
        [
            Item::new("harbor").into(),
            Item::new("juniper").into(),
            Item::new("larkspur").into(),
        ],
    );
    let menu_state = MenuState {
        open: true,
        highlight: 2,
        sub: Some((2, 1)),
    };

    // Selects: the field closed with a value, the field open, the quiet one focused.
    let options = || {
        [
            SelectOption::new("Every hour").detail("0 * * * *"),
            SelectOption::new("Every night").detail("0 2 * * *"),
            SelectOption::new("Every week").detail("0 2 * * 1"),
            SelectOption::new("By hand").disabled(true),
        ]
    };
    let sx = x0 + 40;
    let field = Select::new(look, options()).value(Some(1));
    let closed = Rect::new(sx, y, 26, 1);
    field.render_trigger(closed, f.buffer_mut(), &SelectState::default());
    let open = Select::new(look, options()).value(Some(1)).focused(true);
    let open_r = Rect::new(sx, y + 2, 26, 1);
    let open_state = SelectState {
        open: true,
        highlight: 2,
    };
    open.render_trigger(open_r, f.buffer_mut(), &open_state);
    let quiet = Select::new(
        look,
        [SelectOption::new("Newest first"), SelectOption::new("Oldest first")],
    )
    .value(Some(0))
    .variant(SelectVariant::Quiet)
    .focused(true);
    // Beside the field where there is room, else under the open list.
    let quiet_at = if sx + 30 + 18 <= area.right() {
        Rect::new(sx + 30, y, 16, 1)
    } else {
        Rect::new(sx + 1, y + 10, 16, 1)
    };
    quiet.render_trigger(quiet_at, f.buffer_mut(), &SelectState::default());

    // Overlays last: the menu and the open list over what lies under them.
    let menu_window = Rect::new(area.x, area.y, sx.saturating_sub(2).max(30), area.height);
    menu.render(menu_window, tr, f.buffer_mut(), &menu_state);
    open.render_list(area, open_r, f.buffer_mut(), &open_state);
    y += 15;

    // Dialogs, each in a room of its own.
    heading(f, look, x0, y, "Dialog");
    y += 2;
    let two = area.width >= 100;
    let room_w = if two { (area.width - 6) / 2 } else { area.width - 4 };
    let room_h = 15;
    let rooms: Vec<Rect> = (0..4)
        .map(|i| {
            let (c, r) = if two { (i % 2, i / 2) } else { (0, i) };
            Rect::new(
                x0 + c as u16 * (room_w + 2),
                y + r as u16 * (room_h + 1),
                room_w,
                room_h,
            )
        })
        .collect();
    let dialogs = [
        (
            "confirm, destructive, its confirm focused",
            Dialog::new(look, "Delete nightly-export?")
                .description("Its 1,200 runs go with it.")
                .width(DialogWidth::Sm)
                .confirm(Confirm::new("Delete").destructive())
                .focus(DialogFocus::Confirm),
            "Runs that are going stop first.",
        ),
        (
            "busy past busy-release: Cancel says Stop waiting",
            Dialog::new(look, "Export runs")
                .width(DialogWidth::Sm)
                .confirm(Confirm::new("Export").busy(true, Some("Exporting")))
                .busy_for(6000)
                .focus(DialogFocus::Cancel),
            "1,200 runs to api.example.org.",
        ),
        (
            "shut: its reason in the status line",
            Dialog::new(look, "New job")
                .width(DialogWidth::Sm)
                .back(true)
                .note("Step 2 of 2")
                .confirm(Confirm::new("Create").why(Some("Give the job a schedule first")))
                .focus(DialogFocus::Confirm),
            "Schedule: none yet.",
        ),
        (
            "nothing to confirm: it closes from the corner",
            Dialog::new(look, "About harbor")
                .width(DialogWidth::Sm)
                .actions([Button::new(look, "Open").variant(ButtonVariant::Outline)])
                .focus(DialogFocus::Close),
            "Owned by ada. Three jobs, two hosts.",
        ),
    ];
    for ((name, dialog, words), room) in dialogs.into_iter().zip(rooms) {
        if room.bottom() >= area.bottom() {
            break;
        }
        note(f, look, room.x, room.y.saturating_sub(1), name);
        let dialog = dialog.window(window);
        let r = dialog.area(room, 2);
        (&dialog).render(r, f.buffer_mut());
        let layout = dialog.layout(r);
        body(f, look, layout.body, words);
        if let Some(tip) = dialog.tip() {
            note(f, look, room.x, room.bottom(), &format!("status line: {tip}"));
        }
    }
}
