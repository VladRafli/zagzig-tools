use std::net::IpAddr;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::monitor::Monitor;
use crate::util::{format_ago, format_ms, TextInput};

pub const HINT: &str =
    "tab: next field   enter: add/toggle   x: remove   c: clear log   esc: back";

const INTERVALS: [(u64, &str); 7] = [
    (1, "1s"),
    (10, "10s"),
    (30, "30s"),
    (60, "1m"),
    (300, "5m"),
    (900, "15m"),
    (1800, "30m"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Hostname,
    Server,
    Interval,
    List,
}

pub struct State {
    pub hostname: TextInput,
    pub server: TextInput,
    pub interval_index: usize,
    pub field: Field,
    pub selected: usize,
    pub error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            hostname: TextInput::default(),
            server: TextInput::default(),
            interval_index: 3,
            field: Field::Hostname,
            selected: 0,
            error: None,
        }
    }
}

pub fn on_key(app: &mut App, key: KeyEvent) {
    let field = app.dns_monitor.field;
    match key.code {
        KeyCode::Tab => {
            app.dns_monitor.field = match field {
                Field::Hostname => Field::Server,
                Field::Server => Field::Interval,
                Field::Interval => Field::List,
                Field::List => Field::Hostname,
            };
        }
        KeyCode::BackTab => {
            app.dns_monitor.field = match field {
                Field::Hostname => Field::List,
                Field::Server => Field::Hostname,
                Field::Interval => Field::Server,
                Field::List => Field::Interval,
            };
        }
        KeyCode::Char(c) if field == Field::Hostname => app.dns_monitor.hostname.push(c),
        KeyCode::Backspace if field == Field::Hostname => app.dns_monitor.hostname.backspace(),
        KeyCode::Char(c) if field == Field::Server => app.dns_monitor.server.push(c),
        KeyCode::Backspace if field == Field::Server => app.dns_monitor.server.backspace(),
        KeyCode::Left if field == Field::Interval => {
            let i = &mut app.dns_monitor.interval_index;
            *i = i.checked_sub(1).unwrap_or(INTERVALS.len() - 1);
        }
        KeyCode::Right if field == Field::Interval => {
            let i = &mut app.dns_monitor.interval_index;
            *i = (*i + 1) % INTERVALS.len();
        }
        KeyCode::Up if field == Field::List => {
            app.dns_monitor.selected = app.dns_monitor.selected.saturating_sub(1);
        }
        KeyCode::Down if field == Field::List => {
            let len = app.monitor_engine.list().len();
            if len > 0 {
                app.dns_monitor.selected = (app.dns_monitor.selected + 1).min(len - 1);
            }
        }
        KeyCode::Enter if field == Field::List => toggle_selected(app),
        KeyCode::Char('x') if field == Field::List => remove_selected(app),
        KeyCode::Char('c') if field == Field::List => clear_selected(app),
        KeyCode::Enter => add_monitor(app),
        _ => {}
    }
}

fn add_monitor(app: &mut App) {
    let hostname = app.dns_monitor.hostname.value.trim().to_string();
    if hostname.is_empty() {
        app.dns_monitor.error = Some("Enter a hostname.".to_string());
        return;
    }

    let server_text = app.dns_monitor.server.value.trim().to_string();
    let server = if server_text.is_empty() {
        None
    } else {
        match server_text.parse::<IpAddr>() {
            Ok(ip) => Some(ip),
            Err(_) => {
                app.dns_monitor.error = Some(format!("'{server_text}' isn't a valid IP address."));
                return;
            }
        }
    };

    let interval_secs = INTERVALS[app.dns_monitor.interval_index].0;
    app.monitor_engine
        .add(hostname, server, server_text, Duration::from_secs(interval_secs));
    app.dns_monitor.hostname.clear();
    app.dns_monitor.server.clear();
    app.dns_monitor.error = None;
    app.dns_monitor.field = Field::Hostname;
}

fn toggle_selected(app: &mut App) {
    if let Some(monitor) = app.monitor_engine.list().into_iter().nth(app.dns_monitor.selected) {
        if monitor.running {
            app.monitor_engine.stop(monitor.id);
        } else {
            app.monitor_engine.start(monitor.id);
        }
    }
}

fn remove_selected(app: &mut App) {
    let monitors = app.monitor_engine.list();
    if let Some(monitor) = monitors.get(app.dns_monitor.selected) {
        app.monitor_engine.remove(monitor.id);
    }
    let new_len = app.monitor_engine.list().len();
    app.dns_monitor.selected = new_len.saturating_sub(1).min(app.dns_monitor.selected);
}

fn clear_selected(app: &mut App) {
    if let Some(monitor) = app.monitor_engine.list().get(app.dns_monitor.selected) {
        app.monitor_engine.clear_entries(monitor.id);
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = &app.dns_monitor;
    let monitors = app.monitor_engine.list();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(6)])
        .split(area);

    render_form(frame, outer[0], state);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[1]);

    render_list(frame, body[0], state, &monitors);
    render_detail(frame, body[1], state, &monitors);
}

fn field_style(active: bool) -> Style {
    if active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    }
}

fn render_form(frame: &mut Frame, area: Rect, state: &State) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(area);

    let hostname = Paragraph::new(format!("{}_", state.hostname.value)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Hostname")
            .border_style(field_style(state.field == Field::Hostname)),
    );
    frame.render_widget(hostname, cols[0]);

    let server = Paragraph::new(format!("{}_", state.server.value)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("DNS server (blank = system default)")
            .border_style(field_style(state.field == Field::Server)),
    );
    frame.render_widget(server, cols[1]);

    let interval = Paragraph::new(format!("◂ {} ▸", INTERVALS[state.interval_index].1)).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Check every")
            .border_style(field_style(state.field == Field::Interval)),
    );
    frame.render_widget(interval, cols[2]);
}

fn render_list(frame: &mut Frame, area: Rect, state: &State, monitors: &[Monitor]) {
    let items: Vec<ListItem> = if monitors.is_empty() {
        vec![ListItem::new("No monitors yet — fill in the form and press Enter.")]
    } else {
        monitors
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let latest = m.entries.first();
                let bullet_color = match (m.running, latest.map(|e| e.resolved)) {
                    (false, _) => Color::DarkGray,
                    (true, Some(true)) => Color::Green,
                    (true, Some(false)) => Color::Red,
                    (true, None) => Color::Yellow,
                };
                let mut spans = vec![
                    Span::styled("● ", Style::default().fg(bullet_color)),
                    Span::raw(m.hostname.clone()),
                ];
                if m.checking {
                    spans.push(Span::styled(" …", Style::default().fg(Color::DarkGray)));
                }
                let line = Line::from(spans);
                let style = if state.field == Field::List && i == state.selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                ListItem::new(line).style(style)
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Monitors")
            .border_style(field_style(state.field == Field::List)),
    );
    frame.render_widget(list, area);
}

fn render_detail(frame: &mut Frame, area: Rect, state: &State, monitors: &[Monitor]) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(err) = &state.error {
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
    }

    if let Some(monitor) = monitors.get(state.selected) {
        let via = if monitor.server_label.is_empty() {
            "via system default".to_string()
        } else {
            format!("via {}", monitor.server_label)
        };
        let status = if monitor.running { "running" } else { "stopped" };
        lines.push(Line::from(format!(
            "{}  ({via}, every {}, {status})",
            monitor.hostname,
            INTERVALS
                .iter()
                .find(|(secs, _)| Duration::from_secs(*secs) == monitor.interval)
                .map(|(_, label)| *label)
                .unwrap_or("custom"),
        )));
        lines.push(Line::from(""));

        if monitor.entries.is_empty() {
            lines.push(Line::from("No checks yet."));
        }
        for entry in &monitor.entries {
            let (color, text) = if entry.resolved {
                (
                    Color::Green,
                    format!(
                        "{}  {}",
                        entry
                            .addresses
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        format_ms(entry.query_time),
                    ),
                )
            } else {
                (
                    Color::Red,
                    entry.error.clone().unwrap_or_else(|| "not resolvable".to_string()),
                )
            };
            lines.push(Line::from(vec![
                Span::styled(format_ago(entry.timestamp), Style::default().fg(Color::DarkGray)),
                Span::raw("  "),
                Span::styled(text, Style::default().fg(color)),
            ]));
        }
    } else {
        lines.push(Line::from("Select a monitor (tab to the list, ↑/↓ to pick)."));
    }

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Log"));
    frame.render_widget(paragraph, area);
}
