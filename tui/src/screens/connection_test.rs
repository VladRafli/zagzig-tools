use std::sync::{Arc, Mutex};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ping::{self, PingResult};
use crate::util::{format_ago, format_ms, TextInput};

pub const HINT: &str = "type a host, enter: run test   esc: back to menu";

#[derive(Clone)]
struct HistoryEntry {
    target: String,
    timestamp: Instant,
    reachable: bool,
}

#[derive(Default)]
struct Shared {
    running: bool,
    result: Option<PingResult>,
    history: Vec<HistoryEntry>,
}

pub struct State {
    pub input: TextInput,
    shared: Arc<Mutex<Shared>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            input: TextInput::default(),
            shared: Arc::new(Mutex::new(Shared::default())),
        }
    }
}

impl State {
    pub fn last_summary(&self) -> Option<String> {
        let shared = self.shared.lock().unwrap();
        shared.history.first().map(|h| {
            format!(
                "{} — {}",
                h.target,
                if h.reachable { "reachable" } else { "unreachable" }
            )
        })
    }
}

pub fn on_key(app: &mut App, key: KeyEvent) {
    let state = &mut app.connection_test;
    match key.code {
        KeyCode::Enter => start_test(state),
        KeyCode::Backspace => state.input.backspace(),
        KeyCode::Char(c) => state.input.push(c),
        _ => {}
    }
}

fn start_test(state: &mut State) {
    let target = state.input.value.trim().to_string();
    if target.is_empty() {
        return;
    }
    {
        let mut shared = state.shared.lock().unwrap();
        if shared.running {
            return;
        }
        shared.running = true;
    }

    let shared = state.shared.clone();
    tokio::spawn(async move {
        let result = ping::ping_host(&target).await;
        let reachable = result.replies.iter().any(|r| r.success);

        let mut shared = shared.lock().unwrap();
        shared.running = false;
        shared.history.insert(
            0,
            HistoryEntry {
                target: result.target.clone(),
                timestamp: Instant::now(),
                reachable,
            },
        );
        shared.history.truncate(10);
        shared.result = Some(result);
    });
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let state = &app.connection_test;
    let shared = state.shared.lock().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(8),
        ])
        .split(area);

    let input_title = if shared.running {
        "Host or IP (testing…)"
    } else {
        "Host or IP"
    };
    let input = Paragraph::new(format!("{}_", state.input.value))
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, chunks[0]);

    let result_lines = render_result(&shared);
    let result = Paragraph::new(result_lines)
        .block(Block::default().borders(Borders::ALL).title("Result"));
    frame.render_widget(result, chunks[1]);

    let items: Vec<ListItem> = shared
        .history
        .iter()
        .map(|h| {
            let color = if h.reachable { Color::Green } else { Color::Red };
            ListItem::new(Line::from(vec![
                Span::styled(
                    if h.reachable { "● " } else { "○ " },
                    Style::default().fg(color),
                ),
                Span::raw(format!("{}  ", h.target)),
                Span::styled(format_ago(h.timestamp), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();
    let history = List::new(items).block(Block::default().borders(Borders::ALL).title("History"));
    frame.render_widget(history, chunks[2]);
}

fn render_result(shared: &Shared) -> Vec<Line<'static>> {
    if shared.running {
        return vec![Line::from("Testing…")];
    }
    let Some(result) = &shared.result else {
        return vec![Line::from("Type a hostname or IP above and press Enter.")];
    };

    let mut lines = Vec::new();
    match (result.resolved_address, &result.resolve_error) {
        (Some(addr), _) => lines.push(Line::from(format!("Resolved: {addr}"))),
        (None, Some(err)) => {
            lines.push(Line::from(Span::styled(
                format!("Could not resolve host: {err}"),
                Style::default().fg(Color::Red),
            )));
            return lines;
        }
        (None, None) => {}
    }

    let successes = result.replies.iter().filter(|r| r.success).count();
    let total = result.replies.len();
    let summary_color = if successes > 0 { Color::Green } else { Color::Red };
    lines.push(Line::from(Span::styled(
        format!(
            "{}/{} replies received",
            successes, total
        ),
        Style::default().fg(summary_color).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, reply) in result.replies.iter().enumerate() {
        let text = if reply.success {
            format!(
                "  #{} reply: time={}",
                i + 1,
                reply.roundtrip.map(format_ms).unwrap_or_default()
            )
        } else {
            format!("  #{} failed: {}", i + 1, reply.status)
        };
        let color = if reply.success { Color::Green } else { Color::Red };
        lines.push(Line::from(Span::styled(text, Style::default().fg(color))));
    }

    lines
}
