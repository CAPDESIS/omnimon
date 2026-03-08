//! OmniMon TUI — Real-time terminal interface powered by ratatui + crossterm.
//!
//! Consumes the same `core::watcher` shared state as the Tauri desktop app,
//! running its own render loop on a dedicated thread without blocking the
//! background telemetry collector.

mod app;
mod event;
mod ui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

/// Launch the TUI and block until the user exits (press `q` or `Esc`).
pub fn run() -> io::Result<()> {
    // Start the core watcher so we get live telemetry.
    core::watcher::start_watcher();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new();
    let result = event::run_loop(&mut terminal, &mut app);

    // Restore terminal state regardless of outcome.
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}
