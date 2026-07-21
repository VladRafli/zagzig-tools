use std::sync::{Arc, Mutex};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::sysdns::{self, DnsServerGroup};
use crate::util::format_ago;

pub const HINT: &str = "r: refresh   esc: back to menu";

#[derive(Default)]
struct Shared {
    loading: bool,
    error: Option<String>,
    groups: Vec<DnsServerGroup>,
    last_updated: Option<Instant>,
}

pub struct State {
    shared: Arc<Mutex<Shared>>,
}

impl State {
    pub fn new() -> Self {
        let state = Self {
            shared: Arc::new(Mutex::new(Shared::default())),
        };
        state.refresh();
        state
    }

    pub fn groups_len(&self) -> usize {
        self.shared.lock().unwrap().groups.len()
    }

    pub fn refresh(&self) {
        {
            let mut shared = self.shared.lock().unwrap();
            if shared.loading {
                return;
            }
            shared.loading = true;
        }
        let shared = self.shared.clone();
        tokio::spawn(async move {
            let result = sysdns::list_dns_servers().await;
            let mut shared = shared.lock().unwrap();
            shared.loading = false;
            shared.last_updated = Some(Instant::now());
            match result {
                Ok(groups) => {
                    shared.groups = groups;
                    shared.error = None;
                }
                Err(err) => shared.error = Some(err),
            }
        });
    }
}

pub fn on_key(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('r') {
        app.dns_servers.refresh();
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let shared = app.dns_servers.shared.lock().unwrap();

    let mut lines: Vec<Line> = Vec::new();
    if shared.loading {
        lines.push(Line::from("Reading DNS configuration…"));
    }
    if let Some(err) = &shared.error {
        lines.push(Line::from(Span::styled(
            format!("Couldn't read DNS configuration: {err}"),
            Style::default().fg(Color::Red),
        )));
    }
    if !shared.loading && shared.error.is_none() && shared.groups.is_empty() {
        lines.push(Line::from("No DNS servers found."));
    }

    for group in &shared.groups {
        if !lines.is_empty() {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            group.label.clone(),
            Style::default().fg(Color::Cyan),
        )));
        if group.servers.is_empty() {
            lines.push(Line::from("  (none configured — using automatic)"));
        }
        for server in &group.servers {
            lines.push(Line::from(format!("  • {server}")));
        }
    }

    if let Some(updated) = shared.last_updated {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Updated {}", format_ago(updated)),
            Style::default().fg(Color::DarkGray),
        )));
    }

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("DNS Servers"));
    frame.render_widget(paragraph, area);
}
