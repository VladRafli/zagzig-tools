use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::dns;

const MAX_ENTRIES: usize = 50;

#[derive(Clone)]
pub struct MonitorEntry {
    pub timestamp: Instant,
    pub resolved: bool,
    pub addresses: Vec<IpAddr>,
    pub error: Option<String>,
    pub query_time: Duration,
}

#[derive(Clone)]
pub struct Monitor {
    pub id: Uuid,
    pub hostname: String,
    pub server: Option<IpAddr>,
    pub server_label: String,
    pub interval: Duration,
    pub running: bool,
    pub checking: bool,
    pub entries: Vec<MonitorEntry>,
}

// Mirrors the GUI's module-level dns-monitor-store.ts: a background engine
// that outlives whichever screen is currently drawn, so switching screens
// never pauses a running monitor. `std::sync::Mutex` (not tokio's) is used
// deliberately — the ratatui render closure is synchronous and can't .await,
// so state has to be lockable from a plain sync context; every task here
// only holds the lock across short, non-blocking critical sections.
#[derive(Clone)]
pub struct MonitorEngine {
    monitors: Arc<Mutex<Vec<Monitor>>>,
    tasks: Arc<Mutex<HashMap<Uuid, JoinHandle<()>>>>,
}

impl MonitorEngine {
    pub fn new() -> Self {
        Self {
            monitors: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn list(&self) -> Vec<Monitor> {
        self.monitors.lock().unwrap().clone()
    }

    pub fn add(
        &self,
        hostname: String,
        server: Option<IpAddr>,
        server_label: String,
        interval: Duration,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let monitor = Monitor {
            id,
            hostname,
            server,
            server_label,
            interval,
            running: false,
            checking: false,
            entries: Vec::new(),
        };
        self.monitors.lock().unwrap().insert(0, monitor);
        self.start(id);
        id
    }

    pub fn remove(&self, id: Uuid) {
        self.stop(id);
        self.monitors.lock().unwrap().retain(|m| m.id != id);
    }

    pub fn clear_entries(&self, id: Uuid) {
        if let Some(m) = self.monitors.lock().unwrap().iter_mut().find(|m| m.id == id) {
            m.entries.clear();
        }
    }

    pub fn start(&self, id: Uuid) {
        {
            let mut monitors = self.monitors.lock().unwrap();
            let Some(monitor) = monitors.iter_mut().find(|m| m.id == id) else {
                return;
            };
            if monitor.running {
                return;
            }
            monitor.running = true;
        }

        let engine = self.clone();
        let handle = tokio::spawn(async move { engine.run_loop(id).await });
        self.tasks.lock().unwrap().insert(id, handle);
    }

    pub fn stop(&self, id: Uuid) {
        if let Some(m) = self.monitors.lock().unwrap().iter_mut().find(|m| m.id == id) {
            m.running = false;
            m.checking = false;
        }
        if let Some(handle) = self.tasks.lock().unwrap().remove(&id) {
            handle.abort();
        }
    }

    // Self-rescheduling check-then-sleep loop (same shape as the GUI's
    // setTimeout chain): the next sleep only starts after the previous check
    // finishes, so a slow lookup can never cause overlapping checks.
    async fn run_loop(&self, id: Uuid) {
        loop {
            let (hostname, server) = {
                let monitors = self.monitors.lock().unwrap();
                match monitors.iter().find(|m| m.id == id) {
                    Some(m) if m.running => (m.hostname.clone(), m.server),
                    _ => return,
                }
            };

            {
                let mut monitors = self.monitors.lock().unwrap();
                if let Some(m) = monitors.iter_mut().find(|m| m.id == id) {
                    m.checking = true;
                } else {
                    return;
                }
            }

            let outcome = dns::resolve_hostname(&hostname, server).await;

            let interval = {
                let mut monitors = self.monitors.lock().unwrap();
                let Some(m) = monitors.iter_mut().find(|m| m.id == id) else {
                    return;
                };
                m.checking = false;
                m.entries.insert(
                    0,
                    MonitorEntry {
                        timestamp: Instant::now(),
                        resolved: outcome.resolved,
                        addresses: outcome.addresses,
                        error: outcome.error,
                        query_time: outcome.query_time,
                    },
                );
                m.entries.truncate(MAX_ENTRIES);
                if !m.running {
                    return;
                }
                m.interval
            };

            tokio::time::sleep(interval).await;
        }
    }
}
