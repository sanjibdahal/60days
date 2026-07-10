mod pet;
mod github;
mod storage;

use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal, Frame,
};
use pet::Pet;

const GITHUB_REFRESH: u64 = 300;
const TICK_MS:        u64 = 80;

struct App {
    pet:         Pet,
    last_github: Instant,
    fetching:    bool,
    fetch_error: Option<String>,
}

impl App {
    fn new() -> Self {
        let mut pet = storage::load().unwrap_or_else(|| {
            let username = std::env::args().nth(1).unwrap_or_else(|| String::from("sanjibdahal"));
            let name     = std::env::args().nth(2).unwrap_or_else(|| String::from("Whiskers"));
            Pet::new(name, username)
        });

        if let Some(username) = std::env::args().nth(1) { pet.username = username; }
        if let Some(name)     = std::env::args().nth(2) { pet.name     = name; }

        App {
            pet,
            last_github:  Instant::now() - Duration::from_secs(GITHUB_REFRESH + 1),
            fetching:     false,
            fetch_error:  None,
        }
    }

    fn maybe_fetch(&mut self) {
        if self.last_github.elapsed().as_secs() >= GITHUB_REFRESH && !self.fetching {
            self.fetching = true;
            let username  = self.pet.username.clone();
            match github::fetch_stats(&username) {
                Ok(stats) => {
                    self.pet.commits_today = stats.commits_today;
                    self.pet.total_commits = stats.total_commits;
                    self.pet.streak        = stats.streak;
                    self.pet.last_commit   = stats.last_commit;
                    self.pet.top_language  = stats.top_language;
                    self.pet.update_mood();
                    self.fetch_error = None;
                    storage::save(&self.pet);
                }
                Err(e) => {
                    self.fetch_error = Some(e);
                    self.pet.update_mood();
                }
            }
            self.last_github = Instant::now();
            self.fetching    = false;
        }
    }
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top info bar
            Constraint::Min(0),     // pet walk area
            Constraint::Length(1),  // bottom stats
        ])
        .split(area);

    let color = app.pet.mood_color();

    // top bar — name, mood, streak
    let streak_str = format!("🔥{} days", app.pet.streak);
    let top = Line::from(vec![
        Span::raw(" "),
        Span::styled(&app.pet.name, Style::default().fg(color).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(app.pet.mood_label(), Style::default().fg(color)),
        Span::raw("  "),
        Span::styled(&streak_str, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(&app.pet.top_language, Style::default().fg(Color::Cyan)),
    ]);
    f.render_widget(
        Paragraph::new(top).style(Style::default().bg(Color::Reset)),
        chunks[0],
    );

    // pet walk area
    let walk_area = chunks[1];
    let art       = app.pet.ascii_art();
    let status    = app.pet.status_message();
    let px        = (app.pet.x as u16).min(walk_area.width.saturating_sub(12));
    let py        = 0u16;

    for (i, line) in art.iter().enumerate() {
        let y = walk_area.y + py + i as u16;
        if y >= walk_area.y + walk_area.height { break; }
        let p = Paragraph::new(*line)
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD));
        let r = ratatui::layout::Rect {
            x:      walk_area.x + px,
            y,
            width:  10,
            height: 1,
        };
        if r.x + r.width <= walk_area.x + walk_area.width {
            f.render_widget(p, r);
        }
    }

    // status message next to pet
    let msg_y = walk_area.y + py + 1;
    let msg_x = (walk_area.x + px + 11).min(walk_area.x + walk_area.width.saturating_sub(status.len() as u16));
    if msg_y < walk_area.y + walk_area.height && msg_x + status.len() as u16 <= walk_area.x + walk_area.width {
        let p = Paragraph::new(status)
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
        let r = ratatui::layout::Rect { x: msg_x, y: msg_y, width: status.len() as u16, height: 1 };
        f.render_widget(p, r);
    }

    // bottom stats bar
    let last_str = app.pet.last_commit.map(|lc| {
        let h = (chrono::Utc::now() - lc).num_hours();
        match h {
            0       => String::from("just now"),
            h if h < 24 => format!("{}h ago", h),
            h       => format!("{}d ago", h / 24),
        }
    }).unwrap_or_else(|| String::from("never"));

    let err_part = app.fetch_error.as_deref().unwrap_or("");
    let bottom = Line::from(vec![
        Span::raw(" commits today: "),
        Span::styled(app.pet.commits_today.to_string(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw("  last: "),
        Span::styled(&last_str, Style::default().fg(Color::White)),
        Span::raw("  total: "),
        Span::styled(app.pet.total_commits.to_string(), Style::default().fg(Color::White)),
        // Span::raw("  "),
        // Span::styled(err_part, Style::default().fg(Color::Red)),
    ]);
    f.render_widget(Paragraph::new(bottom), chunks[2]);
}

#[tokio::main]
async fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend  = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let mut app       = App::new();
    let mut last_tick = Instant::now();

    loop {
        app.maybe_fetch();

        let size = term.size()?;
        let safe_width = (size.width as f64).max(1.0);
        let safe_height = ((size.height as f64) - 2.0).max(1.0);
        app.pet.tick(safe_width, safe_height);

        term.draw(|f| draw(f, &app))?;

        let timeout = Duration::from_millis(TICK_MS)
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        app.last_github = Instant::now() - Duration::from_secs(GITHUB_REFRESH + 1);
                    }
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(TICK_MS) {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;
    Ok(())
}
