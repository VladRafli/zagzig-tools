use std::net::IpAddr;
use std::time::{Duration, Instant};

use hickory_resolver::config::{NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::TokioResolver;

pub struct ResolveOutcome {
    pub resolved: bool,
    pub addresses: Vec<IpAddr>,
    pub error: Option<String>,
    pub query_time: Duration,
}

// Pure-Rust DNS client (hickory-resolver): implements the wire protocol itself
// instead of calling into the OS resolver, so this is one code path on both
// Windows and Linux and can target an arbitrary nameserver by IP.
pub async fn resolve_hostname(hostname: &str, server: Option<IpAddr>) -> ResolveOutcome {
    let started = Instant::now();

    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(5);

    let builder = match server {
        Some(ip) => Ok(TokioResolver::builder_with_config(
            ResolverConfig::from_parts(None, Vec::new(), vec![NameServerConfig::udp_and_tcp(ip)]),
            TokioRuntimeProvider::default(),
        )),
        // No explicit server: use the system's configured resolver
        // (/etc/resolv.conf on Unix, the registry on Windows).
        None => TokioResolver::builder_tokio(),
    };

    let resolver = match builder.map(|b| b.with_options(opts).build()) {
        Ok(Ok(resolver)) => resolver,
        Ok(Err(err)) | Err(err) => {
            return ResolveOutcome {
                resolved: false,
                addresses: Vec::new(),
                error: Some(err.to_string()),
                query_time: started.elapsed(),
            };
        }
    };

    // Force an exact, single-query lookup of the literal name (no search-domain
    // suffixing), matching what the GUI's Resolve-DnsName-based check does.
    let fqdn = if hostname.ends_with('.') {
        hostname.to_string()
    } else {
        format!("{hostname}.")
    };

    match resolver.lookup_ip(fqdn).await {
        Ok(lookup) => {
            let addresses: Vec<IpAddr> = lookup.iter().collect();
            ResolveOutcome {
                resolved: !addresses.is_empty(),
                addresses,
                error: None,
                query_time: started.elapsed(),
            }
        }
        Err(err) => ResolveOutcome {
            resolved: false,
            addresses: Vec::new(),
            error: Some(err.to_string()),
            query_time: started.elapsed(),
        },
    }
}

#[cfg(test)]
mod live_tests {
    use super::*;

    // Not run in CI (needs real network) — a manual real-environment check
    // that the pure-Rust resolver path actually resolves something on this
    // OS, run with: cargo test -- --ignored resolves_a_real_hostname
    #[tokio::test]
    #[ignore]
    async fn resolves_a_real_hostname() {
        let outcome = resolve_hostname("one.one.one.one", None).await;
        assert!(outcome.resolved, "expected a resolved address, got: {:?}", outcome.error);
        assert!(!outcome.addresses.is_empty());
    }
}
