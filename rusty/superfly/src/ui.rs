use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use crate::app::{App, Mode};
use crate::fs::preview;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // top bar
            Constraint::Min(0),     // main
            Constraint::Length(1),  // status bar
        ])
        .split(f.size());

    draw_topbar(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_statusbar(f, app, chunks[2]);
}

fn draw_topbar(f: &mut Frame, app: &App, area: Rect) {
    let path = if app.in_search {
        format!("🔍 Search results in: {}", app.current_dir.display())
    } else {
        format!("📁 {}", app.current_dir.display())
    };

    let p = Paragraph::new(path)
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(p, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_file_list(f, app, chunks[0]);
    draw_preview(f, app, chunks[1]);
}

fn draw_file_list(f: &mut Frame, app: &App, area: Rect) {
    let entries = app.current_entries();
    let title   = if app.in_search {
        format!(" Results ({}) ", entries.len())
    } else {
        format!(" Files ({}) ", entries.len())
    };

    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let icon  = if e.is_dir { "▶ " } else { "  " };
            let size  = format!("{:>6}", e.size_display());
            let label = format!("{}{:<40} {}", icon, e.name, size);

            let style = if i == app.selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if e.is_dir {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(Span::styled(label, style)))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let (title, content) = match app.mode {
        Mode::Rename => (
            " Rename ".to_string(),
            format!("New name:\n\n  {}_", app.input),
        ),
        Mode::Search => (
            " Search ".to_string(),
            format!("Query:\n\n  {}_", app.input),
        ),
        Mode::Confirm(_) => (
            " Confirm ".to_string(),
            format!("{}\n\n  y = yes\n  n = no", app.status),
        ),
        Mode::Normal => {
            if let Some(entry) = app.selected_entry() {
                let size_str = if !app.folder_size.is_empty() {
                    format!(" [{}]", app.folder_size)
                } else {
                    String::new()
                };
                let content = preview(&entry.path);
                (format!(" {}{} ", entry.name, size_str), content)
            } else {
                (" Preview ".to_string(), String::from("Nothing selected"))
            }
        }
    };

    let p = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(title))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Gray));

    f.render_widget(p, area);
}

fn draw_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let style = match app.mode {
        Mode::Normal  => Style::default().fg(Color::DarkGray),
        Mode::Rename  => Style::default().fg(Color::Yellow),
        Mode::Search  => Style::default().fg(Color::Green),
        Mode::Confirm(_) => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let p = Paragraph::new(app.status.clone()).style(style);
    f.render_widget(p, area);
}
