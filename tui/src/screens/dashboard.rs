use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub struct Summary {
    pub monitor_count: usize,
    pub monitors_running: usize,
    pub last_connection_test: Option<String>,
    pub dns_server_groups: usize,
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let summary = crate::app::render_dashboard_summary(app);

    let mut lines = vec![
        Line::from(Span::styled(
            "zagzig-tui",
            Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from("Cross-platform DNS diagnostics: Connection Test, DNS Servers, DNS Monitor."),
        Line::from(""),
        Line::from(format!(
            "DNS Monitor    {} running / {} total",
            summary.monitors_running, summary.monitor_count
        )),
        Line::from(format!(
            "DNS Servers    {} group(s) last read",
            summary.dns_server_groups
        )),
    ];
    if let Some(last) = summary.last_connection_test {
        lines.push(Line::from(format!("Connection Test    last: {last}")));
    } else {
        lines.push(Line::from("Connection Test    no tests run yet"));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "↓/j to move, enter/tab to open a section.",
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Dashboard"));
    frame.render_widget(paragraph, area);
}
