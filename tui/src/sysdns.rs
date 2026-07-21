pub struct DnsServerGroup {
    pub label: String,
    pub servers: Vec<String>,
}

pub async fn list_dns_servers() -> Result<Vec<DnsServerGroup>, String> {
    tokio::task::spawn_blocking(list_dns_servers_blocking)
        .await
        .map_err(|err| format!("failed to read DNS configuration: {err}"))?
}

#[cfg(target_os = "windows")]
fn list_dns_servers_blocking() -> Result<Vec<DnsServerGroup>, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let output = Command::new("ipconfig")
        .arg("/all")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|err| format!("failed to run ipconfig: {err}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ipconfig(&text))
}

// `ipconfig /all` groups settings under an adapter header line, then indents
// `Key . . . : Value` pairs beneath it. DNS Servers can also continue on the
// following indented line(s) with no key at all, so a blank-keyed line that
// still parses as an address is folded into the previous "DNS Servers" entry.
#[cfg(target_os = "windows")]
fn parse_ipconfig(text: &str) -> Vec<DnsServerGroup> {
    let mut groups = Vec::new();
    let mut current_label: Option<String> = None;
    let mut current_servers: Vec<String> = Vec::new();
    let mut in_dns_servers = false;

    fn flush(
        groups: &mut Vec<DnsServerGroup>,
        label: &mut Option<String>,
        servers: &mut Vec<String>,
    ) {
        if let Some(label) = label.take() {
            if !servers.is_empty() {
                groups.push(DnsServerGroup {
                    label,
                    servers: std::mem::take(servers),
                });
            }
        }
        servers.clear();
    }

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            continue;
        }

        // Adapter headers are not indented; everything else is.
        if !line.starts_with(' ') && !line.starts_with('\t') {
            flush(&mut groups, &mut current_label, &mut current_servers);
            current_label = Some(line.trim_end_matches(':').to_string());
            in_dns_servers = false;
            continue;
        }

        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim_matches(|c: char| c == '.' || c.is_whitespace());
            let value = value.trim();
            in_dns_servers = key.eq_ignore_ascii_case("DNS Servers");
            if in_dns_servers && !value.is_empty() {
                current_servers.push(value.to_string());
            }
        } else if in_dns_servers && !trimmed.is_empty() {
            current_servers.push(trimmed.to_string());
        }
    }
    flush(&mut groups, &mut current_label, &mut current_servers);

    groups
}

#[cfg(not(target_os = "windows"))]
fn list_dns_servers_blocking() -> Result<Vec<DnsServerGroup>, String> {
    let text = std::fs::read_to_string("/etc/resolv.conf")
        .map_err(|err| format!("failed to read /etc/resolv.conf: {err}"))?;

    let servers: Vec<String> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("nameserver"))
        .map(|rest| rest.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Unlike Windows, there's no single cross-desktop-environment API for
    // per-interface resolvers (NetworkManager, systemd-resolved, netplan all
    // differ) — /etc/resolv.conf's global list is the pragmatic common ground.
    Ok(vec![DnsServerGroup {
        label: "/etc/resolv.conf".to_string(),
        servers,
    }])
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;

    #[test]
    fn parses_dns_servers_across_continuation_lines() {
        let sample = "\
Windows IP Configuration

Ethernet adapter Ethernet:

   Connection-specific DNS Suffix  . :
   DNS Servers . . . . . . . . . . . : 1.1.1.1
                                       8.8.8.8
   NetBIOS over Tcpip. . . . . . . . : Enabled
";
        let groups = parse_ipconfig(sample);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label, "Ethernet adapter Ethernet");
        assert_eq!(groups[0].servers, vec!["1.1.1.1", "8.8.8.8"]);
    }
}
