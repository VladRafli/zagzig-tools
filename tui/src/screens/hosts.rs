use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::App;
use crate::hostsfile::{self, HostEntry};
use crate::screens::listview::ListState;

pub type State = ListState<HostEntry>;

pub fn new_state() -> State {
    ListState::new(|| Box::pin(hostsfile::read()))
}

pub fn on_key(app: &mut App, key: KeyEvent) {
    app.hosts.on_key(key, &HostEntry::row);
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    app.hosts.render(
        frame,
        area,
        "Hosts File (read only)",
        hostsfile::HEADER,
        "The hosts file has no active entries.",
        &HostEntry::row,
    );
}
