mod app;
mod config;
mod converter;
mod fs_utils;
mod preview;
mod ui;

use std::io;

use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::Rect;
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
    std::io::stdout().execute(EnableMouseCapture)?;
    let terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut DefaultTerminal) -> io::Result<()> {
    std::io::stdout().execute(DisableMouseCapture)?;
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

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                app.handle_key(key);
            }
            Event::Mouse(mouse) => {
                let size = terminal.size()?;
                let area = Rect::new(0, 0, size.width, size.height);
                app.handle_mouse(mouse, area);
            }
            _ => {}
        }
    }
    Ok(())
}
