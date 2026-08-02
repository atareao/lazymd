mod app;
mod config;
mod converter;
mod fs_utils;
mod preview;
mod ui;

use std::io;

use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::{DefaultTerminal, Terminal};

use app::App;

fn main() -> io::Result<()> {
    let mut terminal = init_terminal()?;
    let mut app = App::new();
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

fn init_terminal() -> io::Result<DefaultTerminal> {
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut DefaultTerminal) -> io::Result<()> {
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while app.running {
        terminal.draw(|frame| {
            let area = frame.area();
            ui::render(app, frame, area);
        })?;

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key);
        }
    }
    Ok(())
}
