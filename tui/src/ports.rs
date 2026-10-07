// Which program listens on which port. Windows reads `netstat -ano` plus
// `tasklist` for the process names; Linux reads `ss -tulnp`. The parsers take
// plain text and are not platform-gated so they can be tested anywhere.
#![allow(dead_code)]

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortEntry {
    pub protocol: &'static str,
    pub local: String,
    pub state: String,
    pub pid: Option<u32>,
    pub process: String,
}

impl PortEntry {
    pub fn port(&self) -> u32 {
        self.local.rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or(0)
    }

    pub fn row(&self) -> String {
        let pid = self.pid.map(|p| p.to_string()).unwrap_or_else(|| "-".into());
        let process = if self.process.is_empty() { "?" } else { &self.process };
        format!("{:<5} {:<28} {:<7} {:>7}  {}", self.protocol, self.local, self.state, pid, process)
    }
}

pub const HEADER: &str = "PROTO LOCAL ADDRESS                 STATE       PID  PROCESS";

fn sort(mut entries: Vec<PortEntry>) -> Vec<PortEntry> {
    entries.sort_by(|a, b| (a.port(), a.protocol, &a.local).cmp(&(b.port(), b.protocol, &b.local)));
    entries.dedup();
    entries
}

// `  TCP    0.0.0.0:135    0.0.0.0:0    LISTENING    1234` and
// `  UDP    0.0.0.0:123    *:*                       1234`.
pub fn parse_netstat(text: &str, names: &HashMap<u32, String>) -> Vec<PortEntry> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        let (protocol, state, pid) = match t.as_slice() {
            ["TCP", _, _, "LISTENING", pid] => ("TCP", "LISTEN", pid),
            ["UDP", _, _, pid] => ("UDP", "", pid),
            _ => continue,
        };
        let pid: Option<u32> = pid.parse().ok();
        out.push(PortEntry {
            protocol,
            local: t[1].to_string(),
            state: state.to_string(),
            pid,
            process: pid.and_then(|p| names.get(&p).cloned()).unwrap_or_default(),
        });
    }
    sort(out)
}

// `"svchost.exe","1234","Services","0","10,000 K"` per line.
pub fn parse_tasklist(text: &str) -> HashMap<u32, String> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.trim().trim_matches('"').split("\",\"");
            let name = f.next()?;
            let pid = f.next()?.parse().ok()?;
            Some((pid, name.to_string()))
        })
        .collect()
}

// `tcp LISTEN 0 4096 0.0.0.0:22 0.0.0.0:* users:(("sshd",pid=800,fd=3))`.
pub fn parse_ss(text: &str) -> Vec<PortEntry> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        if t.len() < 6 {
            continue;
        }
        let protocol = match t[0] {
            "tcp" => "TCP",
            "udp" => "UDP",
            _ => continue,
        };
        let users = t.get(6).copied().unwrap_or("");
        let process = users
            .split("((\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .unwrap_or("")
            .to_string();
        let pid = users
            .split("pid=")
            .nth(1)
            .and_then(|s| s.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|s| s.parse().ok());
        out.push(PortEntry {
            protocol,
            local: t[4].to_string(),
            state: if protocol == "TCP" { "LISTEN".into() } else { String::new() },
            pid,
            process,
        });
    }
    sort(out)
}

pub async fn list_ports() -> Result<Vec<PortEntry>, String> {
    tokio::task::spawn_blocking(list_ports_blocking)
        .await
        .map_err(|err| format!("failed to read ports: {err}"))?
}

#[cfg(target_os = "windows")]
fn list_ports_blocking() -> Result<Vec<PortEntry>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let run = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|err| format!("failed to run {program}: {err}"))
    };
    let netstat = run("netstat", &["-ano"])?;
    if !netstat.status.success() {
        return Err(String::from_utf8_lossy(&netstat.stderr).trim().to_string());
    }
    // Names are a nicety: without them the list still shows PIDs.
    let names = run("tasklist", &["/fo", "csv", "/nh"])
        .map(|o| parse_tasklist(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    Ok(parse_netstat(&String::from_utf8_lossy(&netstat.stdout), &names))
}

#[cfg(not(target_os = "windows"))]
fn list_ports_blocking() -> Result<Vec<PortEntry>, String> {
    let out = std::process::Command::new("ss")
        .args(["-H", "-tulnp"])
        .output()
        .map_err(|err| format!("failed to run ss (is iproute2 installed?): {err}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(parse_ss(&String::from_utf8_lossy(&out.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_netstat_and_names() {
        let names = parse_tasklist(
            "\"svchost.exe\",\"1234\",\"Services\",\"0\",\"10,000 K\"\n\"nginx.exe\",\"88\",\"Console\",\"1\",\"5 K\"\n",
        );
        let text = "Active Connections\n\n  Proto  Local Address          Foreign Address        State           PID\n  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1234\n  TCP    10.0.0.5:5000          10.0.0.9:443           ESTABLISHED     88\n  TCP    [::]:80                [::]:0                 LISTENING       88\n  UDP    0.0.0.0:123            *:*                                    1234\n";
        let e = parse_netstat(text, &names);
        assert_eq!(e.len(), 3, "established connections are left out");
        assert_eq!((e[0].protocol, e[0].port(), e[0].process.as_str()), ("TCP", 80, "nginx.exe"));
        assert_eq!(e[0].local, "[::]:80");
        assert_eq!((e[1].protocol, e[1].state.as_str()), ("UDP", ""));
        assert_eq!(e[2].pid, Some(1234));
    }

    #[test]
    fn parses_ss() {
        let text = "tcp LISTEN 0 4096 0.0.0.0:22 0.0.0.0:* users:((\"sshd\",pid=800,fd=3))\nudp UNCONN 0 0 127.0.0.53%lo:53 0.0.0.0:*\ntcp LISTEN 0 128 [::]:8080 [::]:* users:((\"node\",pid=42,fd=19),(\"node\",pid=43,fd=19))\nnetid state\n";
        let e = parse_ss(text);
        assert_eq!(e.len(), 3);
        assert_eq!((e[0].port(), e[0].process.as_str(), e[0].pid), (22, "sshd", Some(800)));
        assert_eq!((e[1].port(), e[1].process.as_str()), (53, ""));
        assert_eq!((e[2].port(), e[2].process.as_str(), e[2].pid), (8080, "node", Some(42)));
    }
}

#[cfg(test)]
mod live_tests {
    #[tokio::test]
    #[ignore = "reads this machine's real listening ports"]
    async fn lists_real_ports() {
        let entries = super::list_ports().await.unwrap();
        for e in entries.iter().take(8) {
            println!("{}", e.row());
        }
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| !e.process.is_empty()), "process names resolve");
    }
}
