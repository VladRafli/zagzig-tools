pub mod connection_test;
pub mod dashboard;
pub mod dns_monitor;
pub mod dns_servers;
pub mod hosts;
pub mod listview;
pub mod ports;

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    match app.screen {
        Screen::Dashboard => dashboard::render(frame, area, app),
        Screen::ConnectionTest => connection_test::render(frame, area, app),
        Screen::DnsServers => dns_servers::render(frame, area, app),
        Screen::DnsMonitor => dns_monitor::render(frame, area, app),
        Screen::Ports => ports::render(frame, area, app),
        Screen::Hosts => hosts::render(frame, area, app),
    }
}
