// A scrollable, filterable, refreshable list shared by the Ports and Hosts
// screens: the loading, error and "updated Ns ago" handling lives here once.
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::util::{format_ago, TextInput};

pub const HINT: &str = "↑/↓ or j/k: scroll   /: filter   c: clear filter   r: refresh   esc: back to menu";
const FILTER_HINT: &str = "type to filter   backspace: delete   enter: done   esc: back to menu";

type Fetch<T> = fn() -> Pin<Box<dyn Future<Output = Result<Vec<T>, String>> + Send>>;

struct Shared<T> {
    loading: bool,
    error: Option<String>,
    items: Vec<T>,
    last_updated: Option<Instant>,
}

pub struct ListState<T> {
    shared: Arc<Mutex<Shared<T>>>,
    fetch: Fetch<T>,
    selected: usize,
    filter: TextInput,
    filtering: bool,
}

impl<T: Clone + Send + 'static> ListState<T> {
    pub fn new(fetch: Fetch<T>) -> Self {
        let state = Self {
            shared: Arc::new(Mutex::new(Shared {
                loading: false,
                error: None,
                items: Vec::new(),
                last_updated: None,
            })),
            fetch,
            selected: 0,
            filter: TextInput::default(),
            filtering: false,
        };
        state.refresh();
        state
    }

    pub fn len(&self) -> usize {
        self.shared.lock().unwrap().items.len()
    }

    pub fn hint(&self) -> &'static str {
        if self.filtering {
            FILTER_HINT
        } else {
            HINT
        }
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
        let fut = (self.fetch)();
        tokio::spawn(async move {
            let result = fut.await;
            let mut shared = shared.lock().unwrap();
            shared.loading = false;
            shared.last_updated = Some(Instant::now());
            match result {
                Ok(items) => {
                    shared.items = items;
                    shared.error = None;
                }
                Err(err) => shared.error = Some(err),
            }
        });
    }

    fn visible(&self, row: &dyn Fn(&T) -> String) -> Vec<String> {
        let needle = self.filter.value.trim().to_lowercase();
        self.shared
            .lock()
            .unwrap()
            .items
            .iter()
            .map(row)
            .filter(|r| needle.is_empty() || r.to_lowercase().contains(&needle))
            .collect()
    }

    pub fn on_key(&mut self, key: KeyEvent, row: &dyn Fn(&T) -> String) {
        if self.filtering {
            match key.code {
                KeyCode::Enter => self.filtering = false,
                KeyCode::Backspace => {
                    self.filter.backspace();
                    self.selected = 0;
                }
                KeyCode::Char(c) => {
                    self.filter.push(c);
                    self.selected = 0;
                }
                _ => {}
            }
            return;
        }
        let last = self.visible(row).len().saturating_sub(1);
        match key.code {
            KeyCode::Char('/') => self.filtering = true,
            KeyCode::Char('c') => {
                self.filter.clear();
                self.selected = 0;
            }
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Down | KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::PageDown => self.selected = (self.selected + 10).min(last),
            KeyCode::PageUp => self.selected = self.selected.saturating_sub(10),
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = last,
            _ => {}
        }
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        header: &str,
        empty: &str,
        row: &dyn Fn(&T) -> String,
    ) {
        let rows = self.visible(row);
        let selected = self.selected.min(rows.len().saturating_sub(1));
        let shared = self.shared.lock().unwrap();

        let mut lines: Vec<Line> = Vec::new();
        let filter_text = if self.filtering {
            format!("filter: {}▏", self.filter.value)
        } else if self.filter.value.is_empty() {
            String::new()
        } else {
            format!("filter: {}  (c clears)", self.filter.value)
        };
        let status = if shared.loading {
            "reading…".to_string()
        } else if let Some(updated) = shared.last_updated {
            format!("{} of {}   updated {}", rows.len(), shared.items.len(), format_ago(updated))
        } else {
            String::new()
        };
        lines.push(Line::from(Span::styled(
            format!("{status}   {filter_text}"),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(Span::styled(
            header.to_string(),
            Style::default().fg(Color::Cyan),
        )));
        if let Some(err) = &shared.error {
            lines.push(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(Color::Red),
            )));
        }
        if rows.is_empty() && !shared.loading && shared.error.is_none() {
            lines.push(Line::from(if shared.items.is_empty() { empty } else { "Nothing matches the filter." }));
        }

        let used = lines.len();
        let height = (area.height as usize).saturating_sub(2 + used).max(1);
        let offset = (selected + 1).saturating_sub(height);
        for (i, text) in rows.iter().enumerate().skip(offset).take(height) {
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(text.clone(), style)));
        }

        let paragraph = Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title.to_string()));
        frame.render_widget(paragraph, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent { code, modifiers: KeyModifiers::NONE, kind: KeyEventKind::Press, state: KeyEventState::NONE }
    }

    fn screen(state: &ListState<String>) -> String {
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
        terminal
            .draw(|f| state.render(f, f.area(), "Test", "HEADER", "nothing here", &|s: &String| s.clone()))
            .unwrap();
        terminal.backend().to_string()
    }

    #[tokio::test]
    async fn loads_filters_and_scrolls() {
        let mut state: ListState<String> = ListState::new(|| {
            Box::pin(async { Ok((1..=30).map(|n| format!("row {n}")).collect()) })
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let row = |s: &String| s.clone();

        let text = screen(&state);
        assert!(text.contains("row 1") && text.contains("30 of 30") && !text.contains("row 30"));

        for _ in 0..29 {
            state.on_key(key(KeyCode::Down), &row);
        }
        assert!(screen(&state).contains("row 30"), "scrolls to keep the selection visible");

        state.on_key(key(KeyCode::Char('/')), &row);
        for c in "row 2".chars() {
            state.on_key(key(KeyCode::Char(c)), &row);
        }
        state.on_key(key(KeyCode::Enter), &row);
        let text = screen(&state);
        assert!(text.contains("11 of 30") && text.contains("row 20") && !text.contains("row 3 "), "{text}");

        state.on_key(key(KeyCode::Char('c')), &row);
        assert!(screen(&state).contains("30 of 30"));
    }

    #[tokio::test]
    async fn shows_errors_and_empty_lists() {
        let state: ListState<String> = ListState::new(|| Box::pin(async { Err("boom".to_string()) }));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(screen(&state).contains("boom"));
        let state: ListState<String> = ListState::new(|| Box::pin(async { Ok(vec![]) }));
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(screen(&state).contains("nothing here"));
    }
}
