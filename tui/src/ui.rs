use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, Focus, MENU_ITEMS};
use crate::screens;

pub fn render(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(22), Constraint::Min(20)])
        .split(root[0]);

    render_menu(frame, body[0], app);
    screens::render(frame, body[1], app);
    render_status_bar(frame, root[1], app);
}

fn render_menu(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = MENU_ITEMS
        .iter()
        .enumerate()
        .map(|(i, (_, label))| {
            let selected = i == app.menu_index;
            let style = if selected && app.focus == Focus::Menu {
                Style::default().add_modifier(Modifier::REVERSED)
            } else if selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(*label).style(style)
        })
        .collect();

    let list =
        List::new(items).block(Block::default().borders(Borders::ALL).title("zagzig-tui"));
    frame.render_widget(list, area);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let line = Line::from(Span::styled(
        app.status_hint(),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(Paragraph::new(line), area);
}
