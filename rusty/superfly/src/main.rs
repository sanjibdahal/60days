mod app;
mod fs;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, ConfirmAction, Mode};

fn main() -> io::Result<()> {
    let start = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app = App::new(start);
    let res = run(&mut term, &mut app);

    // restore terminal
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }
    Ok(())
}

fn run(term: &mut ratatui::Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> io::Result<()> {
    loop {
        app.tick();
        term.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match &app.mode {
                    Mode::Normal => handle_normal(app, key.code, key.modifiers),
                    Mode::Rename => handle_input(app, key.code, InputMode::Rename),
                    Mode::Search => handle_input(app, key.code, InputMode::Search),
                    Mode::Confirm(action) => {
                        let action = match action {
                            ConfirmAction::Delete => ConfirmAction::Delete,
                            ConfirmAction::Copy   => ConfirmAction::Copy,
                        };
                        handle_confirm(app, key.code, action);
                    }
                }

                if app.mode == Mode::Normal && key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}

fn handle_normal(app: &mut App, key: KeyCode, _mods: KeyModifiers) {
    match key {
        KeyCode::Char('q')              => {}  // handled in run()
        KeyCode::Char('j') | KeyCode::Down  => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up    => app.move_up(),
        KeyCode::Enter                      => app.enter(),
        KeyCode::Backspace | KeyCode::Left  => app.go_up(),
        KeyCode::Char('d')                  => app.start_delete(),
        KeyCode::Char('c')                  => app.copy(),
        KeyCode::Char('p')                  => app.paste(),
        KeyCode::Char('r')                  => app.start_rename(),
        KeyCode::Char('/')                  => app.start_search(),
        KeyCode::Esc                        => app.clear_search(),
        _                                   => {}
    }
}

enum InputMode { Rename, Search }

fn handle_input(app: &mut App, key: KeyCode, mode: InputMode) {
    match key {
        KeyCode::Esc => app.cancel_input(),
        KeyCode::Enter => match mode {
            InputMode::Rename => app.confirm_rename(),
            InputMode::Search => app.confirm_search(),
        },
        KeyCode::Backspace => { app.input.pop(); }
        KeyCode::Char(c)   => app.input.push(c),
        _                  => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyCode, action: ConfirmAction) {
    match key {
        KeyCode::Char('y') => match action {
            ConfirmAction::Delete => app.confirm_delete(),
            ConfirmAction::Copy   => {}
        },
        _ => {
            app.mode   = Mode::Normal;
            app.status = String::from("Cancelled");
        }
    }
}
