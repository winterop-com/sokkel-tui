//! The run loop the numbered examples share: the terminal taken and given back, the mouse
//! captured, a frame each `motion-fast` so spinners turn, keys and pointer events handed on.
//! Not an example of its own: cargo makes examples only of `examples/*.rs` and of a folder that
//! holds a `main.rs`.

#![allow(dead_code)]

use ratatui::Frame;
use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;
use sokkel_tui::{Charset, Depth, Key, KeyPress, Look, Pointer, Scheme};
use std::time::{Duration, Instant};

/// What an example answers.
pub trait App {
    fn look(&self) -> &Look;
    fn draw(&mut self, f: &mut Frame);
    /// A key; true quits.
    fn key(&mut self, key: KeyPress) -> bool;
    fn pointer(&mut self, _p: Pointer) {}
    fn tick(&mut self, _now_ms: u64) {}
}

/// The look an example starts in: tidevann, dark unless `light` is given, the terminal's depth and
/// character set.
pub fn look() -> Look {
    let light = std::env::args().any(|a| a == "light");
    Look::new("tidevann", if light { Scheme::Light } else { Scheme::Dark })
        .expect("tidevann")
        .with_depth(Depth::detect())
        .with_charset(Charset::detect())
}

/// Ctrl+Q and Ctrl+C quit every example.
pub fn quits(key: KeyPress) -> bool {
    key.ctrl && matches!(key.key, Key::Char('q') | Key::Char('c'))
}

pub fn run(app: &mut impl App) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let start = Instant::now();
    let result = (|| -> std::io::Result<()> {
        loop {
            app.tick(start.elapsed().as_millis() as u64);
            terminal.draw(|f| app.draw(f))?;
            let wait = Duration::from_millis(app.look().duration_ms("motion-fast"));
            if event::poll(wait)? {
                match event::read()? {
                    Event::Key(k) => {
                        if let Some(key) = KeyPress::from_crossterm(&k)
                            && (quits(key) || app.key(key))
                        {
                            return Ok(());
                        }
                    }
                    Event::Mouse(m) => {
                        if let Some(p) = Pointer::from_crossterm(&m) {
                            app.pointer(p);
                        }
                    }
                    _ => {}
                }
            }
        }
    })();
    execute!(std::io::stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result
}
