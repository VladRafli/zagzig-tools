mod app;
mod dns;
mod monitor;
mod ping;
mod screens;
mod sysdns;
mod ui;
mod update;
mod util;

use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};

use app::App;

const TICK_RATE: Duration = Duration::from_millis(150);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();

    let result = run(&mut terminal, &mut app);
    let restart_requested = app.restart_requested;

    ratatui::restore();

    // The new binary is already on disk (self_update swapped it in via
    // self_replace) — this process just still has the old code loaded, so
    // relaunching means spawning a fresh copy of the same executable and
    // letting this one exit normally afterward.
    if restart_requested {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).spawn();
        }
    }

    result
}

// A synchronous poll/read loop on the tokio multi-thread runtime's calling
// task: this blocks its own worker thread for up to TICK_RATE per iteration,
// but background monitor/ping/DNS tasks (spawned via tokio::spawn elsewhere)
// run on the runtime's other worker threads, so the UI staying "blocked"
// here doesn't stall them.
fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key);
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}
