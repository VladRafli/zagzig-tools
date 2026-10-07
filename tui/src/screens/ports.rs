use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::App;
use crate::ports::{self, PortEntry};
use crate::screens::listview::ListState;

pub type State = ListState<PortEntry>;

pub fn new_state() -> State {
    ListState::new(|| Box::pin(ports::list_ports()))
}

pub fn on_key(app: &mut App, key: KeyEvent) {
    app.ports.on_key(key, &PortEntry::row);
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    app.ports.render(
        frame,
        area,
        "Ports (listening)",
        ports::HEADER,
        "No listening ports found.",
        &PortEntry::row,
    );
}
