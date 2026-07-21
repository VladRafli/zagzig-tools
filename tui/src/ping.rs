use std::io;
use std::net::IpAddr;
use std::time::Duration;

#[derive(Clone)]
pub struct PingReply {
    pub success: bool,
    pub status: String,
    pub roundtrip: Option<Duration>,
}

#[derive(Clone)]
pub struct PingResult {
    pub target: String,
    pub resolved_address: Option<IpAddr>,
    pub resolve_error: Option<String>,
    pub replies: Vec<PingReply>,
}

const ECHO_COUNT: usize = 4;
const PAYLOAD: &[u8] = b"zagzig-tui-ping";

// Resolves `target` (accepting either a literal IP or a hostname) and sends a
// handful of ICMP echo requests to it. `surge-ping` only decodes Echo Reply
// packets (no Time-Exceeded support), so this covers reachability only —
// there's no traceroute here, see tui/README for why.
pub async fn ping_host(target: &str) -> PingResult {
    let (address, resolve_error) = match target.parse::<IpAddr>() {
        Ok(ip) => (Some(ip), None),
        Err(_) => {
            let outcome = crate::dns::resolve_hostname(target, None).await;
            // Prefer IPv4: plenty of networks (this one included) resolve AAAA
            // records without having a working IPv6 route, which would
            // otherwise make every ping fail with ENETUNREACH by bad luck of
            // address ordering.
            let preferred = outcome
                .addresses
                .iter()
                .find(|ip| ip.is_ipv4())
                .or_else(|| outcome.addresses.first())
                .copied();
            match preferred {
                Some(ip) => (Some(ip), None),
                None => (
                    None,
                    Some(outcome.error.unwrap_or_else(|| "could not resolve host".into())),
                ),
            }
        }
    };

    let Some(address) = address else {
        return PingResult {
            target: target.to_string(),
            resolved_address: None,
            resolve_error,
            replies: Vec::new(),
        };
    };

    let mut replies = Vec::with_capacity(ECHO_COUNT);
    for _ in 0..ECHO_COUNT {
        replies.push(send_one(address).await);
    }

    PingResult {
        target: target.to_string(),
        resolved_address: Some(address),
        resolve_error: None,
        replies,
    }
}

async fn send_one(address: IpAddr) -> PingReply {
    match surge_ping::ping(address, PAYLOAD).await {
        Ok((_packet, duration)) => PingReply {
            success: true,
            status: "Success".to_string(),
            roundtrip: Some(duration),
        },
        Err(surge_ping::SurgeError::Timeout { .. }) => PingReply {
            success: false,
            status: "Request timed out".to_string(),
            roundtrip: None,
        },
        Err(surge_ping::SurgeError::IOError(err)) if err.kind() == io::ErrorKind::PermissionDenied => {
            PingReply {
                success: false,
                status: permission_hint(),
                roundtrip: None,
            }
        }
        Err(err) => PingReply {
            success: false,
            status: err.to_string(),
            roundtrip: None,
        },
    }
}

#[cfg(target_os = "windows")]
fn permission_hint() -> String {
    "Permission denied — raw ICMP sockets need this terminal run as Administrator".to_string()
}

#[cfg(not(target_os = "windows"))]
fn permission_hint() -> String {
    "Permission denied — raw ICMP sockets need root, CAP_NET_RAW, or \
     `sudo sysctl -w net.ipv4.ping_group_range=\"0 2147483647\"`"
        .to_string()
}
