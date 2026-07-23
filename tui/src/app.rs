use crossterm::event::KeyEvent;

use crate::monitor::MonitorEngine;
use crate::screens::{connection_test, dashboard, dns_monitor, dns_servers};
use crate::update::UpdateState;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Dashboard,
    ConnectionTest,
    DnsServers,
    DnsMonitor,
}

pub const MENU_ITEMS: [(Screen, &str); 4] = [
    (Screen::Dashboard, "Dashboard"),
    (Screen::ConnectionTest, "Connection Test"),
    (Screen::DnsServers, "DNS Servers"),
    (Screen::DnsMonitor, "DNS Monitor"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Menu,
    Content,
}

pub struct App {
    pub screen: Screen,
    pub menu_index: usize,
    pub focus: Focus,
    pub should_quit: bool,
    pub restart_requested: bool,
    pub monitor_engine: MonitorEngine,
    pub connection_test: connection_test::State,
    pub dns_servers: dns_servers::State,
    pub dns_monitor: dns_monitor::State,
    pub update: UpdateState,
}

impl App {
    pub fn new() -> Self {
        let monitor_engine = MonitorEngine::new();
        Self {
            screen: Screen::Dashboard,
            menu_index: 0,
            focus: Focus::Menu,
            should_quit: false,
            restart_requested: false,
            monitor_engine,
            connection_test: connection_test::State::default(),
            dns_servers: dns_servers::State::new(),
            dns_monitor: dns_monitor::State::default(),
            update: UpdateState::new(),
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.focus {
            Focus::Menu => self.on_menu_key(key),
            Focus::Content => self.on_content_key(key),
        }
    }

    fn on_menu_key(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('r') if self.update.installed_version().is_some() => {
                self.restart_requested = true;
                self.should_quit = true;
            }
            KeyCode::Char('u') if !self.update.checking() && !self.update.installing() => {
                if self.update.available().is_some() {
                    self.update.start_install();
                } else {
                    self.update.check_now();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.menu_index = self.menu_index.checked_sub(1).unwrap_or(MENU_ITEMS.len() - 1);
                self.screen = MENU_ITEMS[self.menu_index].0;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.menu_index = (self.menu_index + 1) % MENU_ITEMS.len();
                self.screen = MENU_ITEMS[self.menu_index].0;
            }
            KeyCode::Enter | KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') => {
                if self.screen != Screen::Dashboard {
                    self.focus = Focus::Content;
                }
            }
            _ => {}
        }
    }

    fn on_content_key(&mut self, key: KeyEvent) {
        use crossterm::event::KeyCode;
        if key.code == KeyCode::Esc {
            self.focus = Focus::Menu;
            return;
        }
        match self.screen {
            Screen::Dashboard => {}
            Screen::ConnectionTest => connection_test::on_key(self, key),
            Screen::DnsServers => dns_servers::on_key(self, key),
            Screen::DnsMonitor => dns_monitor::on_key(self, key),
        }
    }

    pub fn status_hint(&self) -> String {
        if self.focus == Focus::Menu {
            if let Some(banner) = self.update_banner() {
                return banner;
            }
        }
        match self.focus {
            Focus::Menu => {
                "↑/↓ or j/k: move   enter/tab: open   u: check for updates   q: quit".to_string()
            }
            Focus::Content => match self.screen {
                Screen::Dashboard => "esc: back".to_string(),
                Screen::ConnectionTest => connection_test::HINT.to_string(),
                Screen::DnsServers => dns_servers::HINT.to_string(),
                Screen::DnsMonitor => dns_monitor::HINT.to_string(),
            },
        }
    }

    fn update_banner(&self) -> Option<String> {
        if let Some(err) = self.update.error() {
            return Some(format!("update failed: {err}   u: retry"));
        }
        if let Some(version) = self.update.installed_version() {
            return Some(format!("updated to v{version} — r: restart now"));
        }
        if self.update.installing() {
            return Some("installing update…".to_string());
        }
        if let Some(update) = self.update.available() {
            return Some(format!(
                "update available: v{}   u: install and restart",
                update.version
            ));
        }
        if self.update.checking() {
            return Some("checking for updates…".to_string());
        }
        None
    }
}

pub fn render_dashboard_summary(app: &App) -> dashboard::Summary {
    let monitors = app.monitor_engine.list();
    dashboard::Summary {
        monitor_count: monitors.len(),
        monitors_running: monitors.iter().filter(|m| m.running).count(),
        last_connection_test: app.connection_test.last_summary(),
        dns_server_groups: app.dns_servers.groups_len(),
    }
}
