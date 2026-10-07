// A read-only look at the hosts file: active entries only.
#![allow(dead_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostEntry {
    pub ip: String,
    pub names: String,
}

impl HostEntry {
    pub fn row(&self) -> String {
        format!("{:<40} {}", self.ip, self.names)
    }
}

pub const HEADER: &str = "ADDRESS                                  HOST NAMES";

pub fn path() -> &'static str {
    if cfg!(target_os = "windows") {
        r"C:\Windows\System32\drivers\etc\hosts"
    } else {
        "/etc/hosts"
    }
}

pub fn parse(text: &str) -> Vec<HostEntry> {
    text.lines()
        .filter_map(|line| {
            let body = line.split('#').next().unwrap_or("").trim();
            let mut parts = body.split_whitespace();
            let ip = parts.next()?;
            let names: Vec<&str> = parts.collect();
            if names.is_empty() {
                return None;
            }
            Some(HostEntry { ip: ip.to_string(), names: names.join(" ") })
        })
        .collect()
}

pub async fn read() -> Result<Vec<HostEntry>, String> {
    let p = path();
    tokio::task::spawn_blocking(move || {
        std::fs::read_to_string(p)
            .map(|t| parse(&t))
            .map_err(|err| format!("couldn't read {p}: {err}"))
    })
    .await
    .map_err(|err| format!("failed to read the hosts file: {err}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_comments_and_blank_lines() {
        let e = parse("# header\n\n127.0.0.1 localhost   # loop\n  10.0.0.5\tdb.local  db\n192.168.1.1\n#10.0.0.9 off.local\n");
        assert_eq!(e.len(), 2);
        assert_eq!(e[0], HostEntry { ip: "127.0.0.1".into(), names: "localhost".into() });
        assert_eq!(e[1].names, "db.local db");
    }
}
