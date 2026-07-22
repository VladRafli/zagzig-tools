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
    // On any systemd-resolved system (the default on most modern
    // Debian/Ubuntu/Fedora installs), /etc/resolv.conf only points at the
    // local 127.0.0.53 stub resolver — technically correct, useless to show
    // someone asking "what DNS server is configured". `resolvectl status`
    // reports the actual upstream servers (and per-link overrides, mirroring
    // Windows' per-adapter view), so it's tried first; falling back to
    // /etc/resolv.conf covers systems without systemd-resolved at all
    // (NetworkManager-only setups, Alpine, a hand-written static file, ...).
    if let Some(groups) = list_dns_servers_via_resolvectl() {
        return Ok(groups);
    }

    let text = std::fs::read_to_string("/etc/resolv.conf")
        .map_err(|err| format!("failed to read /etc/resolv.conf: {err}"))?;

    let servers: Vec<String> = text
        .lines()
        .filter_map(|line| line.trim().strip_prefix("nameserver"))
        .map(|rest| rest.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(vec![DnsServerGroup {
        label: "/etc/resolv.conf".to_string(),
        servers,
    }])
}

#[cfg(not(target_os = "windows"))]
fn list_dns_servers_via_resolvectl() -> Option<Vec<DnsServerGroup>> {
    let output = std::process::Command::new("resolvectl")
        .arg("status")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let groups = parse_resolvectl_status(&String::from_utf8_lossy(&output.stdout));
    if groups.is_empty() {
        None
    } else {
        Some(groups)
    }
}

// `resolvectl status` right-aligns "Key: value" pairs under section headers
// ("Global", "Link 2 (eth0)", ...) that start at column 0. Sections with no
// DNS servers of their own (a link with no per-interface override) are
// dropped, same as ipconfig groups with no DNS Servers line. The exact-match
// on "DNS Servers" matters: "Current DNS Server" and "Fallback DNS Servers"
// are different fields that would otherwise get folded in by a substring
// match.
#[cfg(not(target_os = "windows"))]
fn parse_resolvectl_status(text: &str) -> Vec<DnsServerGroup> {
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

        if !line.starts_with(' ') && !line.starts_with('\t') {
            flush(&mut groups, &mut current_label, &mut current_servers);
            current_label = Some(line.trim().to_string());
            in_dns_servers = false;
            continue;
        }

        let trimmed = line.trim();
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            in_dns_servers = key == "DNS Servers";
            if in_dns_servers && !value.is_empty() {
                current_servers.extend(value.split_whitespace().map(str::to_string));
            }
        } else if in_dns_servers && !trimmed.is_empty() {
            current_servers.extend(trimmed.split_whitespace().map(str::to_string));
        }
    }
    flush(&mut groups, &mut current_label, &mut current_servers);

    groups
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

#[cfg(all(test, not(target_os = "windows")))]
mod resolvectl_tests {
    use super::*;

    // Captured verbatim from `resolvectl status` on a real Ubuntu (WSL2,
    // systemd-resolved) — this is the exact case that motivated preferring
    // resolvectl over /etc/resolv.conf: Link 2/3 have no DNS servers of
    // their own and must be dropped, "Current DNS Server" and "Fallback DNS
    // Servers" must not be mistaken for "DNS Servers", and the real servers
    // live only under "Global".
    #[test]
    fn parses_global_servers_and_drops_linkless_sections() {
        let sample = "\
Global
           Protocols: -LLMNR -mDNS -DNSOverTLS DNSSEC=no/unsupported
    resolv.conf mode: stub
  Current DNS Server: 192.192.10.52
         DNS Servers: 192.192.10.52 192.192.10.53
Fallback DNS Servers: 9.9.9.9 1.1.1.1 8.8.8.8

Link 2 (enP62741p0s0)
    Current Scopes: none
         Protocols: -DefaultRoute -LLMNR -mDNS -DNSOverTLS DNSSEC=no/unsupported
     Default Route: no

Link 3 (loopback0)
    Current Scopes: none
         Protocols: -DefaultRoute -LLMNR -mDNS -DNSOverTLS DNSSEC=no/unsupported
     Default Route: no
";
        let groups = parse_resolvectl_status(sample);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].label, "Global");
        assert_eq!(groups[0].servers, vec!["192.192.10.52", "192.192.10.53"]);
    }

    #[test]
    fn parses_per_link_override() {
        let sample = "\
Global
         DNS Servers: 1.1.1.1

Link 5 (eth0)
    Current Scopes: DNS
         Protocols: +DefaultRoute
         DNS Servers: 10.0.0.1 10.0.0.2
";
        let groups = parse_resolvectl_status(sample);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].label, "Global");
        assert_eq!(groups[0].servers, vec!["1.1.1.1"]);
        assert_eq!(groups[1].label, "Link 5 (eth0)");
        assert_eq!(groups[1].servers, vec!["10.0.0.1", "10.0.0.2"]);
    }
}
