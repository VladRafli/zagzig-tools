// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            nrpt::get_nrpt_rules,
            nrpt::remove_nrpt_rule,
            nrpt::add_nrpt_rule,
            user::get_current_user,
            connection::ping_host,
            connection::traceroute_host,
            routing::get_routes,
            routing::add_route,
            routing::remove_route,
            dns::get_dns_settings,
            dns::set_dns_servers,
            dns::reset_dns_servers,
            dns::resolve_hostname,
            system::is_administrator,
            system::relaunch_as_administrator,
            signtool::find_signtool,
            signtool::list_code_signing_certificates,
            signtool::sign_file,
            signtool::verify_file,
            hosts::get_hosts_entries,
            hosts::add_hosts_entry,
            hosts::remove_hosts_entry,
            hosts::set_hosts_entry_enabled,
            hosts::set_hosts_raw,
            hosts::list_hosts_backups,
            hosts::create_hosts_backup,
            hosts::read_hosts_backup,
            hosts::restore_hosts_backup,
            hosts::delete_hosts_backup,
            hosts::move_hosts_entry,
            ssh::get_ssh_hosts,
            ssh::add_ssh_host,
            ssh::update_ssh_host,
            ssh::remove_ssh_host,
            ssh::set_ssh_config_raw,
            ports::get_port_usage,
            portctl::get_excluded_port_ranges,
            portctl::stop_process,
            portctl::stop_service,
            portproxy::get_portproxy_rules,
            portproxy::add_portproxy_rule,
            portproxy::remove_portproxy_rule,
            adapters::get_network_adapters,
            adapters::set_adapter_enabled,
            adapters::renew_adapter_dhcp,
            dnslookup::dns_lookup,
            firewall::get_firewall,
            firewall::set_firewall_rule_enabled,
            firewall::create_firewall_rule,
            firewall::delete_firewall_rule,
            neighbors::get_neighbors,
            neighbors::remove_neighbor,
            neighbors::clear_neighbors,
            services::get_services,
            services::service_action,
            eventlog::get_event_log,
            vpn::get_vpn_connections,
            vpn::vpn_action,
            wifi::get_wifi_profiles,
            wifi::reveal_wifi_key,
            languages::list_language_packs,
            languages::import_language_pack,
            languages::remove_language_pack,
            languages::save_language_template,
            languages::reveal_languages_folder,
            envvars::get_env_variables,
            envvars::set_env_variable,
            envvars::delete_env_variable,
            envvars::get_env_history,
            envvars::undo_env_change,
            envvars::check_path_entries,
            wol::get_wol_devices,
            wol::save_wol_device,
            wol::delete_wol_device,
            wol::send_wol,
            portscan::scan_ports,
            portscan::cancel_port_scan,
            startup::get_startup_items,
            startup::set_startup_item_enabled,
            startup::reveal_in_explorer,
            tls::inspect_tls,
            diagnostics::build_diagnostic_report,
            diagnostics::save_text_report,
            snapshots::take_snapshot,
            snapshots::current_snapshot,
            snapshots::list_snapshots,
            snapshots::get_snapshot,
            snapshots::rename_snapshot,
            snapshots::delete_snapshot,
            wsl::get_wsl_status,
            wsl::wsl_terminate_distro,
            wsl::wsl_set_default_distro,
            wsl::wsl_shutdown,
            wsl::wsl_hosts_status,
            wsl::wsl_hosts_sync,
            wsl::wsl_hosts_remove,
            wsl::wsl_hosts_autosync,
            wsl::wsl_force_restart,
            wsl::restart_docker_desktop,
            wsl::set_wsl_settings,
            wsl::set_wsl_config_raw,
            proxy::get_winhttp_proxy,
            proxy::set_winhttp_proxy,
            proxy::reset_winhttp_proxy,
            proxy::import_winhttp_proxy_from_system,
            certificates::get_certificates,
            certificates::delete_certificate,
            certificates::export_certificate,
            certificates::inspect_certificate_file,
            certificates::import_certificate,
            dns_cache::get_dns_cache,
            dns_cache::flush_dns_cache
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Avoids a flashing console window when spawning powershell.exe.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// The `Cert:` drive (used by every script touching a certificate store) is
// only auto-mounted in interactive sessions, or once something else in the
// process has already touched it — a fresh `-NoProfile -NonInteractive`
// process, exactly what `run_powershell` spawns, hits "Cannot find drive. A
// drive with the name 'Cert' does not exist." `Import-Module Microsoft.
// PowerShell.Security` by *name* isn't a reliable fix either: if
// `PSModulePath` happens to list a PowerShell 7 install ahead of the Windows
// PowerShell one (common, and outside Windows' control), it resolves the
// wrong, incompatible copy and throws a duplicate-type-data error instead.
// Importing by its literal path under `$PSHOME` (this session's own engine
// directory) sidesteps `PSModulePath` entirely and always gets the right
// one. Every script that references a `Cert:\...` path must start with this.
const ENSURE_CERT_DRIVE: &str = r#"
if (-not (Get-PSDrive -Name Cert -ErrorAction SilentlyContinue)) {
    Import-Module (Join-Path $PSHOME 'Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
}
"#;

// Actually runs the powershell.exe process and waits for it to exit. This is
// the blocking part — see `run_powershell` below for why it never runs
// directly on a Tauri command's own task.
fn run_powershell_blocking(script: &str, envs: &[(String, String)]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // PowerShell writes to a pipe in the console's OEM code page by default
    // (`é` becomes 0x82, `日` becomes `?`), but this side reads UTF-8 — so
    // every non-ASCII character in any result would arrive damaged. Switching
    // the output encoding, without a BOM, makes the two agree.
    let script = format!("[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)\n{script}");

    let mut command = Command::new("powershell.exe");
    command
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-Command", script.as_str()]);
    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command
        .output()
        .map_err(|err| format!("failed to run powershell: {err}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// Runs a PowerShell script and returns its trimmed stdout, or an error built
// from stderr if the process exits non-zero. `envs` are set on the child
// process so untrusted input (e.g. a user-typed hostname) can reach the
// script via $env:NAME instead of being interpolated into the script text,
// where it could break out into arbitrary PowerShell.
//
// Every Tauri command here runs on a small shared pool of async worker
// threads, not the UI thread — but `Command::output()` still *blocks
// whichever worker thread picks it up* until the process exits. A few slow
// or elevated (UAC-waiting) commands running at once can exhaust that whole
// pool, and since every other pending `invoke()` also needs a free worker to
// even start, the entire app appears to freeze until one frees up. Offloading
// the actual process spawn to `spawn_blocking` — a much larger pool set aside
// specifically for blocking work — keeps that pool free so unrelated UI data
// fetches keep responding no matter how long this particular script takes.
async fn run_powershell(script: &str, envs: &[(&str, &str)]) -> Result<String, String> {
    let script = script.to_string();
    let owned_envs: Vec<(String, String)> = envs
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();

    tauri::async_runtime::spawn_blocking(move || run_powershell_blocking(&script, &owned_envs))
        .await
        .map_err(|err| format!("powershell task failed to run: {err}"))?
}

fn unique_temp_path(suffix: &str) -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "zagzig-elevate-{}-{}-{}",
        std::process::id(),
        nanos,
        suffix
    ))
}

// A generic outer (non-elevated) launcher: it resolves the worker/input/
// output paths from env vars (set by run_elevated below), triggers a single
// UAC prompt to run the worker script elevated, waits for it, and prints
// back whatever the worker wrote to its output file.
const ELEVATE_OUTER_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$launcher = $env:ZAGZIG_ELEVATE_LAUNCHER
$worker = $env:ZAGZIG_ELEVATE_WORKER
$inputPath = $env:ZAGZIG_ELEVATE_INPUT
$outputPath = $env:ZAGZIG_ELEVATE_OUTPUT
try {
    Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File', $launcher, '-Worker', $worker, '-InputPath', $inputPath, '-OutputPath', $outputPath) -Wait
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress
    return
}
if (Test-Path -LiteralPath $outputPath) {
    Get-Content -Raw -Encoding UTF8 -LiteralPath $outputPath
} else {
    @{ Success = $false; Error = 'The elevation request was cancelled.' } | ConvertTo-Json -Compress
}
"#;

// Runs `worker_script` (a PowerShell script with a `param($InputPath,
// $OutputPath)` header) elevated via a single UAC prompt, handing it
// `input` through a temp file and returning whatever JSON it wrote to its
// own output temp file. Used for every write that needs administrator
// rights (NRPT rules, routes, ...) so the app itself can stay unelevated.
//
// This one is the biggest reason to keep everything off the shared worker
// pool: `run_powershell` here waits on `Start-Process -Wait` for the
// elevated worker, which itself waits on the user to respond to a UAC
// prompt — that's an indefinite block, not a quick syscall.
async fn run_elevated(worker_script: &str, input: &str) -> Result<String, String> {
    let launcher_path = unique_temp_path("launcher.ps1");
    let worker_path = unique_temp_path("worker.ps1");
    let input_path = unique_temp_path("input.txt");
    let output_path = unique_temp_path("output.json");

    // Windows PowerShell 5.1 reads a file without a byte-order mark as the
    // ANSI code page, which turns every non-ASCII character in the script or
    // the request (an accented path, a Wi-Fi password, ...) into garbage. A
    // UTF-8 BOM makes it read them correctly.
    let with_bom = |text: &str| format!("\u{feff}{text}");
    std::fs::write(&launcher_path, with_bom(ELEVATE_LAUNCHER_SCRIPT))
        .map_err(|err| format!("failed to prepare elevation script: {err}"))?;
    std::fs::write(&worker_path, with_bom(worker_script))
        .map_err(|err| format!("failed to prepare elevation script: {err}"))?;
    std::fs::write(&input_path, with_bom(input))
        .map_err(|err| format!("failed to prepare request: {err}"))?;

    let launcher_str = launcher_path.to_string_lossy().into_owned();
    let worker_str = worker_path.to_string_lossy().into_owned();
    let input_str = input_path.to_string_lossy().into_owned();
    let output_str = output_path.to_string_lossy().into_owned();

    let result = run_powershell(
        ELEVATE_OUTER_SCRIPT,
        &[
            ("ZAGZIG_ELEVATE_LAUNCHER", launcher_str.as_str()),
            ("ZAGZIG_ELEVATE_WORKER", worker_str.as_str()),
            ("ZAGZIG_ELEVATE_INPUT", input_str.as_str()),
            ("ZAGZIG_ELEVATE_OUTPUT", output_str.as_str()),
        ],
    )
    .await;

    let _ = std::fs::remove_file(&launcher_path);
    let _ = std::fs::remove_file(&worker_path);
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    result
}

// Runs inside the elevated process and calls the worker with output written
// as UTF-8 by default: the workers all finish with `Set-Content` /
// `Out-File`, whose default is the ANSI code page and would turn anything
// outside it into `?`. The preference variable is inherited by the worker's
// scope, so none of the workers has to know about it.
const ELEVATE_LAUNCHER_SCRIPT: &str = r#"param(
    [Parameter(Mandatory)] [string]$Worker,
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$PSDefaultParameterValues['Set-Content:Encoding'] = 'UTF8'
$PSDefaultParameterValues['Out-File:Encoding'] = 'UTF8'
& $Worker -InputPath $InputPath -OutputPath $OutputPath
"#;

// PowerShell's ConvertTo-Json collapses single-element arrays down to a bare
// scalar, so a rule with one namespace/server comes back as a string instead
// of an array of one.
fn string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;
    Ok(match value {
        Value::Null => vec![],
        Value::String(s) => vec![s],
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        _ => vec![],
    })
}

// Same single-element collapse quirk as string_or_vec, but for arrays of
// objects (e.g. a traceroute that resolves in a single hop).
fn value_or_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::Deserialize;
    use serde_json::Value;

    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(vec![]),
        Value::Array(items) => items
            .into_iter()
            .map(|item| T::deserialize(item).map_err(serde::de::Error::custom))
            .collect(),
        single => T::deserialize(single)
            .map(|item| vec![item])
            .map_err(serde::de::Error::custom),
    }
}

mod nrpt {
    use serde::{Deserialize, Serialize};

    use crate::{run_powershell, string_or_vec};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NrptRule {
        pub name: String,
        pub display_name: Option<String>,
        pub comment: Option<String>,
        pub namespace: Vec<String>,
        pub name_servers: Vec<String>,
        pub name_encoding: Option<String>,
        pub version: Option<u32>,
        pub dns_sec_enabled: bool,
        pub dns_sec_validation_required: Option<bool>,
        pub dns_sec_query_ipsec_encryption: Option<String>,
        pub dns_sec_query_ipsec_required: Option<bool>,
        pub direct_access_enabled: bool,
        pub direct_access_dns_servers: Vec<String>,
        pub direct_access_proxy_name: Option<String>,
        pub direct_access_proxy_type: Option<String>,
        pub direct_access_query_ipsec_encryption: Option<String>,
        pub direct_access_query_ipsec_required: Option<bool>,
        pub ipsec_ca_restriction: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct RawNrptRule {
        #[serde(rename = "Name")]
        name: String,
        #[serde(rename = "DisplayName", default)]
        display_name: Option<String>,
        #[serde(rename = "Comment", default)]
        comment: Option<String>,
        #[serde(rename = "Namespace", deserialize_with = "string_or_vec", default)]
        namespace: Vec<String>,
        #[serde(rename = "NameServers", deserialize_with = "string_or_vec", default)]
        name_servers: Vec<String>,
        #[serde(rename = "NameEncoding", default)]
        name_encoding: Option<String>,
        #[serde(rename = "Version", default)]
        version: Option<u32>,
        #[serde(rename = "DnsSecEnabled", default)]
        dns_sec_enabled: bool,
        #[serde(rename = "DnsSecValidationRequired", default)]
        dns_sec_validation_required: Option<bool>,
        #[serde(rename = "DnsSecQueryIPsecEncryption", default)]
        dns_sec_query_ipsec_encryption: Option<String>,
        #[serde(rename = "DnsSecQueryIPsecRequired", default)]
        dns_sec_query_ipsec_required: Option<bool>,
        #[serde(rename = "DirectAccessEnabled", default)]
        direct_access_enabled: bool,
        #[serde(
            rename = "DirectAccessDnsServers",
            deserialize_with = "string_or_vec",
            default
        )]
        direct_access_dns_servers: Vec<String>,
        #[serde(rename = "DirectAccessProxyName", default)]
        direct_access_proxy_name: Option<String>,
        #[serde(rename = "DirectAccessProxyType", default)]
        direct_access_proxy_type: Option<String>,
        #[serde(rename = "DirectAccessQueryIPsecEncryption", default)]
        direct_access_query_ipsec_encryption: Option<String>,
        #[serde(rename = "DirectAccessQueryIPsecRequired", default)]
        direct_access_query_ipsec_required: Option<bool>,
        #[serde(rename = "IPsecCARestriction", default)]
        ipsec_ca_restriction: Option<String>,
    }

    #[tauri::command]
    pub async fn get_nrpt_rules() -> Result<Vec<NrptRule>, String> {
        // Deliberately NOT Select-Object with calculated (`@{Name=...;
        // Expression={...}}`) properties for the array-valued fields:
        // Windows PowerShell 5.1's ConvertTo-Json (unlike PowerShell 7's)
        // serializes an array returned from a calculated property as
        // `{"value": [...], "Count": N}` instead of a plain JSON array —
        // confirmed directly against a real non-elevated `powershell.exe`
        // process (what this app actually spawns; an interactive pwsh
        // session, which is PowerShell 7, does not reproduce this). Building
        // the whole object with ForEach-Object + [ordered]@{} sidesteps it
        // entirely. `if ($_.X) { ... }` guards against $_.X being $null:
        // piping $null into ForEach-Object still invokes the script block
        // once with $_ = $null, which then throws on .ToString().
        let trimmed = run_powershell(
            "@(Get-DnsClientNrptRule | ForEach-Object { [ordered]@{ \
Name = $_.Name; \
DisplayName = $_.DisplayName; \
Comment = $_.Comment; \
Namespace = @(if ($_.Namespace) { $_.Namespace | ForEach-Object { $_.ToString() } }); \
NameServers = @(if ($_.NameServers) { $_.NameServers | ForEach-Object { $_.ToString() } }); \
NameEncoding = $_.NameEncoding; \
Version = $_.Version; \
DnsSecEnabled = $_.DnsSecEnabled; \
DnsSecValidationRequired = $_.DnsSecValidationRequired; \
DnsSecQueryIPsecEncryption = $_.DnsSecQueryIPsecEncryption; \
DnsSecQueryIPsecRequired = $_.DnsSecQueryIPsecRequired; \
DirectAccessEnabled = $_.DirectAccessEnabled; \
DirectAccessDnsServers = @(if ($_.DirectAccessDnsServers) { $_.DirectAccessDnsServers | ForEach-Object { $_.ToString() } }); \
DirectAccessProxyName = $_.DirectAccessProxyName; \
DirectAccessProxyType = $_.DirectAccessProxyType; \
DirectAccessQueryIPsecEncryption = $_.DirectAccessQueryIPsecEncryption; \
DirectAccessQueryIPsecRequired = $_.DirectAccessQueryIPsecRequired; \
IPsecCARestriction = $_.IPsecCARestriction \
} }) | ConvertTo-Json -Depth 4 -Compress",
            &[],
        )
        .await?;

        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let raw: Vec<RawNrptRule> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(raw
            .into_iter()
            .map(|r| NrptRule {
                name: r.name,
                display_name: r.display_name,
                comment: r.comment,
                namespace: r.namespace,
                name_servers: r.name_servers,
                name_encoding: r.name_encoding,
                version: r.version,
                dns_sec_enabled: r.dns_sec_enabled,
                dns_sec_validation_required: r.dns_sec_validation_required,
                dns_sec_query_ipsec_encryption: r.dns_sec_query_ipsec_encryption,
                dns_sec_query_ipsec_required: r.dns_sec_query_ipsec_required,
                direct_access_enabled: r.direct_access_enabled,
                direct_access_dns_servers: r.direct_access_dns_servers,
                direct_access_proxy_name: r.direct_access_proxy_name,
                direct_access_proxy_type: r.direct_access_proxy_type,
                direct_access_query_ipsec_encryption: r.direct_access_query_ipsec_encryption,
                direct_access_query_ipsec_required: r.direct_access_query_ipsec_required,
                ipsec_ca_restriction: r.ipsec_ca_restriction,
            })
            .collect())
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    // Removing an NRPT rule needs administrator rights, but the app itself
    // runs unelevated so every read-only feature stays free of UAC prompts.
    // See `crate::run_elevated` for how the single UAC prompt and temp-file
    // round trip work.
    const REMOVE_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $name = Get-Content -Raw -LiteralPath $InputPath
    Remove-DnsClientNrptRule -Name $name -Force -ErrorAction Stop
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn remove_nrpt_rule(name: String) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Missing rule name.".to_string());
        }

        let trimmed = crate::run_elevated(REMOVE_WORKER_SCRIPT, name).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NewNrptRule {
        pub namespace: String,
        pub name_servers: Vec<String>,
        #[serde(default)]
        pub comment: String,
        pub name_encoding: String,
        pub dns_sec_enabled: bool,
        pub dns_sec_validation_required: bool,
        pub dns_sec_query_ipsec_required: bool,
        pub dns_sec_query_ipsec_encryption: String,
        pub direct_access_enabled: bool,
        pub direct_access_dns_servers: Vec<String>,
        pub direct_access_proxy_type: String,
        pub direct_access_proxy_name: String,
        pub direct_access_query_ipsec_required: bool,
        pub direct_access_query_ipsec_encryption: String,
        pub ipsec_ca_restriction: String,
    }

    fn plain(value: &str, what: &str, max: usize) -> Result<String, String> {
        let v = value.trim();
        if v.len() > max || v.chars().any(|c| c.is_control() || c.is_whitespace() || c == '"' || c == '\'' || c == '`') {
            return Err(format!("{what} has characters that can't be used."));
        }
        Ok(v.to_string())
    }

    fn one_of(value: &str, allowed: &[&str], what: &str) -> Result<String, String> {
        let v = value.trim();
        if v.is_empty() || allowed.contains(&v) {
            Ok(v.to_string())
        } else {
            Err(format!("{what} isn't a recognised value."))
        }
    }

    fn servers(list: &[String], what: &str) -> Result<Vec<String>, String> {
        list.iter()
            .map(|s| {
                let s = plain(s, what, 253)?;
                let hostname = !s.is_empty()
                    && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_');
                if s.parse::<std::net::IpAddr>().is_ok() || hostname {
                    Ok(s)
                } else {
                    Err(format!("\"{s}\" isn't an IP address or a host name."))
                }
            })
            .collect()
    }

    // Checks a form and turns it into the JSON the worker reads. Empty text
    // fields stay empty strings; the worker leaves those parameters out.
    fn validate_rule(spec: &NewNrptRule) -> Result<serde_json::Value, String> {
        let namespace = plain(&spec.namespace, "The namespace", 255)?;
        if namespace.is_empty() {
            return Err("Enter a namespace, like .corp.example.com.".to_string());
        }
        let name_servers = servers(&spec.name_servers, "A DNS server")?;
        let da_servers = servers(&spec.direct_access_dns_servers, "A DirectAccess DNS server")?;
        if name_servers.is_empty() && !(spec.direct_access_enabled && !da_servers.is_empty()) {
            return Err("Enter at least one DNS server.".to_string());
        }
        let comment = spec.comment.trim();
        if comment.len() > 500 || comment.chars().any(char::is_control) {
            return Err("The comment is too long or has characters that can't be used.".to_string());
        }
        let encryption = ["None", "Low", "Medium", "High"];
        let proxy_name = plain(&spec.direct_access_proxy_name, "The proxy name", 253)?;
        let proxy_type = one_of(
            &spec.direct_access_proxy_type,
            &["NoProxy", "UseDefault", "UseProxyName"],
            "The proxy type",
        )?;
        if proxy_type == "UseProxyName" {
            let port_ok = proxy_name
                .rsplit_once(':')
                .is_some_and(|(host, port)| !host.is_empty() && port.parse::<u16>().is_ok());
            if !port_ok {
                return Err("Enter the proxy as name:port, like proxy.corp.example.com:8080.".to_string());
            }
        }
        if !spec.dns_sec_enabled
            && (spec.dns_sec_validation_required
                || spec.dns_sec_query_ipsec_required
                || !spec.dns_sec_query_ipsec_encryption.trim().is_empty())
        {
            return Err("Turn on DNSSEC to use the DNSSEC options.".to_string());
        }
        Ok(serde_json::json!({
            "Namespace": namespace,
            "NameServers": name_servers,
            "Comment": comment,
            "NameEncoding": one_of(
                &spec.name_encoding,
                &["Disable", "Utf8WithMapping", "Utf8WithoutMapping", "Punycode"],
                "The name encoding",
            )?,
            "DnsSecEnabled": spec.dns_sec_enabled,
            "DnsSecValidationRequired": spec.dns_sec_validation_required,
            "DnsSecQueryIpsecRequired": spec.dns_sec_query_ipsec_required,
            "DnsSecQueryIpsecEncryption": one_of(&spec.dns_sec_query_ipsec_encryption, &encryption, "The DNSSEC IPsec encryption")?,
            "DirectAccessEnabled": spec.direct_access_enabled,
            "DirectAccessDnsServers": da_servers,
            "DirectAccessProxyType": proxy_type,
            "DirectAccessProxyName": proxy_name,
            "DirectAccessQueryIpsecRequired": spec.direct_access_query_ipsec_required,
            "DirectAccessQueryIpsecEncryption": one_of(&spec.direct_access_query_ipsec_encryption, &encryption, "The DirectAccess IPsec encryption")?,
            "IpsecCaRestriction": spec.ipsec_ca_restriction.trim(),
        }))
    }

    // Add-DnsClientNrptRule names these parameters differently from the
    // properties Get-DnsClientNrptRule returns (DnsSecEnable, DAEnable,
    // IPsecTrustAuthority, ...), so the mapping is spelled out here.
    fn add_worker() -> String {
        crate::elevated_json::worker(
            r#"    $p = @{ Namespace = @([string]$req.Namespace) }
    if (@($req.NameServers).Count) { $p.NameServers = @($req.NameServers | ForEach-Object { [string]$_ }) }
    if ($req.Comment) { $p.Comment = [string]$req.Comment }
    if ($req.NameEncoding) { $p.NameEncoding = [string]$req.NameEncoding }
    if ($req.DnsSecEnabled) { $p.DnsSecEnable = $true }
    if ($req.DnsSecValidationRequired) { $p.DnsSecValidationRequired = $true }
    if ($req.DnsSecQueryIpsecRequired) { $p.DnsSecIPsecRequired = $true }
    if ($req.DnsSecQueryIpsecEncryption) { $p.DnsSecIPsecEncryptionType = [string]$req.DnsSecQueryIpsecEncryption }
    if ($req.DirectAccessEnabled) { $p.DAEnable = $true }
    if (@($req.DirectAccessDnsServers).Count) { $p.DANameServers = @($req.DirectAccessDnsServers | ForEach-Object { [string]$_ }) }
    if ($req.DirectAccessProxyType) { $p.DAProxyType = [string]$req.DirectAccessProxyType }
    if ($req.DirectAccessProxyName) { $p.DAProxyServerName = [string]$req.DirectAccessProxyName }
    if ($req.DirectAccessQueryIpsecRequired) { $p.DAIPsecRequired = $true }
    if ($req.DirectAccessQueryIpsecEncryption) { $p.DAIPsecEncryptionType = [string]$req.DirectAccessQueryIpsecEncryption }
    if ($req.IpsecCaRestriction) { $p.IPsecTrustAuthority = [string]$req.IpsecCaRestriction }
    Add-DnsClientNrptRule @p -ErrorAction Stop | Out-Null"#,
        )
    }

    #[tauri::command]
    pub async fn add_nrpt_rule(spec: NewNrptRule) -> Result<(), String> {
        let payload = validate_rule(&spec)?.to_string();
        crate::elevated_json::run(&add_worker(), &payload).await
    }

    #[cfg(test)]
    mod add_tests {
        use super::*;

        fn spec() -> NewNrptRule {
            NewNrptRule {
                namespace: ".corp.example.com".into(),
                name_servers: vec!["10.0.0.1".into(), "dns.corp.example.com".into()],
                comment: "".into(),
                name_encoding: "Disable".into(),
                dns_sec_enabled: false,
                dns_sec_validation_required: false,
                dns_sec_query_ipsec_required: false,
                dns_sec_query_ipsec_encryption: "".into(),
                direct_access_enabled: false,
                direct_access_dns_servers: vec![],
                direct_access_proxy_type: "NoProxy".into(),
                direct_access_proxy_name: "".into(),
                direct_access_query_ipsec_required: false,
                direct_access_query_ipsec_encryption: "".into(),
                ipsec_ca_restriction: "".into(),
            }
        }

        #[test]
        fn accepts_a_good_rule() {
            let json = validate_rule(&spec()).unwrap();
            assert_eq!(json["Namespace"], ".corp.example.com");
            assert_eq!(json["NameServers"], serde_json::json!(["10.0.0.1", "dns.corp.example.com"]));
        }

        #[test]
        fn rejects_bad_rules() {
            let bad = |f: &dyn Fn(&mut NewNrptRule)| {
                let mut s = spec();
                f(&mut s);
                assert!(validate_rule(&s).is_err());
            };
            bad(&|s| s.namespace = "  ".into());
            bad(&|s| s.namespace = "a b".into());
            bad(&|s| s.namespace = "x\"; calc; \"".into());
            bad(&|s| s.name_servers = vec![]);
            bad(&|s| s.name_servers = vec!["10.0.0.1; calc".into()]);
            bad(&|s| s.name_encoding = "Rot13".into());
            bad(&|s| s.dns_sec_query_ipsec_encryption = "Maximum".into());
            bad(&|s| s.direct_access_proxy_type = "UseProxyName".into());
            bad(&|s| {
                s.direct_access_proxy_type = "UseProxyName".into();
                s.direct_access_proxy_name = "proxy.example.com".into();
            });
            bad(&|s| s.dns_sec_validation_required = true);
        }

        #[test]
        fn direct_access_servers_can_stand_in() {
            let mut s = spec();
            s.name_servers = vec![];
            assert!(validate_rule(&s).is_err());
            s.direct_access_enabled = true;
            s.direct_access_dns_servers = vec!["fd00::1".into()];
            assert!(validate_rule(&s).is_ok());
        }

        #[test]
        fn worker_parses_and_uses_real_parameter_names() {
            assert_eq!(crate::powershell_syntax_errors(&add_worker()), Vec::<String>::new());
            let out = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "(Get-Command Add-DnsClientNrptRule).Parameters.Keys -join ' '"])
                .output();
            if let Ok(out) = out {
                let keys = String::from_utf8_lossy(&out.stdout).to_string();
                if keys.contains("Namespace") {
                    for name in add_worker().split("$p.").skip(1).map(|s| s.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("")) {
                        assert!(keys.split_whitespace().any(|k| k == name), "no such parameter: {name}");
                    }
                }
            }
        }
    }
}

mod user {
    use serde::{Deserialize, Serialize};

    use crate::run_powershell;

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LdapUser {
        pub display_name: Option<String>,
        pub given_name: Option<String>,
        pub surname: Option<String>,
        pub email_address: Option<String>,
        pub voice_telephone_number: Option<String>,
        pub description: Option<String>,
        pub distinguished_name: Option<String>,
        pub user_principal_name: Option<String>,
        pub enabled: Option<bool>,
        pub title: Option<String>,
        pub department: Option<String>,
        pub company: Option<String>,
        pub office: Option<String>,
        pub manager: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CurrentUser {
        pub user_name: String,
        pub domain: String,
        pub computer_name: String,
        pub is_domain_joined: bool,
        pub sid: Option<String>,
        pub profile_path: String,
        pub ldap: Option<LdapUser>,
        pub ldap_error: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawLdapUser {
        #[serde(default)]
        display_name: Option<String>,
        #[serde(default)]
        given_name: Option<String>,
        #[serde(default)]
        surname: Option<String>,
        #[serde(default)]
        email_address: Option<String>,
        #[serde(default)]
        voice_telephone_number: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        distinguished_name: Option<String>,
        #[serde(default)]
        user_principal_name: Option<String>,
        #[serde(default)]
        enabled: Option<bool>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        department: Option<String>,
        #[serde(default)]
        company: Option<String>,
        #[serde(default)]
        office: Option<String>,
        #[serde(default)]
        manager: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawCurrentUser {
        user_name: String,
        domain: String,
        computer_name: String,
        is_domain_joined: bool,
        #[serde(default)]
        sid: Option<String>,
        profile_path: String,
        #[serde(default)]
        ldap: Option<RawLdapUser>,
        #[serde(default)]
        ldap_error: Option<String>,
    }

    // Reads the signed-in user's local profile info, then — if the machine
    // is domain-joined — looks the account up over LDAP via
    // System.DirectoryServices.AccountManagement for directory details
    // (title, department, manager, etc.) that Windows doesn't expose
    // locally.
    const CURRENT_USER_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.DirectoryServices.AccountManagement

$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
$cs = Get-CimInstance Win32_ComputerSystem

$result = [ordered]@{
    UserName = $env:USERNAME
    Domain = $env:USERDOMAIN
    ComputerName = $env:COMPUTERNAME
    IsDomainJoined = [bool]$cs.PartOfDomain
    Sid = $identity.User.Value
    ProfilePath = $env:USERPROFILE
    Ldap = $null
    LdapError = $null
}

if ($cs.PartOfDomain) {
    try {
        $ctx = New-Object System.DirectoryServices.AccountManagement.PrincipalContext('Domain')
        $user = [System.DirectoryServices.AccountManagement.UserPrincipal]::FindByIdentity($ctx, $env:USERNAME)
        if ($user) {
            $de = $user.GetUnderlyingObject()
            $result.Ldap = [ordered]@{
                DisplayName = $user.DisplayName
                GivenName = $user.GivenName
                Surname = $user.Surname
                EmailAddress = $user.EmailAddress
                VoiceTelephoneNumber = $user.VoiceTelephoneNumber
                Description = $user.Description
                DistinguishedName = $user.DistinguishedName
                UserPrincipalName = $user.UserPrincipalName
                Enabled = $user.Enabled
                Title = $de.Properties['title'].Value
                Department = $de.Properties['department'].Value
                Company = $de.Properties['company'].Value
                Office = $de.Properties['physicalDeliveryOfficeName'].Value
                Manager = $de.Properties['manager'].Value
            }
        } else {
            $result.LdapError = "User principal not found"
        }
    } catch {
        $result.LdapError = $_.Exception.Message
    }
}

$result | ConvertTo-Json -Depth 5 -Compress
"#;

    #[tauri::command]
    pub async fn get_current_user() -> Result<CurrentUser, String> {
        let trimmed = run_powershell(CURRENT_USER_SCRIPT, &[]).await?;

        let raw: RawCurrentUser = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(CurrentUser {
            user_name: raw.user_name,
            domain: raw.domain,
            computer_name: raw.computer_name,
            is_domain_joined: raw.is_domain_joined,
            sid: raw.sid,
            profile_path: raw.profile_path,
            ldap: raw.ldap.map(|l| LdapUser {
                display_name: l.display_name,
                given_name: l.given_name,
                surname: l.surname,
                email_address: l.email_address,
                voice_telephone_number: l.voice_telephone_number,
                description: l.description,
                distinguished_name: l.distinguished_name,
                user_principal_name: l.user_principal_name,
                enabled: l.enabled,
                title: l.title,
                department: l.department,
                company: l.company,
                office: l.office,
                manager: l.manager,
            }),
            ldap_error: raw.ldap_error,
        })
    }
}

// Backs the "Connection Test" feature: a plain-language wrapper around ping
// and traceroute for people who don't know those words.
mod connection {
    use serde::{Deserialize, Serialize};

    use crate::{run_powershell, value_or_vec};

    const TARGET_ENV_VAR: &str = "ZAGZIG_CONN_TARGET";

    fn validated_target(target: &str) -> Result<&str, String> {
        let target = target.trim();
        if target.is_empty() {
            return Err("Enter an address or website to test.".to_string());
        }
        Ok(target)
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PingReply {
        pub success: bool,
        pub status: String,
        pub roundtrip_time_ms: Option<i64>,
        pub address: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PingResult {
        pub target: String,
        pub replies: Vec<PingReply>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawPingReply {
        success: bool,
        status: String,
        #[serde(default)]
        roundtrip_time_ms: Option<i64>,
        #[serde(default)]
        address: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawPingResult {
        target: String,
        replies: Vec<RawPingReply>,
    }

    // Sends 4 pings and reports round-trip time for each. $env:ZAGZIG_CONN_TARGET
    // carries the user-supplied host so it never touches the script text itself.
    const PING_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$targetHost = $env:ZAGZIG_CONN_TARGET
$ping = New-Object System.Net.NetworkInformation.Ping
$replies = 1..4 | ForEach-Object {
    try {
        $reply = $ping.Send($targetHost, 2000)
        [ordered]@{
            Success = $reply.Status -eq 'Success'
            Status = $reply.Status.ToString()
            RoundtripTimeMs = if ($reply.Status -eq 'Success') { $reply.RoundtripTime } else { $null }
            Address = if ($reply.Address) { $reply.Address.ToString() } else { $null }
        }
    } catch {
        $ex = $_.Exception
        while ($ex.InnerException) { $ex = $ex.InnerException }
        [ordered]@{ Success = $false; Status = $ex.Message; RoundtripTimeMs = $null; Address = $null }
    }
}
[ordered]@{ Target = $targetHost; Replies = $replies } | ConvertTo-Json -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn ping_host(target: String) -> Result<PingResult, String> {
        let target = validated_target(&target)?;
        let trimmed = run_powershell(PING_SCRIPT, &[(TARGET_ENV_VAR, target)]).await?;

        let raw: RawPingResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(PingResult {
            target: raw.target,
            replies: raw
                .replies
                .into_iter()
                .map(|r| PingReply {
                    success: r.success,
                    status: r.status,
                    roundtrip_time_ms: r.roundtrip_time_ms,
                    address: r.address,
                })
                .collect(),
        })
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TraceHop {
        pub hop: u32,
        pub address: Option<String>,
        pub hostname: Option<String>,
        pub roundtrip_time_ms: Option<i64>,
        pub status: String,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TracerouteResult {
        pub target: String,
        pub hops: Vec<TraceHop>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawTraceHop {
        hop: u32,
        #[serde(default)]
        address: Option<String>,
        #[serde(default)]
        hostname: Option<String>,
        #[serde(default)]
        roundtrip_time_ms: Option<i64>,
        status: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawTracerouteResult {
        target: String,
        #[serde(deserialize_with = "value_or_vec", default)]
        hops: Vec<RawTraceHop>,
    }

    // Walks TTL from 1 upward, one ping each, recording whichever router
    // replies "time exceeded" at each hop until the target itself answers.
    // This is what traceroute/tracert do; .NET has no built-in for it.
    //
    // Each hop's address also gets a reverse-DNS (PTR) lookup, same as
    // tracert.exe does by default (unlike tracert, there's no way to opt
    // out here — if that turns out to matter, it'd need a UI toggle).
    // -QuickTimeout keeps a hop with no PTR record from stalling the whole
    // trace; the stopwatch stops before the lookup starts, so it never
    // pollutes RoundtripTimeMs.
    const TRACEROUTE_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$targetHost = $env:ZAGZIG_CONN_TARGET
$ping = New-Object System.Net.NetworkInformation.Ping
$maxHops = 30
$timeoutMs = 1000
$buffer = [System.Text.Encoding]::ASCII.GetBytes("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
$hops = @()
for ($ttl = 1; $ttl -le $maxHops; $ttl++) {
    $options = New-Object System.Net.NetworkInformation.PingOptions($ttl, $true)
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $reply = $ping.Send($targetHost, $timeoutMs, $buffer, $options)
        $sw.Stop()
        $address = if ($reply.Address) { $reply.Address.ToString() } else { $null }
        $hostname = $null
        if ($address) {
            try {
                $ptr = Resolve-DnsName -Name $address -Type PTR -DnsOnly -QuickTimeout -ErrorAction Stop
                $hostname = $ptr | Select-Object -First 1 -ExpandProperty NameHost
            } catch {
                $hostname = $null
            }
        }
        $hops += [ordered]@{
            Hop = $ttl
            Address = $address
            Hostname = $hostname
            RoundtripTimeMs = if ($reply.Status -eq 'Success' -or $reply.Status -eq 'TtlExpired') { $sw.ElapsedMilliseconds } else { $null }
            Status = $reply.Status.ToString()
        }
        if ($reply.Status -eq 'Success') { break }
    } catch {
        $ex = $_.Exception
        while ($ex.InnerException) { $ex = $ex.InnerException }
        $hops += [ordered]@{ Hop = $ttl; Address = $null; Hostname = $null; RoundtripTimeMs = $null; Status = $ex.Message }
        break
    }
}
[ordered]@{ Target = $targetHost; Hops = $hops } | ConvertTo-Json -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn traceroute_host(target: String) -> Result<TracerouteResult, String> {
        let target = validated_target(&target)?;
        let trimmed = run_powershell(TRACEROUTE_SCRIPT, &[(TARGET_ENV_VAR, target)]).await?;

        let raw: RawTracerouteResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(TracerouteResult {
            target: raw.target,
            hops: raw
                .hops
                .into_iter()
                .map(|h| TraceHop {
                    hop: h.hop,
                    address: h.address,
                    hostname: h.hostname,
                    roundtrip_time_ms: h.roundtrip_time_ms,
                    status: h.status,
                })
                .collect(),
        })
    }
}

// Backs the "Network Routes" feature — the GUI equivalent of `route print` /
// `route add` / `route delete`, built on the modern NetTCPIP cmdlets instead
// of parsing route.exe's (locale-dependent) text table.
mod routing {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NetRoute {
        pub destination_prefix: String,
        pub next_hop: String,
        pub interface_alias: String,
        pub interface_index: u32,
        pub route_metric: u32,
        pub interface_metric: u32,
        pub protocol: String,
        pub store: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawNetRoute {
        destination_prefix: String,
        next_hop: String,
        interface_alias: String,
        interface_index: u32,
        route_metric: u32,
        interface_metric: u32,
        protocol: String,
        store: String,
    }

    #[tauri::command]
    pub async fn get_routes() -> Result<Vec<NetRoute>, String> {
        let trimmed = run_powershell(
            "@(Get-NetRoute -AddressFamily IPv4 | Select-Object \
DestinationPrefix, NextHop, InterfaceAlias, InterfaceIndex, RouteMetric, InterfaceMetric, \
@{Name='Protocol';Expression={ $_.Protocol.ToString() }}, \
@{Name='Store';Expression={ $_.Store.ToString() }} \
) | Sort-Object Protocol, DestinationPrefix | ConvertTo-Json -Depth 3 -Compress",
            &[],
        )
        .await?;

        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let raw: Vec<RawNetRoute> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(raw
            .into_iter()
            .map(|r| NetRoute {
                destination_prefix: r.destination_prefix,
                next_hop: r.next_hop,
                interface_alias: r.interface_alias,
                interface_index: r.interface_index,
                route_metric: r.route_metric,
                interface_metric: r.interface_metric,
                protocol: r.protocol,
                store: r.store,
            })
            .collect())
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct AddRouteRequest<'a> {
        #[serde(rename = "DestinationPrefix")]
        destination_prefix: &'a str,
        #[serde(rename = "NextHop")]
        next_hop: &'a str,
        #[serde(rename = "InterfaceIndex")]
        interface_index: u32,
        #[serde(rename = "RouteMetric")]
        route_metric: Option<u32>,
        #[serde(rename = "PolicyStore")]
        policy_store: &'a str,
    }

    const ADD_ROUTE_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $params = @{
        DestinationPrefix = $req.DestinationPrefix
        NextHop = $req.NextHop
        InterfaceIndex = $req.InterfaceIndex
        PolicyStore = $req.PolicyStore
    }
    if ($req.RouteMetric) { $params.RouteMetric = $req.RouteMetric }
    New-NetRoute @params -ErrorAction Stop | Out-Null
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn add_route(
        destination_prefix: String,
        next_hop: String,
        interface_index: u32,
        route_metric: Option<u32>,
        persistent: bool,
    ) -> Result<(), String> {
        let destination_prefix = destination_prefix.trim();
        let next_hop = next_hop.trim();
        if destination_prefix.is_empty() || next_hop.is_empty() {
            return Err("Missing destination or next hop.".to_string());
        }

        let request = AddRouteRequest {
            destination_prefix,
            next_hop,
            interface_index,
            route_metric,
            policy_store: if persistent {
                "PersistentStore"
            } else {
                "ActiveStore"
            },
        };
        let input = serde_json::to_string(&request)
            .map_err(|err| format!("failed to prepare request: {err}"))?;

        let trimmed = run_elevated(ADD_ROUTE_WORKER_SCRIPT, &input).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[derive(Debug, Serialize)]
    struct RemoveRouteRequest<'a> {
        #[serde(rename = "DestinationPrefix")]
        destination_prefix: &'a str,
        #[serde(rename = "NextHop")]
        next_hop: &'a str,
        #[serde(rename = "InterfaceIndex")]
        interface_index: u32,
    }

    const REMOVE_ROUTE_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    Remove-NetRoute -DestinationPrefix $req.DestinationPrefix -NextHop $req.NextHop -InterfaceIndex $req.InterfaceIndex -Confirm:$false -ErrorAction Stop
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn remove_route(
        destination_prefix: String,
        next_hop: String,
        interface_index: u32,
    ) -> Result<(), String> {
        let destination_prefix = destination_prefix.trim();
        let next_hop = next_hop.trim();
        if destination_prefix.is_empty() {
            return Err("Missing destination.".to_string());
        }

        let request = RemoveRouteRequest {
            destination_prefix,
            next_hop,
            interface_index,
        };
        let input = serde_json::to_string(&request)
            .map_err(|err| format!("failed to prepare request: {err}"))?;

        let trimmed = run_elevated(REMOVE_ROUTE_WORKER_SCRIPT, &input).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }
}

// Backs the "DNS Servers" feature: per-adapter DNS server configuration,
// same as the "Use the following DNS server addresses" dialog in Network
// Adapter properties — except that dialog hard-codes two fields
// (Preferred/Alternate). Set-DnsClientServerAddress itself has no such
// limit, so this exposes an arbitrary-length list instead.
mod dns {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell, string_or_vec};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DnsInterface {
        pub interface_alias: String,
        pub interface_index: u32,
        pub server_addresses: Vec<String>,
        pub dhcp: bool,
        pub status: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawDnsInterface {
        interface_alias: String,
        interface_index: u32,
        #[serde(deserialize_with = "string_or_vec", default)]
        server_addresses: Vec<String>,
        #[serde(default)]
        dhcp: bool,
        #[serde(default)]
        status: String,
    }

    #[tauri::command]
    pub async fn get_dns_settings() -> Result<Vec<DnsInterface>, String> {
        let trimmed = run_powershell(
            "$dns = Get-DnsClientServerAddress -AddressFamily IPv4; \
$adapters = @{}; \
Get-NetAdapter | ForEach-Object { $adapters[$_.InterfaceIndex] = $_.Status.ToString() }; \
$dhcpMap = @{}; \
Get-NetIPInterface -AddressFamily IPv4 | ForEach-Object { $dhcpMap[$_.InterfaceIndex] = ($_.Dhcp.ToString() -eq 'Enabled') }; \
$result = $dns | Where-Object { $adapters.ContainsKey($_.InterfaceIndex) } | ForEach-Object { \
[ordered]@{ \
InterfaceAlias = $_.InterfaceAlias; \
InterfaceIndex = $_.InterfaceIndex; \
ServerAddresses = @($_.ServerAddresses); \
Dhcp = [bool]$dhcpMap[$_.InterfaceIndex]; \
Status = $adapters[$_.InterfaceIndex] \
} \
}; \
@($result) | Sort-Object InterfaceAlias | ConvertTo-Json -Depth 4 -Compress",
            &[],
        )
        .await?;

        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let raw: Vec<RawDnsInterface> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(raw
            .into_iter()
            .map(|r| DnsInterface {
                interface_alias: r.interface_alias,
                interface_index: r.interface_index,
                server_addresses: r.server_addresses,
                dhcp: r.dhcp,
                status: r.status,
            })
            .collect())
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct SetDnsRequest<'a> {
        #[serde(rename = "InterfaceIndex")]
        interface_index: u32,
        #[serde(rename = "ServerAddresses")]
        server_addresses: &'a [String],
    }

    const SET_DNS_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    Set-DnsClientServerAddress -InterfaceIndex $req.InterfaceIndex -ServerAddresses $req.ServerAddresses -ErrorAction Stop
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn set_dns_servers(interface_index: u32, servers: Vec<String>) -> Result<(), String> {
        let servers: Vec<String> = servers
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if servers.is_empty() {
            return Err("Enter at least one DNS server.".to_string());
        }

        let request = SetDnsRequest {
            interface_index,
            server_addresses: &servers,
        };
        let input = serde_json::to_string(&request)
            .map_err(|err| format!("failed to prepare request: {err}"))?;

        let trimmed = run_elevated(SET_DNS_WORKER_SCRIPT, &input).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[derive(Debug, Serialize)]
    struct ResetDnsRequest {
        #[serde(rename = "InterfaceIndex")]
        interface_index: u32,
    }

    const RESET_DNS_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    Set-DnsClientServerAddress -InterfaceIndex $req.InterfaceIndex -ResetServerAddresses -ErrorAction Stop
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn reset_dns_servers(interface_index: u32) -> Result<(), String> {
        let request = ResetDnsRequest { interface_index };
        let input = serde_json::to_string(&request)
            .map_err(|err| format!("failed to prepare request: {err}"))?;

        let trimmed = run_elevated(RESET_DNS_WORKER_SCRIPT, &input).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DnsResolveResult {
        pub hostname: String,
        pub server: Option<String>,
        pub resolved: bool,
        pub addresses: Vec<String>,
        pub error: Option<String>,
        pub query_time_ms: i64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawDnsResolveResult {
        resolved: bool,
        #[serde(deserialize_with = "string_or_vec", default)]
        addresses: Vec<String>,
        #[serde(default)]
        error: Option<String>,
        query_time_ms: i64,
    }

    const RESOLVE_HOSTNAME_ENV_VAR: &str = "ZAGZIG_RESOLVE_HOSTNAME";
    const RESOLVE_SERVER_ENV_VAR: &str = "ZAGZIG_RESOLVE_SERVER";

    // Resolve-DnsName against either the system's configured resolver or an
    // explicit server, so the DNS monitor can watch a hostname through
    // whichever server it's meant to be reachable from.
    const RESOLVE_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$hostnameToResolve = $env:ZAGZIG_RESOLVE_HOSTNAME
$server = $env:ZAGZIG_RESOLVE_SERVER
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $params = @{ Name = $hostnameToResolve; ErrorAction = 'Stop'; DnsOnly = $true }
    if ($server) { $params.Server = $server }
    $records = @(Resolve-DnsName @params)
    $sw.Stop()
    $addresses = @($records | Where-Object { $_.IPAddress } | Select-Object -ExpandProperty IPAddress)
    [ordered]@{
        Resolved = $addresses.Count -gt 0
        Addresses = $addresses
        Error = $null
        QueryTimeMs = $sw.ElapsedMilliseconds
    } | ConvertTo-Json -Compress
} catch {
    $sw.Stop()
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    [ordered]@{
        Resolved = $false
        Addresses = @()
        Error = $ex.Message
        QueryTimeMs = $sw.ElapsedMilliseconds
    } | ConvertTo-Json -Compress
}
"#;

    #[tauri::command]
    pub async fn resolve_hostname(
        hostname: String,
        server: Option<String>,
    ) -> Result<DnsResolveResult, String> {
        let hostname = hostname.trim();
        if hostname.is_empty() {
            return Err("Enter a hostname to resolve.".to_string());
        }
        let server = server
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let mut envs = vec![(RESOLVE_HOSTNAME_ENV_VAR, hostname)];
        if let Some(server) = server {
            envs.push((RESOLVE_SERVER_ENV_VAR, server));
        }

        let trimmed = run_powershell(RESOLVE_SCRIPT, &envs).await?;
        let raw: RawDnsResolveResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(DnsResolveResult {
            hostname: hostname.to_string(),
            server: server.map(str::to_string),
            resolved: raw.resolved,
            addresses: raw.addresses,
            error: raw.error,
            query_time_ms: raw.query_time_ms,
        })
    }
}

// Lets the UI know upfront whether the signed-in account can actually
// approve a UAC prompt, so admin-only controls (removing an NRPT rule,
// adding/removing a route) can be shown locked instead of surprising the
// user with an elevation request that's doomed to ask for credentials they
// don't have.
mod system {
    use serde::Deserialize;

    use crate::run_powershell;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct AdminCheckResult {
        is_administrator: bool,
    }

    // `IsInRole(Administrator)` only reflects whether *this process* is
    // currently elevated — under UAC, that's false for admins and
    // non-admins alike unless the app was explicitly "Run as
    // Administrator". What the UI actually needs is whether the account
    // could elevate at all, so this checks local Administrators-group
    // membership by SID (covers both direct membership and membership via
    // an AD group like Domain Admins), falling back to IsInRole if that
    // lookup fails for some reason.
    const IS_ADMIN_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
try {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $members = Get-LocalGroupMember -SID "S-1-5-32-544" -ErrorAction Stop
    $memberSids = $members | Select-Object -ExpandProperty SID | ForEach-Object { $_.Value }
    $currentSids = @($id.User.Value) + ($id.Groups | ForEach-Object { $_.Value })
    $isAdmin = [bool]($currentSids | Where-Object { $memberSids -contains $_ })
} catch {
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($id)
    $isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}
@{ IsAdministrator = [bool]$isAdmin } | ConvertTo-Json -Compress
"#;

    #[tauri::command]
    pub async fn is_administrator() -> Result<bool, String> {
        let trimmed = run_powershell(IS_ADMIN_SCRIPT, &[]).await?;
        let parsed: AdminCheckResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        Ok(parsed.is_administrator)
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RelaunchResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    // Launches a second, elevated copy of this same executable via a UAC
    // prompt, then closes the current (unelevated) instance once that
    // succeeds. Left running if the user cancels the prompt.
    const RELAUNCH_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
try {
    Start-Process -FilePath $env:ZAGZIG_EXE_PATH -Verb RunAs | Out-Null
    @{ Success = $true } | ConvertTo-Json -Compress
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress
}
"#;

    #[tauri::command]
    pub async fn relaunch_as_administrator(app: tauri::AppHandle) -> Result<(), String> {
        let exe = std::env::current_exe()
            .map_err(|err| format!("failed to determine executable path: {err}"))?;
        let exe_str = exe.to_string_lossy().into_owned();

        let trimmed =
            run_powershell(RELAUNCH_SCRIPT, &[("ZAGZIG_EXE_PATH", exe_str.as_str())]).await?;
        let parsed: RelaunchResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            app.exit(0);
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }
}

// Backs the "Code Signing" feature: a thin wrapper around signtool.exe (the
// Authenticode signing/verification tool from the Windows SDK). Unlike every
// other feature here, signtool.exe isn't part of Windows itself, so this
// first has to find it, then shells out to the real executable directly
// (rather than through PowerShell) so file paths, thumbprints and PFX
// passwords are passed as argv entries instead of being interpolated into a
// script — that's the difference between "an argument with a space in it"
// and "a script injection".
mod signtool {
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use serde::{Deserialize, Serialize};

    use crate::{run_powershell, CREATE_NO_WINDOW};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SigntoolStatus {
        pub found: bool,
        pub path: Option<String>,
    }

    // Windows SDKs install signtool.exe under a per-version folder, e.g.
    // "...\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe" — there's no
    // registry key that reliably points at "the" install, and multiple SDK
    // versions commonly coexist. This checks a couple of un-versioned
    // fallback layouts, then globs every version folder under both possible
    // Program Files roots and works newest-first (version folder names sort
    // correctly as plain strings).
    fn find_in_kits() -> Option<PathBuf> {
        let arch_dir = if cfg!(target_pointer_width = "64") {
            "x64"
        } else {
            "x86"
        };

        for env_var in ["ProgramFiles(x86)", "ProgramFiles"] {
            let Ok(program_files) = std::env::var(env_var) else {
                continue;
            };
            let kits = PathBuf::from(program_files).join("Windows Kits");

            let unversioned = [
                kits.join("10").join("bin").join(arch_dir).join("signtool.exe"),
                kits.join("8.1").join("bin").join(arch_dir).join("signtool.exe"),
            ];
            for candidate in unversioned {
                if candidate.is_file() {
                    return Some(candidate);
                }
            }

            let versions_dir = kits.join("10").join("bin");
            if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                let mut versions: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
                versions.sort();
                for version in versions.into_iter().rev() {
                    let candidate = version.join(arch_dir).join("signtool.exe");
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }

        None
    }

    // Falls back to whatever's on PATH — e.g. a "Developer Command Prompt"
    // environment, or a machine where signtool was added manually.
    fn find_on_path() -> Option<PathBuf> {
        let output = Command::new("where.exe")
            .arg("signtool.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .map(|line| PathBuf::from(line.trim()))
            .filter(|path| path.is_file())
    }

    // The detection sweep itself is cheap (a handful of directory reads and
    // stat calls, plus a `where.exe` spawn), but it's still blocking I/O —
    // offloaded the same way as everything else here so a slow disk or PATH
    // lookup can't stall the shared async worker pool other invokes rely on.
    #[tauri::command]
    pub async fn find_signtool(custom_path: Option<String>) -> Result<SigntoolStatus, String> {
        tauri::async_runtime::spawn_blocking(move || {
            if let Some(custom) = custom_path {
                let custom = custom.trim();
                if !custom.is_empty() {
                    return SigntoolStatus {
                        found: Path::new(custom).is_file(),
                        path: Some(custom.to_string()),
                    };
                }
            }

            let found = find_in_kits().or_else(find_on_path);
            SigntoolStatus {
                found: found.is_some(),
                path: found.map(|p| p.to_string_lossy().into_owned()),
            }
        })
        .await
        .map_err(|err| format!("signtool detection task failed to run: {err}"))
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CodeSigningCertificate {
        pub thumbprint: String,
        pub subject: String,
        pub issuer: String,
        pub not_after: String,
        pub has_private_key: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawCertificate {
        thumbprint: String,
        subject: String,
        issuer: String,
        not_after: String,
        has_private_key: bool,
    }

    // Lists certificates from the current user's personal store that are
    // usable for code signing: they need a private key to sign with at all,
    // and either no declared Enhanced Key Usage restriction or an explicit
    // Code Signing EKU (OID 1.3.6.1.5.5.7.3.3).
    // Uses the fully-qualified provider path rather than the `Cert:` drive
    // shortcut — that drive is only auto-mounted in interactive sessions, so
    // a fresh `-NoProfile -NonInteractive` process (what `run_powershell`
    // spawns) can hit "Cannot find drive. A drive with the name 'Cert' does
    // not exist." The qualified form loads the provider module directly.
    const LIST_CERTIFICATES_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$certs = Get-ChildItem 'Microsoft.PowerShell.Security\Certificate::CurrentUser\My' | Where-Object {
    $_.HasPrivateKey -and (
        $_.EnhancedKeyUsageList.Count -eq 0 -or
        ($_.EnhancedKeyUsageList | Where-Object { $_.ObjectId -eq '1.3.6.1.5.5.7.3.3' })
    )
}
@($certs | Select-Object Thumbprint, Subject, Issuer,
    @{Name='NotAfter';Expression={ $_.NotAfter.ToString('yyyy-MM-dd') }},
    @{Name='HasPrivateKey';Expression={ [bool]$_.HasPrivateKey }}
) | ConvertTo-Json -Depth 3 -Compress
"#;

    #[tauri::command]
    pub async fn list_code_signing_certificates() -> Result<Vec<CodeSigningCertificate>, String> {
        let script = format!("{}{LIST_CERTIFICATES_SCRIPT}", crate::ENSURE_CERT_DRIVE);
        let trimmed = run_powershell(&script, &[]).await?;
        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let raw: Vec<RawCertificate> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(raw
            .into_iter()
            .map(|r| CodeSigningCertificate {
                thumbprint: r.thumbprint,
                subject: r.subject,
                issuer: r.issuer,
                not_after: r.not_after,
                has_private_key: r.has_private_key,
            })
            .collect())
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SigntoolOutput {
        pub success: bool,
        pub output: String,
    }

    // Actually runs signtool.exe and waits for it to exit — signing can take
    // a while (hashing a large installer, waiting on a slow timestamp
    // server), so this always goes through `spawn_blocking` via the async
    // wrapper below rather than blocking a shared async worker thread.
    fn run_signtool_blocking(signtool_path: &str, args: &[String]) -> Result<SigntoolOutput, String> {
        let output = Command::new(signtool_path)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|err| format!("failed to run signtool: {err}"))?;

        let mut combined = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&stderr);
        }

        Ok(SigntoolOutput {
            success: output.status.success(),
            output: combined,
        })
    }

    // Runs signtool.exe with the given argv and reports its exit status
    // alongside whatever it printed — signtool's own stdout/stderr *is* the
    // useful diagnostic (which timestamp server failed, why a cert wasn't
    // trusted, etc.), so a non-zero exit is surfaced as `success: false`
    // with that output rather than as an `Err`, the same way ping/traceroute
    // return a result to render instead of failing the call.
    async fn run_signtool(signtool_path: String, args: Vec<String>) -> Result<SigntoolOutput, String> {
        tauri::async_runtime::spawn_blocking(move || run_signtool_blocking(&signtool_path, &args))
            .await
            .map_err(|err| format!("signtool task failed to run: {err}"))?
    }

    fn require_signtool(signtool_path: &str) -> Result<(), String> {
        if signtool_path.trim().is_empty() || !Path::new(signtool_path.trim()).is_file() {
            return Err(
                "signtool.exe wasn't found. Locate it manually first.".to_string(),
            );
        }
        Ok(())
    }

    fn non_empty(value: Option<String>) -> Option<String> {
        value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    }

    #[tauri::command]
    pub async fn sign_file(
        signtool_path: String,
        file_path: String,
        thumbprint: Option<String>,
        pfx_path: Option<String>,
        pfx_password: Option<String>,
        digest_algorithm: String,
        timestamp_url: Option<String>,
        description: Option<String>,
    ) -> Result<SigntoolOutput, String> {
        require_signtool(&signtool_path)?;

        let file_path = file_path.trim();
        if file_path.is_empty() {
            return Err("Choose a file to sign.".to_string());
        }

        let digest_algorithm = non_empty(Some(digest_algorithm)).unwrap_or_else(|| "SHA256".to_string());
        let thumbprint = non_empty(thumbprint);
        let pfx_path = non_empty(pfx_path);

        let mut args = vec!["sign".to_string(), "/fd".to_string(), digest_algorithm.clone()];

        if let Some(thumb) = thumbprint {
            args.push("/sha1".to_string());
            args.push(thumb);
        } else if let Some(pfx) = pfx_path {
            args.push("/f".to_string());
            args.push(pfx);
            if let Some(password) = non_empty(pfx_password) {
                args.push("/p".to_string());
                args.push(password);
            }
        } else {
            return Err("Choose a certificate or a PFX file to sign with.".to_string());
        }

        if let Some(url) = non_empty(timestamp_url) {
            args.push("/tr".to_string());
            args.push(url);
            args.push("/td".to_string());
            args.push(digest_algorithm);
        }

        if let Some(desc) = non_empty(description) {
            args.push("/d".to_string());
            args.push(desc);
        }

        args.push(file_path.to_string());

        run_signtool(signtool_path.trim().to_string(), args).await
    }

    #[tauri::command]
    pub async fn verify_file(signtool_path: String, file_path: String) -> Result<SigntoolOutput, String> {
        require_signtool(&signtool_path)?;

        let file_path = file_path.trim();
        if file_path.is_empty() {
            return Err("Choose a file to verify.".to_string());
        }

        run_signtool(
            signtool_path.trim().to_string(),
            vec![
                "verify".to_string(),
                "/pa".to_string(),
                "/v".to_string(),
                file_path.to_string(),
            ],
        )
        .await
    }
}

// Backs the "Hosts File" feature: `%WINDIR%\System32\drivers\etc\hosts` has
// no GUI anywhere in Windows — this is the classic Notepad-as-admin editing
// experience, just structured. Reading is unelevated (any account can read
// it); every write goes through `run_elevated` since the file itself is
// ACL'd to Administrators.
mod hosts {
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    use crate::run_elevated;

    fn hosts_path() -> PathBuf {
        let system_root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
        PathBuf::from(system_root)
            .join("System32")
            .join("drivers")
            .join("etc")
            .join("hosts")
    }

    fn looks_like_ip(token: &str) -> bool {
        if token.contains(':') {
            return token.chars().all(|c| c.is_ascii_hexdigit() || c == ':');
        }
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 4
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    }

    // Parses "<ip> <hostname...> [# comment]" out of an entry line's content
    // (already stripped of any leading `#` used to mark it disabled).
    fn parse_entry(content: &str) -> Option<(String, Vec<String>, Option<String>)> {
        let mut tokens = content.split_whitespace();
        let ip = tokens.next()?;
        if !looks_like_ip(ip) {
            return None;
        }

        let rest: Vec<&str> = tokens.collect();
        let mut hostnames = Vec::new();
        let mut comment = None;
        for (i, tok) in rest.iter().enumerate() {
            if let Some(stripped) = tok.strip_prefix('#') {
                let mut parts = vec![stripped.to_string()];
                parts.extend(rest[i + 1..].iter().map(|s| s.to_string()));
                let joined = parts.join(" ").trim().to_string();
                comment = if joined.is_empty() { None } else { Some(joined) };
                break;
            }
            hostnames.push(tok.to_string());
        }

        if hostnames.is_empty() {
            return None;
        }
        Some((ip.to_string(), hostnames, comment))
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HostsEntry {
        pub line_number: usize,
        pub enabled: bool,
        pub ip: String,
        pub hostnames: Vec<String>,
        pub comment: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HostsFile {
        pub raw: String,
        pub entries: Vec<HostsEntry>,
    }

    fn parse_hosts(raw: &str) -> Vec<HostsEntry> {
        raw.lines()
            .enumerate()
            .filter_map(|(line_number, line)| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                let (enabled, content) = match trimmed.strip_prefix('#') {
                    Some(stripped) => (false, stripped.trim()),
                    None => (true, trimmed),
                };
                let (ip, hostnames, comment) = parse_entry(content)?;
                Some(HostsEntry {
                    line_number,
                    enabled,
                    ip,
                    hostnames,
                    comment,
                })
            })
            .collect()
    }

    fn read_hosts_raw() -> Result<String, String> {
        std::fs::read_to_string(hosts_path()).map_err(|err| format!("failed to read hosts file: {err}"))
    }

    #[tauri::command]
    pub async fn get_hosts_entries() -> Result<HostsFile, String> {
        tauri::async_runtime::spawn_blocking(|| {
            let raw = read_hosts_raw()?;
            let entries = parse_hosts(&raw);
            Ok(HostsFile { raw, entries })
        })
        .await
        .map_err(|err| format!("hosts read task failed to run: {err}"))?
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    // Takes the whole desired file content and overwrites the hosts file
    // with it — every mutation below (add/remove/toggle) reads the current
    // content, computes the new content in Rust (where string handling is
    // less error-prone than PowerShell), and hands the result here. ASCII
    // encoding avoids a UTF-8 BOM, which Windows' resolver has historically
    // choked on for this specific file.
    // Replaces the hosts file without ever leaving it half written. The old
    // worker used Set-Content, which empties the file first and then writes,
    // so any failure in between (or an empty request) left the real file
    // empty. This one writes the new text next to the file, checks it, swaps
    // it in, checks the result, and puts the original bytes back if anything
    // after the swap goes wrong. $target is a PowerShell expression so
    // the tests can point it at a scratch file.
    fn set_hosts_worker(target: &str) -> String {
        format!(
            r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
function Same($a, $b) {{ [Convert]::ToBase64String($a) -ceq [Convert]::ToBase64String($b) }}
$original = $null
$touched = $false
$tmp = $null
try {{
    $content = Get-Content -Raw -LiteralPath $InputPath
    if ([string]::IsNullOrWhiteSpace($content)) {{
        throw 'Refusing to write an empty hosts file. Nothing was changed.'
    }}
    $hostsPath = {target}
    # UTF-8 without a byte-order mark: keeps accented comments intact, which
    # ASCII would turn into question marks.
    $bytes = (New-Object System.Text.UTF8Encoding($false)).GetBytes($content)
    if (Test-Path -LiteralPath $hostsPath) {{ $original = [System.IO.File]::ReadAllBytes($hostsPath) }}

    $tmp = $hostsPath + '.zagzig-new'
    [System.IO.File]::WriteAllBytes($tmp, $bytes)
    if (-not (Same ([System.IO.File]::ReadAllBytes($tmp)) $bytes)) {{
        throw 'The new hosts file could not be written completely. Nothing was changed.'
    }}

    $swapped = $false
    if ($original -ne $null) {{
        try {{
            [System.IO.File]::Replace($tmp, $hostsPath, $null)
            $swapped = $true
            $touched = $true
        }} catch {{
            # Some programs hold the file open in a way that blocks replacing
            # it; fall back to writing it in place.
        }}
    }}
    if (-not $swapped) {{
        $touched = $true
        [System.IO.File]::WriteAllBytes($hostsPath, $bytes)
    }}
    if (-not (Same ([System.IO.File]::ReadAllBytes($hostsPath)) $bytes)) {{
        throw 'The hosts file does not contain the new text after writing.'
    }}
    @{{ Success = $true }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    $message = $ex.Message
    if ($touched -and $original -ne $null) {{
        # Only act if the file really changed; a write that was refused up
        # front leaves it exactly as it was.
        $now = $null
        try {{ $now = [System.IO.File]::ReadAllBytes($hostsPath) }} catch {{ }}
        if ($now -eq $null -or -not (Same $now $original)) {{
            try {{
                [System.IO.File]::WriteAllBytes($hostsPath, $original)
                $message += ' The original hosts file was put back.'
            }} catch {{
                $message += ' The original could not be put back, restore it from Backups.'
            }}
        }}
    }}
    @{{ Success = $false; Error = $message }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}} finally {{
    if ($tmp -and (Test-Path -LiteralPath $tmp)) {{ Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }}
}}
"#
        )
    }

    #[cfg(all(test, windows))]
    mod worker_tests {
        use super::*;
        use std::io::Write;
        use std::os::windows::fs::OpenOptionsExt;

        struct Scratch(std::path::PathBuf);
        impl Scratch {
            fn new(name: &str) -> Self {
                let dir = std::env::temp_dir().join(format!("zagzig-hosts-{name}-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&dir);
                std::fs::create_dir_all(&dir).unwrap();
                Scratch(dir)
            }
            fn hosts(&self) -> std::path::PathBuf {
                self.0.join("hosts")
            }
            // Runs the real worker against the scratch hosts file and returns
            // its reply.
            fn run(&self, content: &str) -> serde_json::Value {
                let target = format!("'{}'", self.hosts().display());
                let worker = self.0.join("worker.ps1");
                let input = self.0.join("input.txt");
                let output = self.0.join("output.json");
                std::fs::write(&worker, format!("﻿{}", set_hosts_worker(&target))).unwrap();
                std::fs::write(&input, format!("﻿{content}")).unwrap();
                let status = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                    .arg(&worker)
                    .arg("-InputPath")
                    .arg(&input)
                    .arg("-OutputPath")
                    .arg(&output)
                    .output()
                    .unwrap();
                let reply = std::fs::read_to_string(&output)
                    .unwrap_or_else(|_| panic!("no reply: {}", String::from_utf8_lossy(&status.stderr)));
                serde_json::from_str(reply.trim_start_matches('﻿').trim()).unwrap()
            }
        }
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        #[test]
        fn replaces_the_file_and_keeps_accents() {
            let s = Scratch::new("ok");
            std::fs::write(s.hosts(), "127.0.0.1 old
").unwrap();
            let reply = s.run("127.0.0.1 localhost # café
10.0.0.1 db
");
            assert_eq!(reply["Success"], true, "{reply}");
            assert_eq!(std::fs::read_to_string(s.hosts()).unwrap(), "127.0.0.1 localhost # café
10.0.0.1 db
");
            assert!(!s.0.join("hosts.zagzig-new").exists(), "the temp file is cleaned up");
        }

        #[test]
        fn creates_the_file_when_it_is_missing() {
            let s = Scratch::new("new");
            let reply = s.run("127.0.0.1 localhost
");
            assert_eq!(reply["Success"], true, "{reply}");
            assert_eq!(std::fs::read_to_string(s.hosts()).unwrap(), "127.0.0.1 localhost
");
        }

        #[test]
        fn an_empty_request_never_empties_the_file() {
            let s = Scratch::new("empty");
            std::fs::write(s.hosts(), "127.0.0.1 keep
").unwrap();
            for blank in ["", "   
"] {
                let reply = s.run(blank);
                assert_eq!(reply["Success"], false, "{reply}");
                assert!(reply["Error"].as_str().unwrap().contains("empty hosts file"));
                assert_eq!(std::fs::read_to_string(s.hosts()).unwrap(), "127.0.0.1 keep
");
            }
        }

        #[test]
        fn a_refused_write_leaves_the_file_as_it_was() {
            let s = Scratch::new("locked");
            std::fs::write(s.hosts(), "127.0.0.1 keep
").unwrap();
            // Held open with no sharing at all: neither the swap nor an in
            // place write can get in.
            let mut lock = std::fs::OpenOptions::new().read(true).write(true).share_mode(0).open(s.hosts()).unwrap();
            let reply = s.run("10.9.9.9 new
");
            assert_eq!(reply["Success"], false, "{reply}");
            lock.flush().unwrap();
            drop(lock);
            assert_eq!(std::fs::read_to_string(s.hosts()).unwrap(), "127.0.0.1 keep
");
            assert!(!s.0.join("hosts.zagzig-new").exists());
        }

        #[test]
        fn a_read_only_file_is_left_untouched_and_the_message_is_honest() {
            let s = Scratch::new("readonly");
            std::fs::write(s.hosts(), "127.0.0.1 keep
").unwrap();
            let mut perms = std::fs::metadata(s.hosts()).unwrap().permissions();
            perms.set_readonly(true);
            std::fs::set_permissions(s.hosts(), perms.clone()).unwrap();
            let reply = s.run("10.9.9.9 new
");
            perms.set_readonly(false);
            std::fs::set_permissions(s.hosts(), perms).unwrap();
            assert_eq!(reply["Success"], false, "{reply}");
            assert!(!reply["Error"].as_str().unwrap().contains("could not be put back"), "{reply}");
            assert_eq!(std::fs::read_to_string(s.hosts()).unwrap(), "127.0.0.1 keep
");
        }

        #[test]
        fn the_worker_parses() {
            assert_eq!(crate::powershell_syntax_errors(&set_hosts_worker(r"'C:\x\hosts'")), Vec::<String>::new());
        }
    }

    // Every write goes through here, so every write is preceded by a backup
    // of what's about to be replaced (see the backup section below). A
    // backup that can't be made doesn't block the edit — the user asked for
    // it, and the file is theirs — but it's the common case that it works.
    async fn write_hosts_raw(app: &tauri::AppHandle, reason: &str, content: String) -> Result<(), String> {
        if content.trim().is_empty() {
            return Err("Refusing to write an empty hosts file. Nothing was changed.".to_string());
        }
        let _ = backup_current(app, reason).await;
        let before = read_hosts_raw().ok();
        let result = run_elevated(&set_hosts_worker(r"Join-Path $env:WINDIR 'System32\drivers\etc\hosts'"), &content).await;
        let outcome = result.and_then(|trimmed| {
            let parsed: ElevatedResult = serde_json::from_str(&trimmed)
                .map_err(|err| format!("failed to parse powershell output: {err}"))?;
            if parsed.success {
                Ok(())
            } else {
                Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
            }
        });
        let after = read_hosts_raw().ok();
        log_hosts_write(app, reason, before.as_deref(), after.as_deref(), &outcome);
        // Whatever went wrong, make sure the real file isn't left worse than
        // it was. The worker rolls back on its own; this catches the case
        // where it never got to (PowerShell itself failing) and says so.
        if let Err(err) = &outcome {
            if let (Some(before), Some(now)) = (&before, &after) {
                if now != before && now.trim().is_empty() {
                    return Err(format!(
                        "{err} The hosts file is now empty. Restore it from Backups (Compare / restore)."
                    ));
                }
            }
        }
        outcome
    }

    // A short trail of every write (when, why, sizes before and after, the
    // error if any) in the app's data folder. If a write ever damages the
    // file again, this says what happened. It holds no file content.
    fn log_hosts_write(
        app: &tauri::AppHandle,
        reason: &str,
        before: Option<&str>,
        after: Option<&str>,
        outcome: &Result<(), String>,
    ) {
        use std::io::Write;
        let Ok(dir) = backups_dir(app) else { return };
        let Some(parent) = dir.parent() else { return };
        let path = parent.join("hosts-write.log");
        if std::fs::metadata(&path).map(|m| m.len() > 100_000).unwrap_or(false) {
            let _ = std::fs::remove_file(&path);
        }
        let size = |t: Option<&str>| t.map(|t| t.len().to_string()).unwrap_or_else(|| "unreadable".to_string());
        let line = format!(
            "{} reason={reason} before={} after={} result={}\n",
            now_secs(),
            size(before),
            size(after),
            match outcome {
                Ok(()) => "ok".to_string(),
                Err(err) => format!("error: {}", err.replace('\n', " ")),
            }
        );
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = file.write_all(line.as_bytes());
        }
    }

    #[tauri::command]
    pub async fn add_hosts_entry(
        app: tauri::AppHandle,
        ip: String,
        hostnames: Vec<String>,
        comment: Option<String>,
    ) -> Result<(), String> {
        let ip = ip.trim().to_string();
        let hostnames: Vec<String> = hostnames
            .into_iter()
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
            .collect();
        if ip.is_empty() || hostnames.is_empty() {
            return Err("Enter an IP address and at least one hostname.".to_string());
        }
        if !looks_like_ip(&ip) {
            return Err("That doesn't look like a valid IP address.".to_string());
        }

        let raw = tauri::async_runtime::spawn_blocking(read_hosts_raw)
            .await
            .map_err(|err| format!("hosts read task failed to run: {err}"))??;

        let mut line = format!("{ip}\t{}", hostnames.join(" "));
        if let Some(c) = comment.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
            line.push_str(" # ");
            line.push_str(c);
        }

        let mut new_content = raw;
        if !new_content.is_empty() && !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push_str(&line);
        new_content.push('\n');

        write_hosts_raw(&app, "add", new_content).await
    }

    #[tauri::command]
    pub async fn remove_hosts_entry(app: tauri::AppHandle, line_number: usize) -> Result<(), String> {
        let raw = tauri::async_runtime::spawn_blocking(read_hosts_raw)
            .await
            .map_err(|err| format!("hosts read task failed to run: {err}"))??;

        let mut lines: Vec<&str> = raw.lines().collect();
        if line_number >= lines.len() {
            return Err("That entry no longer exists — refresh and try again.".to_string());
        }
        lines.remove(line_number);
        let new_content = if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        };

        write_hosts_raw(&app, "remove", new_content).await
    }

    #[tauri::command]
    pub async fn set_hosts_entry_enabled(
        app: tauri::AppHandle,
        line_number: usize,
        enabled: bool,
    ) -> Result<(), String> {
        let raw = tauri::async_runtime::spawn_blocking(read_hosts_raw)
            .await
            .map_err(|err| format!("hosts read task failed to run: {err}"))??;

        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
        let Some(line) = lines.get_mut(line_number) else {
            return Err("That entry no longer exists — refresh and try again.".to_string());
        };

        let trimmed_start = line.trim_start();
        let indent_len = line.len() - trimmed_start.len();
        let indent = line[..indent_len].to_string();

        *line = if enabled {
            format!(
                "{indent}{}",
                trimmed_start.trim_start_matches('#').trim_start()
            )
        } else if trimmed_start.starts_with('#') {
            line.clone()
        } else {
            format!("{indent}# {trimmed_start}")
        };

        let new_content = format!("{}\n", lines.join("\n"));
        write_hosts_raw(&app, "toggle", new_content).await
    }

    // Moves the entry at `line_number` so it lands where the entry at
    // `target_line_number` currently is: above it when dragged upwards, below
    // it when dragged downwards. Either way that's index `target` once the
    // moved line has been taken out.
    #[tauri::command]
    pub async fn move_hosts_entry(
        app: tauri::AppHandle,
        line_number: usize,
        target_line_number: usize,
    ) -> Result<(), String> {
        if line_number == target_line_number {
            return Ok(());
        }
        let raw = tauri::async_runtime::spawn_blocking(read_hosts_raw)
            .await
            .map_err(|err| format!("hosts read task failed to run: {err}"))??;

        let mut lines: Vec<&str> = raw.lines().collect();
        if line_number >= lines.len() || target_line_number >= lines.len() {
            return Err("That entry no longer exists — refresh and try again.".to_string());
        }
        let moved = lines.remove(line_number);
        lines.insert(target_line_number, moved);

        write_hosts_raw(&app, "move", format!("{}\n", lines.join("\n"))).await
    }

    #[tauri::command]
    pub async fn set_hosts_raw(app: tauri::AppHandle, content: String) -> Result<(), String> {
        write_hosts_raw(&app, "raw", content).await
    }

    // --- Backups ---------------------------------------------------------
    //
    // A copy of the hosts file is saved before every change (unless it's
    // identical to the newest copy), plus on demand, in the app's data
    // folder. The newest 30 are kept. Restoring one is itself a change, so
    // it takes a backup of what it replaces first.

    const BACKUP_LIMIT: usize = 30;
    const BACKUP_REASONS: [&str; 7] = ["add", "remove", "toggle", "move", "raw", "manual", "restore"];

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct HostsBackup {
        pub file: String,
        /// Seconds since the Unix epoch.
        pub time: u64,
        /// What was about to happen when it was taken.
        pub reason: String,
        pub size: u64,
        pub lines: usize,
    }

    fn backups_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        use tauri::Manager;
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("couldn't find the app data folder: {err}"))?
            .join("hosts-backups");
        std::fs::create_dir_all(&dir).map_err(|err| format!("couldn't create the backups folder: {err}"))?;
        Ok(dir)
    }

    // "hosts-<seconds>-<reason>.txt"
    fn parse_backup_name(name: &str) -> Option<(u64, String)> {
        let middle = name.strip_prefix("hosts-")?.strip_suffix(".txt")?;
        let (secs, reason) = middle.split_once('-')?;
        if !BACKUP_REASONS.contains(&reason) {
            return None;
        }
        Some((secs.parse().ok()?, reason.to_string()))
    }

    fn list_backups_in(dir: &std::path::Path) -> Vec<HostsBackup> {
        let mut backups: Vec<HostsBackup> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let file = entry.file_name().to_string_lossy().into_owned();
                let (time, reason) = parse_backup_name(&file)?;
                let size = entry.metadata().ok()?.len();
                let lines = std::fs::read_to_string(entry.path()).map(|t| t.lines().count()).unwrap_or(0);
                Some(HostsBackup { file, time, reason, size, lines })
            })
            .collect();
        backups.sort_by(|a, b| b.time.cmp(&a.time).then_with(|| b.file.cmp(&a.file)));
        backups
    }

    // Saves `raw` unless it's identical to the newest backup. Returns the
    // new file's name, or `None` if nothing was written.
    fn save_backup(dir: &std::path::Path, raw: &str, reason: &str, now: u64) -> Result<Option<String>, String> {
        if !BACKUP_REASONS.contains(&reason) {
            return Err("Unknown backup reason.".to_string());
        }
        let existing = list_backups_in(dir);
        if let Some(newest) = existing.first() {
            if std::fs::read_to_string(dir.join(&newest.file)).is_ok_and(|t| t == raw) {
                return Ok(None);
            }
        }
        let file = format!("hosts-{now}-{reason}.txt");
        std::fs::write(dir.join(&file), raw).map_err(|err| format!("couldn't save the backup: {err}"))?;
        for old in list_backups_in(dir).into_iter().skip(BACKUP_LIMIT) {
            let _ = std::fs::remove_file(dir.join(old.file));
        }
        Ok(Some(file))
    }

    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    async fn backup_current(app: &tauri::AppHandle, reason: &str) -> Result<Option<String>, String> {
        let dir = backups_dir(app)?;
        let reason = reason.to_string();
        tauri::async_runtime::spawn_blocking(move || {
            let raw = read_hosts_raw()?;
            save_backup(&dir, &raw, &reason, now_secs())
        })
        .await
        .map_err(|err| format!("backup task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn list_hosts_backups(app: tauri::AppHandle) -> Result<Vec<HostsBackup>, String> {
        let dir = backups_dir(&app)?;
        tauri::async_runtime::spawn_blocking(move || list_backups_in(&dir))
            .await
            .map_err(|err| format!("backup task failed to run: {err}"))
    }

    /// Backs the file up now. `false` if it's identical to the newest backup.
    #[tauri::command]
    pub async fn create_hosts_backup(app: tauri::AppHandle) -> Result<bool, String> {
        backup_current(&app, "manual").await.map(|saved| saved.is_some())
    }

    fn backup_path(app: &tauri::AppHandle, file: &str) -> Result<PathBuf, String> {
        if parse_backup_name(file).is_none() {
            return Err("That isn't one of this app's backups.".to_string());
        }
        Ok(backups_dir(app)?.join(file))
    }

    #[tauri::command]
    pub async fn read_hosts_backup(app: tauri::AppHandle, file: String) -> Result<String, String> {
        let path = backup_path(&app, &file)?;
        tauri::async_runtime::spawn_blocking(move || {
            std::fs::read_to_string(path).map_err(|err| format!("couldn't read the backup: {err}"))
        })
        .await
        .map_err(|err| format!("backup task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn restore_hosts_backup(app: tauri::AppHandle, file: String) -> Result<(), String> {
        let path = backup_path(&app, &file)?;
        let content = tauri::async_runtime::spawn_blocking(move || {
            std::fs::read_to_string(path).map_err(|err| format!("couldn't read the backup: {err}"))
        })
        .await
        .map_err(|err| format!("backup task failed to run: {err}"))??;
        write_hosts_raw(&app, "restore", content).await
    }

    #[tauri::command]
    pub async fn delete_hosts_backup(app: tauri::AppHandle, file: String) -> Result<(), String> {
        let path = backup_path(&app, &file)?;
        tauri::async_runtime::spawn_blocking(move || match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("couldn't delete the backup: {err}")),
        })
        .await
        .map_err(|err| format!("backup task failed to run: {err}"))?
    }

    #[cfg(test)]
    mod backup_tests {
        use super::*;

        fn temp_dir(tag: &str) -> PathBuf {
            let dir = std::env::temp_dir().join(format!("zagzig-hosts-test-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        #[test]
        fn parses_only_this_apps_backup_names() {
            assert_eq!(parse_backup_name("hosts-1700000000-add.txt"), Some((1_700_000_000, "add".into())));
            assert_eq!(parse_backup_name("hosts-5-restore.txt"), Some((5, "restore".into())));
            for bad in ["hosts", "hosts-1-evil.txt", "hosts-x-add.txt", "hosts-1-add.exe", "../hosts-1-add.txt", "hosts-1-add.txt.bak", ""] {
                assert_eq!(parse_backup_name(bad), None, "{bad}");
            }
        }

        #[test]
        fn skips_a_backup_identical_to_the_newest_and_lists_newest_first() {
            let dir = temp_dir("dedupe");
            assert!(save_backup(&dir, "127.0.0.1 a\n", "manual", 100).unwrap().is_some());
            assert!(save_backup(&dir, "127.0.0.1 a\n", "add", 101).unwrap().is_none(), "unchanged");
            assert!(save_backup(&dir, "127.0.0.1 b\n", "add", 102).unwrap().is_some());
            let list = list_backups_in(&dir);
            assert_eq!(list.iter().map(|b| b.time).collect::<Vec<_>>(), vec![102, 100]);
            assert_eq!(list[0].reason, "add");
            assert_eq!(list[1].lines, 1);
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn keeps_only_the_newest_thirty() {
            let dir = temp_dir("prune");
            for i in 0..(BACKUP_LIMIT as u64 + 6) {
                save_backup(&dir, &format!("# version {i}\n"), "raw", 1000 + i).unwrap();
            }
            let list = list_backups_in(&dir);
            assert_eq!(list.len(), BACKUP_LIMIT);
            assert_eq!(list[0].time, 1000 + BACKUP_LIMIT as u64 + 5);
            assert_eq!(list.last().unwrap().time, 1006, "the six oldest were pruned");
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn rejects_an_unknown_reason() {
            let dir = temp_dir("reason");
            assert!(save_backup(&dir, "x", "../../evil", 1).is_err());
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

// Backs the "SSH Config" feature: `~/.ssh/config` holds the `Host` aliases
// OpenSSH expands before connecting. It's owned by the current user, so —
// unlike the Windows hosts file — nothing here needs elevation. A `Host`
// block runs from its `Host` line up to the next `Host`/`Match` line; edits
// only touch the keys the UI knows about and leave every other line alone.
mod ssh {
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    fn config_path() -> Result<PathBuf, String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map_err(|_| "couldn't determine the user profile directory".to_string())?;
        Ok(PathBuf::from(home).join(".ssh").join("config"))
    }

    fn read_config_raw() -> Result<String, String> {
        let path = config_path()?;
        match std::fs::read_to_string(&path) {
            Ok(raw) => Ok(raw),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(err) => Err(format!("failed to read ssh config: {err}")),
        }
    }

    fn write_config_raw(content: &str) -> Result<(), String> {
        let path = config_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|err| format!("failed to create .ssh directory: {err}"))?;
        }
        std::fs::write(&path, content).map_err(|err| format!("failed to write ssh config: {err}"))
    }

    async fn read_blocking() -> Result<String, String> {
        tauri::async_runtime::spawn_blocking(read_config_raw)
            .await
            .map_err(|err| format!("ssh config read task failed to run: {err}"))?
    }

    async fn write_blocking(content: String) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || write_config_raw(&content))
            .await
            .map_err(|err| format!("ssh config write task failed to run: {err}"))?
    }

    // Splits "Key value", "Key=value" or "Key = value" into (key, value),
    // with surrounding quotes stripped from the value.
    fn split_directive(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        let idx = trimmed.find(|c: char| c.is_whitespace() || c == '=')?;
        let key = &trimmed[..idx];
        let value = trimmed[idx..]
            .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
            .trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        Some((key.to_string(), value.to_string()))
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SshHost {
        pub line_number: usize,
        pub patterns: Vec<String>,
        pub host_name: Option<String>,
        pub user: Option<String>,
        pub port: Option<String>,
        pub identity_file: Option<String>,
        pub proxy_jump: Option<String>,
        pub other_options: usize,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SshConfig {
        pub raw: String,
        pub hosts: Vec<SshHost>,
    }

    // Returns (start, end) line ranges, end exclusive, of each `Host` block.
    fn host_blocks(lines: &[&str]) -> Vec<(usize, usize)> {
        let mut blocks = Vec::new();
        let mut current: Option<usize> = None;
        for (i, line) in lines.iter().enumerate() {
            let keyword = split_directive(line).map(|(k, _)| k.to_ascii_lowercase());
            match keyword.as_deref() {
                Some("host") => {
                    if let Some(start) = current.take() {
                        blocks.push((start, i));
                    }
                    current = Some(i);
                }
                Some("match") => {
                    if let Some(start) = current.take() {
                        blocks.push((start, i));
                    }
                }
                _ => {}
            }
        }
        if let Some(start) = current {
            blocks.push((start, lines.len()));
        }
        blocks
    }

    fn parse_hosts(raw: &str) -> Vec<SshHost> {
        let lines: Vec<&str> = raw.lines().collect();
        host_blocks(&lines)
            .into_iter()
            .map(|(start, end)| {
                let patterns = split_directive(lines[start])
                    .map(|(_, v)| v.split_whitespace().map(str::to_string).collect())
                    .unwrap_or_default();
                let mut host = SshHost {
                    line_number: start,
                    patterns,
                    host_name: None,
                    user: None,
                    port: None,
                    identity_file: None,
                    proxy_jump: None,
                    other_options: 0,
                };
                for line in &lines[start + 1..end] {
                    let Some((key, value)) = split_directive(line) else {
                        continue;
                    };
                    let slot = match key.to_ascii_lowercase().as_str() {
                        "hostname" => &mut host.host_name,
                        "user" => &mut host.user,
                        "port" => &mut host.port,
                        "identityfile" => &mut host.identity_file,
                        "proxyjump" => &mut host.proxy_jump,
                        _ => {
                            host.other_options += 1;
                            continue;
                        }
                    };
                    if slot.is_none() {
                        *slot = Some(value);
                    }
                }
                host
            })
            .collect()
    }

    #[tauri::command]
    pub async fn get_ssh_hosts() -> Result<SshConfig, String> {
        let raw = read_blocking().await?;
        let hosts = parse_hosts(&raw);
        Ok(SshConfig { raw, hosts })
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SshHostInput {
        pub patterns: String,
        pub host_name: Option<String>,
        pub user: Option<String>,
        pub port: Option<String>,
        pub identity_file: Option<String>,
        pub proxy_jump: Option<String>,
    }

    fn clean(value: &Option<String>, field: &str, allow_spaces: bool) -> Result<Option<String>, String> {
        let Some(v) = value.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
            return Ok(None);
        };
        // Newlines would let a value smuggle in extra directives (e.g. a
        // ProxyCommand), so control characters are never allowed.
        if v.chars().any(char::is_control) || v.contains('"') {
            return Err(format!("{field} contains characters that aren't allowed."));
        }
        if !allow_spaces && v.chars().any(char::is_whitespace) {
            return Err(format!("{field} can't contain spaces."));
        }
        Ok(Some(v.to_string()))
    }

    // Index order matches MANAGED_KEYS.
    const MANAGED_KEYS: [&str; 5] = ["HostName", "User", "Port", "IdentityFile", "ProxyJump"];

    fn managed_values(input: &SshHostInput) -> Result<(String, [Option<String>; 5]), String> {
        let patterns = input.patterns.split_whitespace().collect::<Vec<_>>();
        if patterns.is_empty() {
            return Err("Enter a host alias.".to_string());
        }
        if input.patterns.chars().any(char::is_control) || input.patterns.contains('"') {
            return Err("The host alias contains characters that aren't allowed.".to_string());
        }
        let port = clean(&input.port, "Port", false)?;
        if let Some(p) = &port {
            if p.parse::<u16>().map_or(true, |n| n == 0) {
                return Err("Port must be a number between 1 and 65535.".to_string());
            }
        }
        let identity = clean(&input.identity_file, "Identity file", true)?.map(|v| {
            if v.contains(char::is_whitespace) {
                format!("\"{v}\"")
            } else {
                v
            }
        });
        Ok((
            patterns.join(" "),
            [
                clean(&input.host_name, "Hostname", false)?,
                clean(&input.user, "User", false)?,
                port,
                identity,
                clean(&input.proxy_jump, "Proxy jump", false)?,
            ],
        ))
    }

    fn join_lines(lines: &[String]) -> String {
        if lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", lines.join("\n"))
        }
    }

    fn normalized_patterns(line: &str) -> String {
        split_directive(line)
            .map(|(_, v)| v.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default()
    }

    // Finds the block that starts at `line_number`, verifying it's still the
    // same host the UI was showing (the file may have changed since).
    fn find_block(
        lines: &[&str],
        line_number: usize,
        original_patterns: &str,
    ) -> Result<(usize, usize), String> {
        let stale = || "That host changed on disk — refresh and try again.".to_string();
        let (start, end) = host_blocks(lines)
            .into_iter()
            .find(|(start, _)| *start == line_number)
            .ok_or_else(stale)?;
        if normalized_patterns(lines[start]) != original_patterns {
            return Err(stale());
        }
        Ok((start, end))
    }

    #[tauri::command]
    pub async fn add_ssh_host(input: SshHostInput) -> Result<(), String> {
        let (patterns, values) = managed_values(&input)?;
        let raw = read_blocking().await?;

        if parse_hosts(&raw)
            .iter()
            .any(|h| h.patterns.join(" ").eq_ignore_ascii_case(&patterns))
        {
            return Err("A host with that alias already exists.".to_string());
        }

        let mut out = raw;
        if !out.is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
        out.push_str(&format!("Host {patterns}\n"));
        for (key, value) in MANAGED_KEYS.iter().zip(values) {
            if let Some(v) = value {
                out.push_str(&format!("    {key} {v}\n"));
            }
        }
        write_blocking(out).await
    }

    #[tauri::command]
    pub async fn update_ssh_host(
        line_number: usize,
        original_patterns: String,
        input: SshHostInput,
    ) -> Result<(), String> {
        let (patterns, values) = managed_values(&input)?;
        let raw = read_blocking().await?;
        let lines: Vec<&str> = raw.lines().collect();
        let (start, end) = find_block(&lines, line_number, &original_patterns)?;

        let indent = lines[start + 1..end]
            .iter()
            .find(|l| split_directive(l).is_some())
            .map(|l| l[..l.len() - l.trim_start().len()].to_string())
            .unwrap_or_else(|| "    ".to_string());

        let mut out: Vec<String> = lines[..start].iter().map(|l| l.to_string()).collect();
        out.push(format!("Host {patterns}"));

        // Existing lines for managed keys are rewritten in place (extra
        // duplicates dropped); unmanaged lines and comments pass through.
        let mut written = [false; 5];
        for line in &lines[start + 1..end] {
            let slot = split_directive(line).and_then(|(k, _)| {
                MANAGED_KEYS
                    .iter()
                    .position(|m| m.eq_ignore_ascii_case(&k))
            });
            match slot {
                Some(i) => {
                    if written[i] {
                        continue;
                    }
                    written[i] = true;
                    if let Some(v) = &values[i] {
                        out.push(format!("{indent}{} {v}", MANAGED_KEYS[i]));
                    }
                }
                None => out.push(line.to_string()),
            }
        }

        // Newly set keys go right after the existing content of the block,
        // above any trailing blank separator lines.
        let mut insert_at = out.len();
        while insert_at > start + 1 && out[insert_at - 1].trim().is_empty() {
            insert_at -= 1;
        }
        let mut inserted = 0;
        for (i, value) in values.iter().enumerate() {
            if written[i] {
                continue;
            }
            if let Some(v) = value {
                out.insert(insert_at + inserted, format!("{indent}{} {v}", MANAGED_KEYS[i]));
                inserted += 1;
            }
        }

        out.extend(lines[end..].iter().map(|l| l.to_string()));
        write_blocking(join_lines(&out)).await
    }

    #[tauri::command]
    pub async fn remove_ssh_host(line_number: usize, original_patterns: String) -> Result<(), String> {
        let raw = read_blocking().await?;
        let lines: Vec<&str> = raw.lines().collect();
        let (start, mut end) = find_block(&lines, line_number, &original_patterns)?;

        // Keep the blank separator after the block with whatever follows,
        // rather than deleting it along with the block.
        while end > start + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
        }
        let out: Vec<String> = lines[..start]
            .iter()
            .chain(lines[end..].iter())
            .map(|l| l.to_string())
            .collect();
        write_blocking(join_lines(&out)).await
    }

    #[tauri::command]
    pub async fn set_ssh_config_raw(content: String) -> Result<(), String> {
        write_blocking(content).await
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const SAMPLE: &str = "# my hosts\nHost prod\n    HostName 10.0.0.1\n    User deploy\n    ForwardAgent yes\n\nHost *\n    ServerAliveInterval 30\n";

        #[test]
        fn parses_blocks() {
            let hosts = parse_hosts(SAMPLE);
            assert_eq!(hosts.len(), 2);
            assert_eq!(hosts[0].patterns, vec!["prod"]);
            assert_eq!(hosts[0].host_name.as_deref(), Some("10.0.0.1"));
            assert_eq!(hosts[0].other_options, 1);
            assert_eq!(hosts[1].patterns, vec!["*"]);
        }

        #[test]
        fn parses_equals_and_quotes() {
            let hosts = parse_hosts("Host a\n  IdentityFile = \"C:/my keys/id\"\n");
            assert_eq!(hosts[0].identity_file.as_deref(), Some("C:/my keys/id"));
        }

        #[test]
        fn rejects_newline_injection() {
            let input = SshHostInput {
                patterns: "x".into(),
                host_name: Some("a\nProxyCommand evil".into()),
                user: None,
                port: None,
                identity_file: None,
                proxy_jump: None,
            };
            assert!(managed_values(&input).is_err());
        }
    }
}

// Backs the "WSL" feature: lists distributions, terminates one or shuts the
// whole WSL VM down (which is also how WSL "restarts" — it comes back up
// lazily on next use, and that's when a changed `.wslconfig` takes effect),
// and edits `%USERPROFILE%\.wslconfig`. All of it runs as the current user;
// nothing needs elevation. `wsl.exe` writes its list output as UTF-16LE, so
// it's spawned directly (not through PowerShell) and decoded here.
mod wsl {
    use std::path::PathBuf;
    use std::time::Duration;

    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, CREATE_NO_WINDOW};

    struct WslOutput {
        success: bool,
        stdout: String,
        stderr: String,
    }

    fn decode(bytes: &[u8]) -> String {
        // wsl.exe emits UTF-16LE for most of its own messages, but passes
        // through the distro's UTF-8 for anything it runs inside one. A NUL
        // byte in the first few bytes is the tell for UTF-16.
        if bytes.len() >= 2 && bytes.iter().take(8).any(|b| *b == 0) {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        }
        .trim_start_matches('\u{feff}')
        .to_string()
    }

    const TIMED_OUT: &str = "timed out";

    // A broken WSL is exactly when `wsl.exe` hangs forever (the service
    // never answers), so every call gets a deadline: on expiry the process
    // is killed and a `TIMED_OUT` error comes back instead of blocking the
    // worker thread — and the UI — indefinitely.
    fn run_wsl_blocking(args: &[String], timeout: Duration) -> Result<WslOutput, String> {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};

        let child = Command::new("wsl.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| format!("failed to run wsl.exe: {err}"))?;
        let pid = child.id();

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait_with_output());
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(output)) => Ok(WslOutput {
                success: output.status.success(),
                stdout: decode(&output.stdout),
                stderr: decode(&output.stderr),
            }),
            Ok(Err(err)) => Err(format!("failed to run wsl.exe: {err}")),
            Err(_) => {
                let _ = Command::new("taskkill")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["/PID", &pid.to_string(), "/F"])
                    .output();
                Err(TIMED_OUT.to_string())
            }
        }
    }

    async fn run_wsl_with(args: &[&str], timeout: Duration) -> Result<WslOutput, String> {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        tauri::async_runtime::spawn_blocking(move || run_wsl_blocking(&args, timeout))
            .await
            .map_err(|err| format!("wsl task failed to run: {err}"))?
    }

    async fn run_wsl(args: &[&str]) -> Result<WslOutput, String> {
        run_wsl_with(args, Duration::from_secs(20)).await
    }

    fn failure_message(out: &WslOutput) -> String {
        let text = if out.stderr.trim().is_empty() { &out.stdout } else { &out.stderr };
        let text = text.replace('\0', "").trim().to_string();
        if text.is_empty() {
            "wsl.exe reported an error.".to_string()
        } else {
            text
        }
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WslDistro {
        pub name: String,
        pub running: bool,
        pub version: String,
        pub is_default: bool,
    }

    // Parses `wsl -l -v`. The header row is localized, so rows are read by
    // position instead: last token is the version, the one before is the
    // (localized) state, a leading `*` marks the default, and the rest is
    // the name.
    fn parse_distros(listing: &str, running: &[String]) -> Vec<WslDistro> {
        listing
            .lines()
            .skip(1)
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let (is_default, rest) = match line.strip_prefix('*') {
                    Some(rest) => (true, rest.trim()),
                    None => (false, line),
                };
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                if tokens.len() < 3 {
                    return None;
                }
                let version = tokens[tokens.len() - 1].to_string();
                let name = tokens[..tokens.len() - 2].join(" ");
                Some(WslDistro {
                    running: running.iter().any(|r| r == &name),
                    name,
                    version,
                    is_default,
                })
            })
            .collect()
    }

    fn config_path() -> Result<PathBuf, String> {
        let home = std::env::var("USERPROFILE")
            .map_err(|_| "couldn't determine the user profile directory".to_string())?;
        Ok(PathBuf::from(home).join(".wslconfig"))
    }

    fn read_config_raw() -> Result<String, String> {
        match std::fs::read_to_string(config_path()?) {
            Ok(raw) => Ok(raw),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(err) => Err(format!("failed to read .wslconfig: {err}")),
        }
    }

    async fn read_config_blocking() -> Result<String, String> {
        tauri::async_runtime::spawn_blocking(read_config_raw)
            .await
            .map_err(|err| format!("wslconfig read task failed to run: {err}"))?
    }

    async fn write_config_blocking(content: String) -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(move || {
            std::fs::write(config_path()?, content).map_err(|err| format!("failed to write .wslconfig: {err}"))
        })
        .await
        .map_err(|err| format!("wslconfig write task failed to run: {err}"))?
    }

    // The `[wsl2]` keys the UI manages, in display order.
    const MANAGED_KEYS: [&str; 7] = [
        "memory",
        "processors",
        "swap",
        "localhostForwarding",
        "networkingMode",
        "autoMemoryReclaim",
        "nestedVirtualization",
    ];

    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WslSettings {
        pub memory: Option<String>,
        pub processors: Option<String>,
        pub swap: Option<String>,
        pub localhost_forwarding: Option<String>,
        pub networking_mode: Option<String>,
        pub auto_memory_reclaim: Option<String>,
        pub nested_virtualization: Option<String>,
    }

    impl WslSettings {
        fn as_array(&self) -> [&Option<String>; 7] {
            [
                &self.memory,
                &self.processors,
                &self.swap,
                &self.localhost_forwarding,
                &self.networking_mode,
                &self.auto_memory_reclaim,
                &self.nested_virtualization,
            ]
        }
    }

    // Returns the section name for a `[section]` line.
    fn section_name(line: &str) -> Option<String> {
        let t = line.trim();
        let inner = t.strip_prefix('[')?.strip_suffix(']')?;
        Some(inner.trim().to_ascii_lowercase())
    }

    fn key_of(line: &str) -> Option<(String, String)> {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t.starts_with('[') {
            return None;
        }
        let (k, v) = t.split_once('=')?;
        Some((k.trim().to_string(), v.trim().to_string()))
    }

    fn parse_settings(raw: &str) -> WslSettings {
        let mut settings = WslSettings::default();
        let mut in_wsl2 = false;
        for line in raw.lines() {
            if let Some(name) = section_name(line) {
                in_wsl2 = name == "wsl2";
                continue;
            }
            if !in_wsl2 {
                continue;
            }
            let Some((key, value)) = key_of(line) else { continue };
            let slot = match key.to_ascii_lowercase().as_str() {
                "memory" => &mut settings.memory,
                "processors" => &mut settings.processors,
                "swap" => &mut settings.swap,
                "localhostforwarding" => &mut settings.localhost_forwarding,
                "networkingmode" => &mut settings.networking_mode,
                "automemoryreclaim" => &mut settings.auto_memory_reclaim,
                "nestedvirtualization" => &mut settings.nested_virtualization,
                _ => continue,
            };
            if slot.is_none() {
                *slot = Some(value);
            }
        }
        settings
    }

    fn is_size(v: &str) -> bool {
        let digits = v.chars().take_while(|c| c.is_ascii_digit()).count();
        let unit = &v[digits..];
        digits > 0 && ["", "B", "KB", "MB", "GB", "TB"].iter().any(|u| unit.eq_ignore_ascii_case(u))
    }

    fn validate(settings: &WslSettings) -> Result<(), String> {
        let bad = |field: &str| Err(format!("Invalid value for {field}."));
        if let Some(v) = &settings.memory {
            if !is_size(v) {
                return bad("memory (use e.g. 8GB)");
            }
        }
        if let Some(v) = &settings.swap {
            if !is_size(v) {
                return bad("swap (use e.g. 4GB, or 0)");
            }
        }
        if let Some(v) = &settings.processors {
            if v.parse::<u32>().map_or(true, |n| n == 0) {
                return bad("processors (a positive number)");
            }
        }
        for (v, field) in [
            (&settings.localhost_forwarding, "localhostForwarding"),
            (&settings.nested_virtualization, "nestedVirtualization"),
        ] {
            if let Some(v) = v {
                if v != "true" && v != "false" {
                    return bad(field);
                }
            }
        }
        if let Some(v) = &settings.networking_mode {
            if !["NAT", "mirrored", "bridged", "virtioproxy", "none"]
                .iter()
                .any(|m| m.eq_ignore_ascii_case(v))
            {
                return bad("networkingMode");
            }
        }
        if let Some(v) = &settings.auto_memory_reclaim {
            if !["disabled", "gradual", "dropcache"].iter().any(|m| m.eq_ignore_ascii_case(v)) {
                return bad("autoMemoryReclaim");
            }
        }
        Ok(())
    }

    // Rewrites the managed keys inside `[wsl2]` (adding the section if
    // missing), keeping every other line, comment and section as it was.
    fn apply_settings(raw: &str, settings: &WslSettings) -> String {
        let values = settings.as_array();
        let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();

        let start = lines
            .iter()
            .position(|l| section_name(l).as_deref() == Some("wsl2"));
        let Some(start) = start else {
            let mut block: Vec<String> = Vec::new();
            for (key, value) in MANAGED_KEYS.iter().zip(values) {
                if let Some(v) = value {
                    block.push(format!("{key}={v}"));
                }
            }
            if block.is_empty() {
                return raw.to_string();
            }
            if !lines.is_empty() && !lines.last().is_some_and(|l| l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push("[wsl2]".to_string());
            lines.extend(block);
            return format!("{}\n", lines.join("\n"));
        };
        let end = lines[start + 1..]
            .iter()
            .position(|l| section_name(l).is_some())
            .map_or(lines.len(), |p| start + 1 + p);

        let mut written = [false; 7];
        let mut out: Vec<String> = lines[..=start].to_vec();
        for line in &lines[start + 1..end] {
            let slot = key_of(line)
                .and_then(|(k, _)| MANAGED_KEYS.iter().position(|m| m.eq_ignore_ascii_case(&k)));
            match slot {
                Some(i) => {
                    if written[i] {
                        continue;
                    }
                    written[i] = true;
                    if let Some(v) = values[i] {
                        out.push(format!("{}={v}", MANAGED_KEYS[i]));
                    }
                }
                None => out.push(line.clone()),
            }
        }
        // New keys go after the last non-blank line of the section.
        let mut insert_at = out.len();
        while insert_at > start + 1 && out[insert_at - 1].trim().is_empty() {
            insert_at -= 1;
        }
        let mut inserted = 0;
        for (i, value) in values.iter().enumerate() {
            if written[i] {
                continue;
            }
            if let Some(v) = value {
                out.insert(insert_at + inserted, format!("{}={v}", MANAGED_KEYS[i]));
                inserted += 1;
            }
        }
        out.extend(lines[end..].iter().cloned());
        format!("{}\n", out.join("\n"))
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WslStatus {
        pub installed: bool,
        // True when wsl.exe didn't answer in time — the "WSL is broken"
        // state the force restart exists for.
        pub unresponsive: bool,
        pub docker_desktop: DockerDesktop,
        pub distros: Vec<WslDistro>,
        pub settings: WslSettings,
        pub raw_config: String,
    }

    #[tauri::command]
    pub async fn get_wsl_status() -> Result<WslStatus, String> {
        let raw_config = read_config_blocking().await?;
        let settings = parse_settings(&raw_config);
        let docker_desktop = docker_state().await;

        let listing = match run_wsl(&["--list", "--verbose"]).await {
            Ok(out) if out.success => out,
            other => {
                // Timed out → WSL is installed but hung. Anything else:
                // wsl.exe missing, or present but with no WSL installed yet.
                let unresponsive = matches!(&other, Err(e) if e == TIMED_OUT);
                return Ok(WslStatus {
                    installed: unresponsive,
                    unresponsive,
                    docker_desktop,
                    distros: Vec::new(),
                    settings,
                    raw_config,
                });
            }
        };
        let running: Vec<String> = match run_wsl(&["--list", "--running", "--quiet"]).await {
            Ok(out) if out.success => out
                .stdout
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            _ => Vec::new(),
        };
        Ok(WslStatus {
            installed: true,
            unresponsive: false,
            docker_desktop,
            distros: parse_distros(&listing.stdout, &running),
            settings,
            raw_config,
        })
    }

    // Only names that `wsl -l` itself reports are passed back to wsl.exe, so
    // a crafted name can't turn into an extra option.
    async fn known_distro(name: &str) -> Result<(), String> {
        let out = run_wsl(&["--list", "--verbose"]).await?;
        if out.success && parse_distros(&out.stdout, &[]).iter().any(|d| d.name == name) {
            Ok(())
        } else {
            Err("That distribution no longer exists — refresh and try again.".to_string())
        }
    }

    async fn run_checked(args: &[&str]) -> Result<(), String> {
        let out = run_wsl(args).await?;
        if out.success {
            Ok(())
        } else {
            Err(failure_message(&out))
        }
    }

    #[tauri::command]
    pub async fn wsl_terminate_distro(name: String) -> Result<(), String> {
        known_distro(&name).await?;
        run_checked(&["--terminate", &name]).await
    }

    #[tauri::command]
    pub async fn wsl_set_default_distro(name: String) -> Result<(), String> {
        known_distro(&name).await?;
        run_checked(&["--set-default", &name]).await
    }

    // ---- Copying the Windows hosts file into a distribution ----------------
    //
    // WSL rebuilds /etc/hosts every time a distribution starts (unless
    // generateHosts is off in /etc/wsl.conf), so a one-off edit disappears at
    // the next start. "Sync now" writes a marked block into /etc/hosts, and
    // "keep synced" installs a small script that /etc/wsl.conf runs at every
    // start to put the block back from the live Windows file.

    const SYNC_BEGIN: &str = "# BEGIN zagzig-tools hosts (managed, do not edit)";
    const SYNC_END: &str = "# END zagzig-tools hosts";
    const BOOT_SCRIPT_PATH: &str = "/usr/local/sbin/zagzig-hosts-sync";
    const MAX_BLOCK_BYTES: usize = 20_000;

    // An IP address, optionally with an IPv6 zone (`fe80::1%eth0`).
    fn safe_address(token: &str) -> bool {
        let (ip, zone) = match token.split_once('%') {
            Some((ip, zone)) => (ip, Some(zone)),
            None => (token, None),
        };
        ip.parse::<std::net::IpAddr>().is_ok()
            && zone.map_or(true, |z| !z.is_empty() && z.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
    }

    fn safe_hostname(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 253
            && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '*'))
    }

    #[derive(Debug, PartialEq, Eq)]
    struct Block {
        text: String,
        entries: usize,
        skipped: usize,
    }

    // The active Windows entries as a block. `localhost` lines are left out
    // (the distribution has its own), and so is anything whose address or
    // names contain characters /etc/hosts wouldn't accept.
    fn build_block(entries: &[(String, Vec<String>)]) -> Result<Block, String> {
        let mut lines = Vec::new();
        let mut skipped = 0;
        for (ip, names) in entries {
            if names.is_empty() || names.iter().all(|n| n.eq_ignore_ascii_case("localhost")) {
                continue;
            }
            if !safe_address(ip) || !names.iter().all(|n| safe_hostname(n)) {
                skipped += 1;
                continue;
            }
            lines.push(format!("{ip} {}", names.join(" ")));
        }
        let mut text = format!("{SYNC_BEGIN}\n");
        for line in &lines {
            text.push_str(line);
            text.push('\n');
        }
        text.push_str(SYNC_END);
        text.push('\n');
        if text.len() > MAX_BLOCK_BYTES {
            return Err("There are too many hosts entries to copy in one go.".to_string());
        }
        Ok(Block { text, entries: lines.len(), skipped })
    }

    // Removes our block. An unterminated one (BEGIN with no END) only loses
    // its BEGIN line, so a damaged file never costs unrelated lines.
    fn strip_block(existing: &str) -> String {
        let lines: Vec<&str> = existing.lines().collect();
        let mut out: Vec<&str> = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            if lines[i].trim_end() == SYNC_BEGIN {
                if let Some(offset) = lines[i + 1..].iter().position(|l| l.trim_end() == SYNC_END) {
                    i += offset + 2;
                    continue;
                }
                i += 1;
                continue;
            }
            out.push(lines[i]);
            i += 1;
        }
        let mut text = out.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        text
    }

    fn merge_block(existing: &str, block: &str) -> String {
        let mut text = strip_block(existing);
        while text.ends_with("\n\n") {
            text.pop();
        }
        text.push_str(block);
        text
    }

    fn block_entry_count(existing: &str) -> Option<usize> {
        let lines: Vec<&str> = existing.lines().collect();
        let start = lines.iter().position(|l| l.trim_end() == SYNC_BEGIN)?;
        let end = lines[start + 1..].iter().position(|l| l.trim_end() == SYNC_END)?;
        Some(lines[start + 1..start + 1 + end].iter().filter(|l| !l.trim().is_empty()).count())
    }

    // /etc/wsl.conf: one `command` under [boot] is all WSL runs, so ours is
    // only added when that slot is free.
    fn boot_command_of(conf: &str) -> Option<String> {
        let mut in_boot = false;
        for line in conf.lines() {
            if let Some(section) = section_name(line) {
                in_boot = section == "boot";
            } else if in_boot {
                if let Some((key, value)) = key_of(line) {
                    if key == "command" {
                        return Some(value.trim().trim_matches('"').to_string());
                    }
                }
            }
        }
        None
    }

    fn set_boot_command(conf: &str) -> Result<String, String> {
        match boot_command_of(conf) {
            Some(existing) if existing == BOOT_SCRIPT_PATH => return Ok(conf.to_string()),
            Some(existing) => {
                return Err(format!(
                    "This distribution already runs another boot command ({existing}), and WSL allows only one. Add {BOOT_SCRIPT_PATH} to it yourself, or remove the other command first."
                ))
            }
            None => {}
        }
        let line = format!("command = {BOOT_SCRIPT_PATH}");
        let mut lines: Vec<String> = conf.lines().map(str::to_string).collect();
        match lines.iter().position(|l| section_name(l).as_deref() == Some("boot")) {
            Some(i) => lines.insert(i + 1, line),
            None => {
                if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                    lines.push(String::new());
                }
                lines.push("[boot]".to_string());
                lines.push(line);
            }
        }
        Ok(format!("{}\n", lines.join("\n")))
    }

    fn remove_boot_command(conf: &str) -> String {
        let mut in_boot = false;
        let kept: Vec<&str> = conf
            .lines()
            .filter(|line| {
                if let Some(section) = section_name(line) {
                    in_boot = section == "boot";
                    return true;
                }
                !(in_boot
                    && key_of(line)
                        .is_some_and(|(k, v)| k == "command" && v.trim().trim_matches('"') == BOOT_SCRIPT_PATH))
            })
            .collect();
        // An emptied [boot] section goes too, with the blank line before it.
        let mut lines: Vec<&str> = kept;
        if let Some(i) = lines.iter().position(|l| section_name(l).as_deref() == Some("boot")) {
            let body = lines[i + 1..].iter().take_while(|l| section_name(l).is_none()).count();
            if lines[i + 1..i + 1 + body].iter().all(|l| l.trim().is_empty()) {
                lines.drain(i..i + 1 + body);
                if i > 0 && lines.get(i - 1).is_some_and(|l| l.trim().is_empty()) {
                    lines.remove(i - 1);
                }
            }
        }
        let text = lines.join("\n");
        if text.trim().is_empty() {
            String::new()
        } else {
            format!("{text}\n")
        }
    }

    fn generate_hosts_enabled(conf: &str) -> bool {
        let mut in_network = false;
        let mut enabled = true;
        for line in conf.lines() {
            if let Some(section) = section_name(line) {
                in_network = section == "network";
            } else if in_network {
                if let Some((key, value)) = key_of(line) {
                    if key == "generateHosts" {
                        enabled = !value.trim().trim_matches('"').eq_ignore_ascii_case("false");
                    }
                }
            }
        }
        enabled
    }

    // `C:\Windows\System32\drivers\etc\hosts` as WSL sees it.
    fn posix_path_of(windows: &str) -> Option<String> {
        let mut chars = windows.chars();
        let drive = chars.next().filter(|c| c.is_ascii_alphabetic())?;
        if chars.next() != Some(':') {
            return None;
        }
        Some(format!("/mnt/{}{}", drive.to_ascii_lowercase(), windows[2..].replace('\\', "/")))
    }

    // Runs at every start of the distribution and rebuilds the block from the
    // live Windows file, with the same rules as `build_block`. It never
    // fails the boot: any problem just leaves /etc/hosts as WSL made it.
    fn boot_script(source: &str) -> String {
        format!(
            r#"#!/bin/sh
# Installed by zagzig-tools. Copies the active entries of the Windows hosts
# file into /etc/hosts. Remove it from the app (Hosts File > WSL) or delete
# the "command" line under [boot] in /etc/wsl.conf.
SRC='{source}'
HOSTS=/etc/hosts
[ -r "$SRC" ] || exit 0
tmp=$(mktemp) || exit 0
awk -v b='{SYNC_BEGIN}' -v e='{SYNC_END}' '
  $0 == b {{ skip = 1; next }}
  skip && $0 == e {{ skip = 0; next }}
  !skip {{ print }}' "$HOSTS" > "$tmp"
{{
  printf '%s\n' '{SYNC_BEGIN}'
  awk '
    {{ sub(/\r$/, ""); sub(/#.*/, "") }}
    NF < 2 {{ next }}
    $1 !~ /^[0-9A-Fa-f:.]+(%[A-Za-z0-9_]+)?$/ {{ next }}
    {{
      all = 1; ok = 1
      for (i = 2; i <= NF; i++) {{
        if (tolower($i) != "localhost") all = 0
        if ($i !~ /^[A-Za-z0-9._*-]+$/) ok = 0
      }}
      if (all || !ok) next
      line = $1
      for (i = 2; i <= NF; i++) line = line " " $i
      print line
    }}' "$SRC"
  printf '%s\n' '{SYNC_END}'
}} >> "$tmp"
[ -s "$tmp" ] && cat "$tmp" > "$HOSTS"
rm -f "$tmp"
exit 0
"#
        )
    }

    // base64 for passing file contents to the distribution as one plain
    // argument (no quoting problems, whatever the text holds).
    fn base64(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let n = (chunk[0] as u32) << 16
                | (*chunk.get(1).unwrap_or(&0) as u32) << 8
                | *chunk.get(2).unwrap_or(&0) as u32;
            out.push(TABLE[(n >> 18) as usize & 63] as char);
            out.push(TABLE[(n >> 12) as usize & 63] as char);
            out.push(if chunk.len() > 1 { TABLE[(n >> 6) as usize & 63] as char } else { '=' });
            out.push(if chunk.len() > 2 { TABLE[n as usize & 63] as char } else { '=' });
        }
        out
    }

    const DISTRO_TIMEOUT: Duration = Duration::from_secs(60);

    // Runs `script` as root inside the distribution without a shell in
    // between (`--exec`), so the arguments arrive exactly as given. The
    // scripts below use no double quotes: they have to survive wsl.exe's
    // command-line parsing.
    async fn distro_sh(name: &str, script: &str, args: &[&str]) -> Result<WslOutput, String> {
        let mut full = vec!["-d", name, "-u", "root", "--exec", "sh", "-c", script, "sh"];
        full.extend_from_slice(args);
        run_wsl_with(&full, DISTRO_TIMEOUT).await
    }

    async fn distro_read(name: &str, path: &str) -> Result<Option<String>, String> {
        let out = distro_sh(name, "[ -e $1 ] || exit 3; cat $1", &[path]).await?;
        if out.success {
            Ok(Some(out.stdout))
        } else if out.stderr.trim().is_empty() {
            // The file just isn't there (exit 3, nothing on stderr).
            Ok(None)
        } else {
            Err(failure_message(&out))
        }
    }

    // Writes `content` to `path`: into a temporary file first, and only a
    // complete one is copied over the target. `keep_backup` leaves a
    // one-time copy of the original next to it.
    async fn distro_write(name: &str, path: &str, content: &str, mode: &str, keep_backup: bool) -> Result<(), String> {
        const SCRIPT: &str = "set -e; mkdir -p $(dirname $2); t=$2.zagzig-new; printf %s $1 | base64 -d > $t; \
            [ -s $t ] || { rm -f $t; echo the new content came through empty >&2; exit 1; }; \
            if [ $4 = 1 ] && [ -e $2 ] && [ ! -e $2.zagzig-backup ]; then cp $2 $2.zagzig-backup; fi; \
            cat $t > $2 || { rm -f $t; exit 1; }; if [ $3 != - ]; then chmod $3 $2; fi; rm -f $t";
        let encoded = base64(content.as_bytes());
        let out = distro_sh(name, SCRIPT, &[&encoded, path, mode, if keep_backup { "1" } else { "0" }]).await?;
        if out.success {
            Ok(())
        } else {
            Err(failure_message(&out))
        }
    }

    async fn windows_entries() -> Result<Vec<(String, Vec<String>)>, String> {
        Ok(super::hosts::get_hosts_entries()
            .await?
            .entries
            .into_iter()
            .filter(|e| e.enabled)
            .map(|e| (e.ip, e.hostnames))
            .collect())
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WslHostsStatus {
        /// Entries in the synced block, or None when there is no block.
        pub synced_entries: Option<usize>,
        /// False when /etc/wsl.conf turns off WSL's own /etc/hosts rebuild,
        /// which is what makes a synced block survive a restart.
        pub generate_hosts: bool,
        pub auto_sync: bool,
        /// Another program's boot command, which stops "keep synced".
        pub boot_conflict: Option<String>,
    }

    #[tauri::command]
    pub async fn wsl_hosts_status(name: String) -> Result<WslHostsStatus, String> {
        known_distro(&name).await?;
        let hosts = distro_read(&name, "/etc/hosts").await?.unwrap_or_default();
        let conf = distro_read(&name, "/etc/wsl.conf").await?.unwrap_or_default();
        let script = distro_read(&name, BOOT_SCRIPT_PATH).await?.is_some();
        let command = boot_command_of(&conf);
        Ok(WslHostsStatus {
            synced_entries: block_entry_count(&hosts),
            generate_hosts: generate_hosts_enabled(&conf),
            auto_sync: script && command.as_deref() == Some(BOOT_SCRIPT_PATH),
            boot_conflict: command.filter(|c| c != BOOT_SCRIPT_PATH),
        })
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WslHostsSyncResult {
        pub entries: usize,
        pub skipped: usize,
    }

    #[tauri::command]
    pub async fn wsl_hosts_sync(name: String) -> Result<WslHostsSyncResult, String> {
        known_distro(&name).await?;
        let block = build_block(&windows_entries().await?)?;
        let existing = distro_read(&name, "/etc/hosts").await?.unwrap_or_default();
        distro_write(&name, "/etc/hosts", &merge_block(&existing, &block.text), "-", true).await?;
        Ok(WslHostsSyncResult { entries: block.entries, skipped: block.skipped })
    }

    #[tauri::command]
    pub async fn wsl_hosts_remove(name: String) -> Result<(), String> {
        known_distro(&name).await?;
        let existing = distro_read(&name, "/etc/hosts").await?.unwrap_or_default();
        if block_entry_count(&existing).is_none() && !existing.contains(SYNC_BEGIN) {
            return Ok(());
        }
        let stripped = strip_block(&existing);
        if stripped.trim().is_empty() {
            return Err("Removing the block would leave /etc/hosts empty, so nothing was changed.".to_string());
        }
        distro_write(&name, "/etc/hosts", &stripped, "-", true).await
    }

    #[tauri::command]
    pub async fn wsl_hosts_autosync(name: String, enabled: bool) -> Result<(), String> {
        known_distro(&name).await?;
        let conf = distro_read(&name, "/etc/wsl.conf").await?.unwrap_or_default();
        if enabled {
            let source = std::env::var("WINDIR")
                .ok()
                .and_then(|w| posix_path_of(&format!("{w}\\System32\\drivers\\etc\\hosts")))
                .ok_or_else(|| "Couldn't work out where the Windows hosts file is for WSL.".to_string())?;
            let new_conf = set_boot_command(&conf)?;
            distro_write(&name, BOOT_SCRIPT_PATH, &boot_script(&source), "755", false).await?;
            if new_conf != conf {
                distro_write(&name, "/etc/wsl.conf", &new_conf, "-", true).await?;
            }
            // Apply it straight away too, so the user sees the result.
            wsl_hosts_sync(name).await.map(|_| ())
        } else {
            if boot_command_of(&conf).as_deref() == Some(BOOT_SCRIPT_PATH) {
                let new_conf = remove_boot_command(&conf);
                // An emptied file is replaced by a comment: an empty write is refused.
                let new_conf = if new_conf.is_empty() { "# /etc/wsl.conf\n".to_string() } else { new_conf };
                distro_write(&name, "/etc/wsl.conf", &new_conf, "-", true).await?;
            }
            let out = distro_sh(&name, "rm -f $1", &[BOOT_SCRIPT_PATH]).await?;
            if out.success {
                Ok(())
            } else {
                Err(failure_message(&out))
            }
        }
    }

    #[cfg(test)]
    mod hosts_sync_tests {
        use super::*;

        fn e(ip: &str, names: &[&str]) -> (String, Vec<String>) {
            (ip.to_string(), names.iter().map(|n| n.to_string()).collect())
        }

        #[test]
        fn block_skips_localhost_and_unsafe_lines() {
            let block = build_block(&[
                e("127.0.0.1", &["localhost"]),
                e("::1", &["localhost", "LOCALHOST"]),
                e("127.0.0.1", &["myapp.local", "api.local"]),
                e("10.0.0.5", &["db.corp"]),
                e("fe80::1%eth0", &["router"]),
                e("10.0.0.6", &["bad name; rm"]),
                e("not-an-ip", &["x"]),
                e("10.0.0.7", &[]),
            ])
            .unwrap();
            assert_eq!(block.entries, 3);
            assert_eq!(block.skipped, 2);
            assert_eq!(
                block.text,
                format!("{SYNC_BEGIN}\n127.0.0.1 myapp.local api.local\n10.0.0.5 db.corp\nfe80::1%eth0 router\n{SYNC_END}\n")
            );
        }

        #[test]
        fn merge_replaces_instead_of_stacking() {
            let base = "127.0.0.1 localhost\n127.0.1.1 box\n";
            let one = build_block(&[e("10.0.0.5", &["a.local"])]).unwrap().text;
            let two = build_block(&[e("10.0.0.6", &["b.local"])]).unwrap().text;
            let first = merge_block(base, &one);
            assert!(first.starts_with(base) && first.contains("a.local"));
            let second = merge_block(&first, &two);
            assert!(second.contains("b.local") && !second.contains("a.local"));
            assert_eq!(second.matches(SYNC_BEGIN).count(), 1);
            assert_eq!(merge_block(&second, &two), second, "syncing twice changes nothing");
            assert_eq!(strip_block(&second), base, "removing the block restores the original");
            assert_eq!(block_entry_count(&second), Some(1));
            assert_eq!(block_entry_count(base), None);
        }

        #[test]
        fn a_damaged_block_never_costs_other_lines() {
            let broken = format!("1.1.1.1 keep\n{SYNC_BEGIN}\n2.2.2.2 also.keep\n");
            assert_eq!(strip_block(&broken), "1.1.1.1 keep\n2.2.2.2 also.keep\n");
            assert_eq!(strip_block("no trailing newline"), "no trailing newline\n");
        }

        #[test]
        fn too_many_entries_are_refused() {
            let many: Vec<_> = (0..2000).map(|i| e("10.0.0.1", &[&format!("host-number-{i}.example.local")])).collect();
            assert!(build_block(&many).is_err());
        }

        #[test]
        fn boot_command_is_added_removed_and_never_clobbers() {
            let none = "[network]\ngenerateHosts = false\n";
            let added = set_boot_command(none).unwrap();
            assert_eq!(boot_command_of(&added).as_deref(), Some(BOOT_SCRIPT_PATH));
            assert!(added.contains("[network]") && added.contains("generateHosts = false"));
            assert_eq!(set_boot_command(&added).unwrap(), added, "adding twice changes nothing");
            assert_eq!(remove_boot_command(&added), none);

            let with_section = "[boot]\nsystemd = true\n\n[user]\ndefault = me\n";
            let added = set_boot_command(with_section).unwrap();
            assert_eq!(boot_command_of(&added).as_deref(), Some(BOOT_SCRIPT_PATH));
            assert!(added.contains("systemd = true") && added.contains("default = me"));
            assert_eq!(remove_boot_command(&added), with_section);

            let other = "[boot]\ncommand = service docker start\n";
            let err = set_boot_command(other).unwrap_err();
            assert!(err.contains("service docker start"));
            assert_eq!(remove_boot_command(other), other, "someone else's command is left alone");

            assert_eq!(set_boot_command("").unwrap(), format!("[boot]\ncommand = {BOOT_SCRIPT_PATH}\n"));
            assert_eq!(remove_boot_command(&set_boot_command("").unwrap()), "");
        }

        #[test]
        fn reads_generate_hosts() {
            assert!(generate_hosts_enabled(""));
            assert!(generate_hosts_enabled("[network]\nhostname = x\n"));
            assert!(!generate_hosts_enabled("[network]\ngenerateHosts = false\n"));
            assert!(!generate_hosts_enabled("[network]\ngenerateHosts=False\n"));
            assert!(generate_hosts_enabled("[boot]\ngenerateHosts = false\n"), "only the [network] section counts");
        }

        #[test]
        fn converts_windows_paths() {
            assert_eq!(
                posix_path_of(r"C:\Windows\System32\drivers\etc\hosts").as_deref(),
                Some("/mnt/c/Windows/System32/drivers/etc/hosts")
            );
            assert_eq!(posix_path_of(r"D:\Win\hosts").as_deref(), Some("/mnt/d/Win/hosts"));
            assert_eq!(posix_path_of("relative"), None);
        }

        // Runs against a real distribution (the default one) and puts
        // /etc/hosts back exactly as it was. Run by hand:
        // cargo test live_distro -- --ignored --nocapture
        #[test]
        #[ignore = "changes /etc/hosts of the default WSL distribution, then restores it"]
        fn live_distro() {
            tauri::async_runtime::block_on(async {
                let listing = run_wsl(&["--list", "--verbose"]).await.unwrap();
                let distros = parse_distros(&listing.stdout, &[]);
                let name = distros.iter().find(|d| d.is_default).expect("a default distribution").name.clone();
                println!("using {name}");

                // 1. The file writer: unicode, mode, one-time backup, refusing empty content.
                let dir = "/tmp/zagzig-live";
                distro_sh(&name, "rm -rf $1; mkdir -p $1", &[dir]).await.unwrap();
                let file = format!("{dir}/sub/test.txt");
                distro_write(&name, &file, "caf\u{e9} one\n", "640", true).await.unwrap();
                distro_write(&name, &file, "caf\u{e9} two\n", "640", true).await.unwrap();
                assert_eq!(distro_read(&name, &file).await.unwrap().unwrap(), "caf\u{e9} two\n");
                assert_eq!(distro_read(&name, &format!("{file}.zagzig-backup")).await.unwrap().unwrap(), "caf\u{e9} one\n");
                let mode = distro_sh(&name, "stat -c %a $1", &[&file]).await.unwrap();
                assert_eq!(mode.stdout.trim(), "640");
                assert!(distro_write(&name, &file, "", "-", false).await.is_err(), "empty content is refused");
                assert_eq!(distro_read(&name, &file).await.unwrap().unwrap(), "caf\u{e9} two\n", "and the file is untouched");
                assert_eq!(distro_read(&name, &format!("{dir}/nothing")).await.unwrap(), None);

                // 2. The boot script gives the same block as build_block.
                let windows = "# comment\r\n127.0.0.1 localhost\r\n::1\tlocalhost  LOCALHOST\r\n127.0.0.1\tmyapp.local   api.local # note\r\n10.0.0.5 db.corp\r\nfe80::1%eth0 router\r\n10.0.0.6 bad;name\r\nnot-an-ip x\r\n10.0.0.7\r\n  10.0.0.8   spaced.host  \r\n";
                distro_write(&name, &format!("{dir}/win-hosts"), windows, "-", false).await.unwrap();
                distro_write(&name, &format!("{dir}/etc-hosts"), "127.0.0.1 localhost\n127.0.1.1 box\n", "-", false).await.unwrap();
                let script = boot_script(&format!("{dir}/win-hosts")).replace("HOSTS=/etc/hosts", &format!("HOSTS={dir}/etc-hosts"));
                distro_write(&name, &format!("{dir}/boot.sh"), &script, "755", false).await.unwrap();
                let ran = distro_sh(&name, "sh $1", &[&format!("{dir}/boot.sh")]).await.unwrap();
                assert!(ran.success, "{}", failure_message(&ran));
                let by_script = distro_read(&name, &format!("{dir}/etc-hosts")).await.unwrap().unwrap();
                let expected = build_block(&[
                    e("127.0.0.1", &["myapp.local", "api.local"]),
                    e("10.0.0.5", &["db.corp"]),
                    e("fe80::1%eth0", &["router"]),
                    e("10.0.0.8", &["spaced.host"]),
                ])
                .unwrap();
                assert_eq!(by_script, merge_block("127.0.0.1 localhost\n127.0.1.1 box\n", &expected.text));
                // Running it again changes nothing.
                distro_sh(&name, "sh $1", &[&format!("{dir}/boot.sh")]).await.unwrap();
                assert_eq!(distro_read(&name, &format!("{dir}/etc-hosts")).await.unwrap().unwrap(), by_script);
                // A missing Windows file leaves /etc/hosts alone.
                distro_sh(&name, "rm $1", &[&format!("{dir}/win-hosts")]).await.unwrap();
                distro_sh(&name, "sh $1", &[&format!("{dir}/boot.sh")]).await.unwrap();
                assert_eq!(distro_read(&name, &format!("{dir}/etc-hosts")).await.unwrap().unwrap(), by_script);

                // 3. The real thing on /etc/hosts, then put back.
                let original = distro_read(&name, "/etc/hosts").await.unwrap().unwrap();
                let had_backup = distro_read(&name, "/etc/hosts.zagzig-backup").await.unwrap().is_some();
                let outcome = async {
                    let synced = wsl_hosts_sync(name.clone()).await?;
                    println!("synced {} entries ({} skipped)", synced.entries, synced.skipped);
                    let status = wsl_hosts_status(name.clone()).await?;
                    assert_eq!(status.synced_entries, Some(synced.entries));
                    let after = distro_read(&name, "/etc/hosts").await?.unwrap();
                    assert!(after.starts_with(original.trim_end_matches('\n')), "the original lines are untouched");
                    wsl_hosts_sync(name.clone()).await?;
                    assert_eq!(distro_read(&name, "/etc/hosts").await?.unwrap(), after, "syncing twice changes nothing");
                    wsl_hosts_remove(name.clone()).await?;
                    assert_eq!(distro_read(&name, "/etc/hosts").await?.unwrap(), original, "removing restores the file");
                    assert_eq!(wsl_hosts_status(name.clone()).await?.synced_entries, None);
                    Ok::<(), String>(())
                }
                .await;
                // Whatever happened, leave /etc/hosts as found.
                if distro_read(&name, "/etc/hosts").await.unwrap().unwrap() != original {
                    distro_write(&name, "/etc/hosts", &original, "-", false).await.unwrap();
                }
                if !had_backup {
                    distro_sh(&name, "rm -f /etc/hosts.zagzig-backup", &[]).await.unwrap();
                }
                distro_sh(&name, "rm -rf $1", &[dir]).await.unwrap();
                outcome.unwrap();
            });
        }

        #[test]
        fn base64_matches_the_standard_vectors() {
            for (input, expected) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")] {
                assert_eq!(base64(input.as_bytes()), expected);
            }
            assert_eq!(base64("é\n".as_bytes()), "w6kK");
        }
    }

    #[tauri::command]
    pub async fn wsl_shutdown() -> Result<(), String> {
        run_checked(&["--shutdown"]).await
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        killed: Vec<String>,
    }

    // For when `wsl --shutdown` itself hangs or fails: stops the WSL
    // service(s) (which tears down the VM and every distro), then kills
    // whatever processes are still left, then starts the services again.
    // `WSLService` is the Store/"WSL 2 package" service, `LxssManager` the
    // inbox one — whichever exist are handled. Processes that are already
    // gone, or refuse to die, aren't errors; only the service stop is.
    const FORCE_KILL_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $serviceNames = @('WSLService', 'LxssManager') | Where-Object { Get-Service -Name $_ -ErrorAction SilentlyContinue }
    foreach ($name in $serviceNames) {
        Stop-Service -Name $name -Force -ErrorAction Stop
    }
    $killed = @()
    foreach ($name in @('wsl', 'wslhost', 'wslrelay', 'wslservice', 'wslg', 'vmmem', 'vmmemWSL')) {
        $procs = Get-Process -Name $name -ErrorAction SilentlyContinue
        foreach ($p in $procs) {
            try { $p | Stop-Process -Force -ErrorAction Stop; $killed += $name } catch {}
        }
    }
    foreach ($name in $serviceNames) {
        Start-Service -Name $name -ErrorAction SilentlyContinue
    }
    @{ Success = $true; Killed = @($killed | Select-Object -Unique) } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DockerDesktop {
        pub installed: bool,
        pub running: bool,
    }

    // Docker Desktop's WSL backend lives in its own `docker-desktop`
    // distro, and it's Docker Desktop (not WSL) that recreates and rewires
    // it — so a broken WSL usually needs Docker Desktop restarted as well.
    fn docker_exe() -> Option<PathBuf> {
        ["ProgramFiles", "ProgramW6432"]
            .iter()
            .filter_map(|var| std::env::var(var).ok())
            .map(|dir| PathBuf::from(dir).join("Docker").join("Docker").join("Docker Desktop.exe"))
            .find(|path| path.exists())
    }

    // Everything Docker Desktop runs as the current user. (Its Windows
    // service, `com.docker.service`, is left alone — it's what Docker
    // Desktop itself talks to on startup.)
    const DOCKER_PROCESSES: [&str; 5] = [
        "Docker Desktop.exe",
        "com.docker.backend.exe",
        "com.docker.build.exe",
        "com.docker.dev-envs.exe",
        "vpnkit.exe",
    ];

    fn docker_running_blocking() -> bool {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("tasklist")
            .creation_flags(CREATE_NO_WINDOW)
            .args(["/FI", "IMAGENAME eq Docker Desktop.exe", "/FO", "CSV", "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("Docker Desktop.exe"))
            .unwrap_or(false)
    }

    async fn docker_state() -> DockerDesktop {
        let running = tauri::async_runtime::spawn_blocking(docker_running_blocking)
            .await
            .unwrap_or(false);
        DockerDesktop {
            installed: docker_exe().is_some(),
            running,
        }
    }

    fn stop_docker_blocking() {
        use std::os::windows::process::CommandExt;
        for name in DOCKER_PROCESSES {
            let _ = std::process::Command::new("taskkill")
                .creation_flags(CREATE_NO_WINDOW)
                .args(["/F", "/T", "/IM", name])
                .output();
        }
    }

    fn start_docker_blocking() -> Result<(), String> {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        let exe = docker_exe().ok_or_else(|| "Docker Desktop isn't installed.".to_string())?;
        std::process::Command::new(exe)
            .creation_flags(DETACHED_PROCESS)
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("failed to start Docker Desktop: {err}"))
    }

    async fn stop_docker() {
        let _ = tauri::async_runtime::spawn_blocking(stop_docker_blocking).await;
    }

    async fn start_docker() -> Result<(), String> {
        tauri::async_runtime::spawn_blocking(start_docker_blocking)
            .await
            .map_err(|err| format!("docker start task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn restart_docker_desktop() -> Result<(), String> {
        if docker_exe().is_none() {
            return Err("Docker Desktop isn't installed.".to_string());
        }
        stop_docker().await;
        sleep_off_worker(Duration::from_secs(3)).await;
        start_docker().await
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ForceRestartResult {
        pub killed: Vec<String>,
        pub started: Vec<String>,
        pub failed_to_start: Vec<String>,
        pub docker_restarted: bool,
        pub docker_error: Option<String>,
    }

    // Best effort, never blocking for long: names of distros that were
    // running before the restart, so they can be brought back after it.
    async fn running_distros_before() -> Vec<String> {
        match run_wsl_with(&["--list", "--running", "--quiet"], Duration::from_secs(5)).await {
            Ok(out) if out.success => out
                .stdout
                .lines()
                .map(|l| l.trim().trim_start_matches('*').trim().to_string())
                .filter(|l| !l.is_empty())
                .collect(),
            _ => Vec::new(),
        }
    }

    #[tauri::command]
    pub async fn wsl_force_restart(restart_docker: bool) -> Result<ForceRestartResult, String> {
        let mut to_restart = running_distros_before().await;

        // Docker Desktop goes first so it isn't fighting the service stop
        // (or respawning its distro) while WSL is torn down, and its own
        // distro isn't started by hand below — Docker recreates it.
        let restart_docker = restart_docker && docker_exe().is_some();
        if restart_docker {
            stop_docker().await;
            to_restart.retain(|name| !name.to_ascii_lowercase().starts_with("docker-desktop"));
        }

        // Elevated: stopping the service and killing vmmem needs admin.
        let raw = run_elevated(FORCE_KILL_WORKER_SCRIPT, "").await?;
        let parsed: ElevatedResult = serde_json::from_str(&raw)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if !parsed.success {
            return Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()));
        }

        // Give the service a moment to come back up before poking it.
        sleep_off_worker(Duration::from_secs(3)).await;

        // Started unelevated on purpose — a distro launched from here would
        // otherwise run in an elevated context. With nothing known to be
        // running (the usual case when WSL was too broken to answer), start
        // the default distro, which also brings the WSL VM itself back.
        let mut started = Vec::new();
        let mut failed_to_start = Vec::new();
        let targets: Vec<Option<String>> = if to_restart.is_empty() {
            // (With Docker Desktop restarting, the default distro still
            // brings the VM up before Docker starts.)
            vec![None]
        } else {
            to_restart.into_iter().map(Some).collect()
        };
        for target in targets {
            let label = target.clone().unwrap_or_else(|| "(default)".to_string());
            let args: Vec<&str> = match &target {
                Some(name) => vec!["--distribution", name, "--exec", "true"],
                None => vec!["--exec", "true"],
            };
            match run_wsl_with(&args, Duration::from_secs(60)).await {
                Ok(out) if out.success => started.push(label),
                _ => failed_to_start.push(label),
            }
        }

        let (docker_restarted, docker_error) = if restart_docker {
            match start_docker().await {
                Ok(()) => (true, None),
                Err(err) => (false, Some(err)),
            }
        } else {
            (false, None)
        };

        Ok(ForceRestartResult {
            killed: parsed.killed,
            started,
            failed_to_start,
            docker_restarted,
            docker_error,
        })
    }

    async fn sleep_off_worker(duration: Duration) {
        let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(duration)).await;
    }

    #[tauri::command]
    pub async fn set_wsl_settings(settings: WslSettings) -> Result<(), String> {
        let settings = WslSettings {
            memory: normalize(settings.memory),
            processors: normalize(settings.processors),
            swap: normalize(settings.swap),
            localhost_forwarding: normalize(settings.localhost_forwarding),
            networking_mode: normalize(settings.networking_mode),
            auto_memory_reclaim: normalize(settings.auto_memory_reclaim),
            nested_virtualization: normalize(settings.nested_virtualization),
        };
        validate(&settings)?;
        let raw = read_config_blocking().await?;
        write_config_blocking(apply_settings(&raw, &settings)).await
    }

    fn normalize(value: Option<String>) -> Option<String> {
        value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
    }

    #[tauri::command]
    pub async fn set_wsl_config_raw(content: String) -> Result<(), String> {
        write_config_blocking(content).await
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_distro_listing() {
            let listing = "  NAME            STATE           VERSION\n* Ubuntu          Running         2\n  docker-desktop  Stopped         2\n";
            let running = vec!["Ubuntu".to_string()];
            let d = parse_distros(listing, &running);
            assert_eq!(d.len(), 2);
            assert!(d[0].is_default && d[0].running && d[0].version == "2");
            assert!(!d[1].is_default && !d[1].running && d[1].name == "docker-desktop");
        }

        #[test]
        fn decodes_utf16() {
            let bytes: Vec<u8> = "Hi\n".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            assert_eq!(decode(&bytes), "Hi\n");
        }

        #[test]
        fn applies_settings_preserving_others() {
            let raw = "[wsl2]\n# keep me\nmemory=4GB\nkernel=C:\\\\k\n\n[experimental]\nsparseVhd=true\n";
            let settings = WslSettings {
                memory: Some("8GB".into()),
                processors: Some("4".into()),
                ..Default::default()
            };
            let out = apply_settings(raw, &settings);
            assert_eq!(
                out,
                "[wsl2]\n# keep me\nmemory=8GB\nkernel=C:\\\\k\nprocessors=4\n\n[experimental]\nsparseVhd=true\n"
            );
            assert_eq!(parse_settings(&out).processors.as_deref(), Some("4"));
        }

        #[test]
        fn creates_section_and_removes_keys() {
            let s = WslSettings { memory: Some("2GB".into()), ..Default::default() };
            assert_eq!(apply_settings("", &s), "[wsl2]\nmemory=2GB\n");
            let cleared = apply_settings("[wsl2]\nmemory=2GB\n", &WslSettings::default());
            assert_eq!(cleared, "[wsl2]\n");
        }

        #[test]
        fn validates_values() {
            let ok = WslSettings { memory: Some("8GB".into()), swap: Some("0".into()), ..Default::default() };
            assert!(validate(&ok).is_ok());
            let bad = WslSettings { memory: Some("8GB\nkernel=x".into()), ..Default::default() };
            assert!(validate(&bad).is_err());
        }
    }
}

// Backs the "Ports" feature: which process owns which TCP connection / UDP
// endpoint — what `netstat -ano` plus a trip to Task Manager's Details tab
// would tell you, joined into one view. Read-only and unelevated; a process
// owned by another user or the system just comes back without an executable
// path or command line (Windows won't show those to a standard user).
mod ports {
    use serde::{Deserialize, Serialize};

    use crate::{run_powershell, string_or_vec, value_or_vec};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PortEntry {
        pub protocol: String,
        pub local_address: String,
        pub local_port: u16,
        pub remote_address: String,
        pub remote_port: u16,
        pub state: String,
        pub pid: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PortProcess {
        pub pid: u32,
        pub name: String,
        pub path: Option<String>,
        pub command_line: Option<String>,
        pub parent_pid: Option<u32>,
        pub parent_name: Option<String>,
        pub start_time: Option<String>,
        pub company: Option<String>,
        pub description: Option<String>,
        pub version: Option<String>,
        pub working_set_mb: Option<f64>,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub services: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PortsSnapshot {
        #[serde(default, deserialize_with = "value_or_vec")]
        pub entries: Vec<PortEntry>,
        #[serde(default, deserialize_with = "value_or_vec")]
        pub processes: Vec<PortProcess>,
    }

    // Windows PowerShell 5.1 serializes DateTime as "\/Date(...)\/", so
    // start times are emitted as ISO-8601 strings instead. Keys are
    // camelCase here so the same structs serve both directions.
    const GET_PORTS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'

$entries = New-Object System.Collections.Generic.List[object]
foreach ($c in @(Get-NetTCPConnection)) {
    $entries.Add([pscustomobject]@{
        protocol = 'TCP'
        localAddress = [string]$c.LocalAddress
        localPort = [int]$c.LocalPort
        remoteAddress = [string]$c.RemoteAddress
        remotePort = [int]$c.RemotePort
        state = $c.State.ToString()
        pid = [int]$c.OwningProcess
    })
}
foreach ($u in @(Get-NetUDPEndpoint)) {
    $entries.Add([pscustomobject]@{
        protocol = 'UDP'
        localAddress = [string]$u.LocalAddress
        localPort = [int]$u.LocalPort
        remoteAddress = ''
        remotePort = 0
        state = ''
        pid = [int]$u.OwningProcess
    })
}

$cim = @{}
foreach ($p in @(Get-CimInstance Win32_Process)) { $cim[[int]$p.ProcessId] = $p }

$services = @{}
foreach ($s in @(Get-CimInstance Win32_Service | Where-Object { $_.ProcessId -gt 0 })) {
    $key = [int]$s.ProcessId
    if (-not $services.ContainsKey($key)) { $services[$key] = New-Object System.Collections.Generic.List[string] }
    $services[$key].Add([string]$s.Name)
}

$versionCache = @{}
$procs = New-Object System.Collections.Generic.List[object]
foreach ($procId in @($entries | ForEach-Object { $_.pid } | Sort-Object -Unique)) {
    $p = $cim[$procId]
    $name = if ($p) { [string]$p.Name } elseif ($procId -eq 0) { 'System Idle Process' } else { "PID $procId" }
    $path = if ($p -and $p.ExecutablePath) { [string]$p.ExecutablePath } else { $null }

    $company = $null; $description = $null; $version = $null
    if ($path) {
        if (-not $versionCache.ContainsKey($path)) {
            try { $versionCache[$path] = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($path) } catch { $versionCache[$path] = $null }
        }
        $vi = $versionCache[$path]
        if ($vi) {
            if ($vi.CompanyName) { $company = [string]$vi.CompanyName }
            if ($vi.FileDescription) { $description = [string]$vi.FileDescription }
            if ($vi.FileVersion) { $version = [string]$vi.FileVersion }
        }
    }

    $parentPid = $null; $parentName = $null
    if ($p -and $p.ParentProcessId) {
        $parentPid = [int]$p.ParentProcessId
        if ($cim.ContainsKey($parentPid)) { $parentName = [string]$cim[$parentPid].Name }
    }

    $procs.Add([pscustomobject]@{
        pid = $procId
        name = $name
        path = $path
        commandLine = if ($p -and $p.CommandLine) { [string]$p.CommandLine } else { $null }
        parentPid = $parentPid
        parentName = $parentName
        startTime = if ($p -and $p.CreationDate) { $p.CreationDate.ToString('o') } else { $null }
        company = $company
        description = $description
        version = $version
        workingSetMb = if ($p -and $p.WorkingSetSize) { [math]::Round($p.WorkingSetSize / 1MB, 1) } else { $null }
        services = if ($services.ContainsKey($procId)) { $services[$procId].ToArray() } else { @() }
    })
}

[pscustomobject]@{ entries = $entries.ToArray(); processes = $procs.ToArray() } | ConvertTo-Json -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn get_port_usage() -> Result<PortsSnapshot, String> {
        let trimmed = run_powershell(GET_PORTS_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(PortsSnapshot {
                entries: Vec::new(),
                processes: Vec::new(),
            });
        }
        serde_json::from_str(&trimmed).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_snapshot_with_collapsed_arrays() {
            // A single entry / single service comes back un-arrayed.
            let json = r#"{"entries":{"protocol":"TCP","localAddress":"0.0.0.0","localPort":80,"remoteAddress":"0.0.0.0","remotePort":0,"state":"Listen","pid":4},"processes":[{"pid":4,"name":"System","path":null,"commandLine":null,"parentPid":0,"parentName":null,"startTime":null,"company":null,"description":null,"version":null,"workingSetMb":null,"services":"Foo"}]}"#;
            let snap: PortsSnapshot = serde_json::from_str(json).unwrap();
            assert_eq!(snap.entries.len(), 1);
            assert_eq!(snap.processes[0].services, vec!["Foo"]);
        }

        #[test]
        fn parses_empty_snapshot() {
            let snap: PortsSnapshot = serde_json::from_str(r#"{"entries":[],"processes":[]}"#).unwrap();
            assert!(snap.entries.is_empty() && snap.processes.is_empty());
        }
    }
}

// Backs the "Port Proxy" feature: `netsh interface portproxy` forwards a
// port on this machine to another address (the usual WSL/Docker workaround)
// and has no GUI anywhere in Windows. Reading is unelevated; add/remove go
// through `run_elevated`. `netsh`'s headers are localized, so output is
// parsed positionally: which of the four rule types a block belongs to comes
// from the command that was run, and a rule is any row of
// `<addr> <port> <addr> <port>`.
mod portproxy {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell};

    const KINDS: [&str; 4] = ["v4tov4", "v4tov6", "v6tov4", "v6tov6"];

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PortProxyRule {
        pub kind: String,
        pub listen_address: String,
        pub listen_port: u16,
        pub connect_address: String,
        pub connect_port: u16,
    }

    fn parse_rules(kind: &str, output: &str) -> Vec<PortProxyRule> {
        output
            .lines()
            .filter_map(|line| {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                if tokens.len() != 4 {
                    return None;
                }
                Some(PortProxyRule {
                    kind: kind.to_string(),
                    listen_address: tokens[0].to_string(),
                    listen_port: tokens[1].parse().ok()?,
                    connect_address: tokens[2].to_string(),
                    connect_port: tokens[3].parse().ok()?,
                })
            })
            .collect()
    }

    #[tauri::command]
    pub async fn get_portproxy_rules() -> Result<Vec<PortProxyRule>, String> {
        let output = run_powershell(
            "foreach ($k in 'v4tov4','v4tov6','v6tov4','v6tov6') { '### ' + $k; netsh interface portproxy show $k }",
            &[],
        )
        .await?;

        let mut rules = Vec::new();
        for block in output.split("### ").skip(1) {
            let (kind, body) = block.split_once('\n').unwrap_or((block, ""));
            let kind = kind.trim();
            if KINDS.contains(&kind) {
                rules.extend(parse_rules(kind, body));
            }
        }
        rules.sort_by(|a, b| (a.listen_port, &a.listen_address).cmp(&(b.listen_port, &b.listen_address)));
        Ok(rules)
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct RuleRequest<'a> {
        #[serde(rename = "Action")]
        action: &'a str,
        #[serde(rename = "Kind")]
        kind: &'a str,
        #[serde(rename = "ListenAddress")]
        listen_address: &'a str,
        #[serde(rename = "ListenPort")]
        listen_port: u16,
        #[serde(rename = "ConnectAddress")]
        connect_address: &'a str,
        #[serde(rename = "ConnectPort")]
        connect_port: u16,
    }

    // Values reach netsh as separate arguments (never spliced into script
    // text), but they're still restricted to what an address can contain.
    fn valid_address(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 255
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' | '%'))
    }

    const PORTPROXY_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $netshArgs = @('interface', 'portproxy', $req.Action, $req.Kind, "listenaddress=$($req.ListenAddress)", "listenport=$($req.ListenPort)")
    if ($req.Action -eq 'add') {
        $netshArgs += "connectaddress=$($req.ConnectAddress)"
        $netshArgs += "connectport=$($req.ConnectPort)"
    }
    $output = & netsh @netshArgs 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    async fn apply(request: RuleRequest<'_>) -> Result<(), String> {
        let payload = serde_json::to_string(&request).map_err(|err| format!("failed to build request: {err}"))?;
        let raw = run_elevated(PORTPROXY_WORKER_SCRIPT, &payload).await?;
        let parsed: ElevatedResult =
            serde_json::from_str(&raw).map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    fn check_kind_and_listen(kind: &str, listen_address: &str, listen_port: u16) -> Result<(), String> {
        if !KINDS.contains(&kind) {
            return Err("Unknown rule type.".to_string());
        }
        if !valid_address(listen_address) {
            return Err("Enter a valid listen address (for example 0.0.0.0).".to_string());
        }
        if listen_port == 0 {
            return Err("The listen port must be between 1 and 65535.".to_string());
        }
        Ok(())
    }

    #[tauri::command]
    pub async fn add_portproxy_rule(
        kind: String,
        listen_address: String,
        listen_port: u16,
        connect_address: String,
        connect_port: u16,
    ) -> Result<(), String> {
        let (listen_address, connect_address) = (listen_address.trim(), connect_address.trim());
        check_kind_and_listen(&kind, listen_address, listen_port)?;
        if !valid_address(connect_address) {
            return Err("Enter a valid connect address (an IP address or hostname).".to_string());
        }
        if connect_port == 0 {
            return Err("The connect port must be between 1 and 65535.".to_string());
        }
        apply(RuleRequest {
            action: "add",
            kind: &kind,
            listen_address,
            listen_port,
            connect_address,
            connect_port,
        })
        .await
    }

    #[tauri::command]
    pub async fn remove_portproxy_rule(
        kind: String,
        listen_address: String,
        listen_port: u16,
    ) -> Result<(), String> {
        let listen_address = listen_address.trim();
        check_kind_and_listen(&kind, listen_address, listen_port)?;
        apply(RuleRequest {
            action: "delete",
            kind: &kind,
            listen_address,
            listen_port,
            connect_address: "",
            connect_port: 0,
        })
        .await
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_rule_rows_and_skips_headers() {
            let out = "Listen on ipv4:             Connect to ipv4:\n\nAddress         Port        Address         Port\n--------------- ----------  --------------- ----------\n0.0.0.0         8080        172.20.0.2      80\n127.0.0.1       2222        10.0.0.5        22\n";
            let rules = parse_rules("v4tov4", out);
            assert_eq!(rules.len(), 2);
            assert_eq!(rules[0].listen_port, 8080);
            assert_eq!(rules[1].connect_address, "10.0.0.5");
        }

        #[test]
        fn validates_addresses() {
            assert!(valid_address("0.0.0.0") && valid_address("fe80::1%12") && valid_address("host-name.local"));
            assert!(!valid_address("a b") && !valid_address("x\ny") && !valid_address(""));
        }
    }
}

// Backs the extra actions on the "Ports" page: the reserved ("excluded")
// port ranges that explain a port being unavailable while nothing listens on
// it, and stopping the process or service that owns a port.
mod portctl {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ExcludedRange {
        pub start: u16,
        pub end: u16,
        pub administered: bool,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ExcludedRanges {
        pub tcp: Vec<ExcludedRange>,
        pub udp: Vec<ExcludedRange>,
    }

    // Rows are `<start> <end> [*]`; every other line (localized titles, the
    // dashed rule, the legend) fails the numeric parse and is skipped.
    fn parse_ranges(output: &str) -> Vec<ExcludedRange> {
        output
            .lines()
            .filter_map(|line| {
                let tokens: Vec<&str> = line.split_whitespace().collect();
                if tokens.len() < 2 || tokens.len() > 3 {
                    return None;
                }
                let administered = match tokens.get(2) {
                    None => false,
                    Some(&"*") => true,
                    Some(_) => return None,
                };
                Some(ExcludedRange {
                    start: tokens[0].parse().ok()?,
                    end: tokens[1].parse().ok()?,
                    administered,
                })
            })
            .collect()
    }

    #[tauri::command]
    pub async fn get_excluded_port_ranges() -> Result<ExcludedRanges, String> {
        let output = run_powershell(
            "foreach ($p in 'tcp','udp') { '### ' + $p; netsh int ipv4 show excludedportrange protocol=$p }",
            &[],
        )
        .await?;

        let mut ranges = ExcludedRanges {
            tcp: Vec::new(),
            udp: Vec::new(),
        };
        for block in output.split("### ").skip(1) {
            let (protocol, body) = block.split_once('\n').unwrap_or((block, ""));
            match protocol.trim() {
                "tcp" => ranges.tcp = parse_ranges(body),
                "udp" => ranges.udp = parse_ranges(body),
                _ => {}
            }
        }
        Ok(ranges)
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct StopResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    // Processes that must never be stopped from here, whatever the PID.
    const PROTECTED: [&str; 9] = [
        "system",
        "registry",
        "smss.exe",
        "csrss.exe",
        "wininit.exe",
        "winlogon.exe",
        "services.exe",
        "lsass.exe",
        "memory compression",
    ];

    // Shared by the unelevated attempt (parameters via env vars) and the
    // elevated retry (the worker sets the same env vars from its input).
    // Checking the name too guards against the PID having been reused by a
    // different process since the list was read.
    const STOP_PROCESS_CORE: &str = r#"
$procId = [int]$env:ZAGZIG_PID
$p = Get-CimInstance Win32_Process -Filter "ProcessId = $procId"
if (-not $p) { return (@{ Success = $false; Error = 'That process has already exited.' } | ConvertTo-Json -Compress) }
if ($p.Name -ne $env:ZAGZIG_EXPECTED_NAME) { return (@{ Success = $false; Error = 'That PID now belongs to a different process — refresh and try again.' } | ConvertTo-Json -Compress) }
try {
    Stop-Process -Id $procId -Force -ErrorAction Stop
    return (@{ Success = $true } | ConvertTo-Json -Compress)
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    return (@{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress)
}
"#;

    fn stop_process_worker() -> String {
        format!(
            r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {{
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $env:ZAGZIG_PID = [string]$req.Pid
    $env:ZAGZIG_EXPECTED_NAME = [string]$req.Name
    $result = & {{ {STOP_PROCESS_CORE} }}
    $result | Set-Content -LiteralPath $OutputPath
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}}
"#
        )
    }

    fn parse_result(raw: &str) -> Result<StopResult, String> {
        serde_json::from_str(raw.trim()).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    #[tauri::command]
    pub async fn stop_process(pid: u32, expected_name: String) -> Result<(), String> {
        if pid == 0 || pid == 4 || pid == std::process::id() {
            return Err("That process can't be stopped from here.".to_string());
        }
        if PROTECTED.contains(&expected_name.to_ascii_lowercase().as_str()) {
            return Err("That is a critical Windows process and can't be stopped from here.".to_string());
        }

        let pid_str = pid.to_string();
        let first = parse_result(
            &run_powershell(
                STOP_PROCESS_CORE,
                &[("ZAGZIG_PID", pid_str.as_str()), ("ZAGZIG_EXPECTED_NAME", expected_name.as_str())],
            )
            .await?,
        )?;
        if first.success {
            return Ok(());
        }

        // "Already exited" and "different process" won't be fixed by more
        // rights; anything else is most likely access denied, so retry
        // elevated (one UAC prompt).
        let message = first.error.unwrap_or_default();
        if message.starts_with("That ") {
            return Err(message);
        }
        let payload = format!(
            "{{\"Pid\":{pid},\"Name\":{}}}",
            serde_json::to_string(&expected_name).map_err(|err| err.to_string())?
        );
        let retry = parse_result(&run_elevated(&stop_process_worker(), &payload).await?)?;
        if retry.success {
            Ok(())
        } else {
            Err(retry.error.unwrap_or(message))
        }
    }

    const STOP_SERVICE_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $svc = Get-CimInstance Win32_Service -Filter "Name = '$($req.Name)'"
    if (-not $svc -or [int]$svc.ProcessId -ne [int]$req.Pid) {
        throw 'That service is no longer running in that process — refresh and try again.'
    }
    Stop-Service -Name $req.Name -Force -ErrorAction Stop
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn stop_service(pid: u32, service_name: String) -> Result<(), String> {
        let name = service_name.trim();
        if name.is_empty()
            || name.len() > 256
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ' ' | '$'))
        {
            return Err("That doesn't look like a valid service name.".to_string());
        }
        let payload = format!(
            "{{\"Pid\":{pid},\"Name\":{}}}",
            serde_json::to_string(name).map_err(|err| err.to_string())?
        );
        let result = parse_result(&run_elevated(STOP_SERVICE_WORKER_SCRIPT, &payload).await?)?;
        if result.success {
            Ok(())
        } else {
            Err(result.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_excluded_ranges() {
            let out = "\nProtocol tcp Port Exclusion Ranges\n\nStart Port    End Port      \n----------    --------      \n      5985        5985      \n     50000       50059     *\n\n* - Administered port exclusions.\n";
            let r = parse_ranges(out);
            assert_eq!(r.len(), 2);
            assert!(!r[0].administered && r[0].start == 5985);
            assert!(r[1].administered && r[1].end == 50059);
        }

        #[test]
        fn worker_script_embeds_core() {
            let script = stop_process_worker();
            assert!(script.contains("Get-CimInstance Win32_Process") && script.contains("$result = & {"));
        }
    }
}

// Backs the "Network Adapters" page: each adapter's addresses, gateway, DNS,
// DHCP state, link speed and traffic counters in one place, plus enable /
// disable and DHCP renew. Reading is unelevated; the actions are elevated.
mod adapters {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell, string_or_vec};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NetworkAdapter {
        pub name: String,
        pub description: String,
        pub interface_index: u32,
        pub status: String,
        pub mac_address: Option<String>,
        pub link_speed: Option<String>,
        pub media_type: Option<String>,
        pub is_virtual: bool,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub ipv4: Vec<String>,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub ipv6: Vec<String>,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub gateways: Vec<String>,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub dns_servers: Vec<String>,
        pub dhcp: bool,
        pub mtu: Option<u32>,
        pub bytes_received: f64,
        pub bytes_sent: f64,
    }

    // Arrays are built as plain objects (never a generic List) before
    // ConvertTo-Json — Windows PowerShell 5.1 fails on the latter.
    const GET_ADAPTERS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'

$stats = @{}
foreach ($s in @(Get-NetAdapterStatistics -ErrorAction SilentlyContinue)) { $stats[[int]$s.ifIndex] = $s }

$ipif = @{}
foreach ($i in @(Get-NetIPInterface -AddressFamily IPv4 -ErrorAction SilentlyContinue)) { $ipif[[int]$i.InterfaceIndex] = $i }

$v4 = @{}; $v6 = @{}
foreach ($a in @(Get-NetIPAddress -ErrorAction SilentlyContinue)) {
    $idx = [int]$a.InterfaceIndex
    $text = "$($a.IPAddress)/$($a.PrefixLength)"
    if ($a.AddressFamily -eq 'IPv4') {
        if (-not $v4.ContainsKey($idx)) { $v4[$idx] = @() }
        $v4[$idx] += $text
    } else {
        if (-not $v6.ContainsKey($idx)) { $v6[$idx] = @() }
        $v6[$idx] += $text
    }
}

$gw = @{}
foreach ($r in @(Get-NetRoute -DestinationPrefix '0.0.0.0/0' -ErrorAction SilentlyContinue) + @(Get-NetRoute -DestinationPrefix '::/0' -ErrorAction SilentlyContinue)) {
    if ($r.NextHop -and $r.NextHop -ne '0.0.0.0' -and $r.NextHop -ne '::') {
        $idx = [int]$r.InterfaceIndex
        if (-not $gw.ContainsKey($idx)) { $gw[$idx] = @() }
        $gw[$idx] += [string]$r.NextHop
    }
}

$dns = @{}
foreach ($d in @(Get-DnsClientServerAddress -ErrorAction SilentlyContinue)) {
    $idx = [int]$d.InterfaceIndex
    if (-not $dns.ContainsKey($idx)) { $dns[$idx] = @() }
    foreach ($addr in @($d.ServerAddresses)) { if ($addr -and ($dns[$idx] -notcontains $addr)) { $dns[$idx] += [string]$addr } }
}

$result = @()
foreach ($n in @(Get-NetAdapter)) {
    $idx = [int]$n.InterfaceIndex
    $st = $stats[$idx]
    $if4 = $ipif[$idx]
    $result += [pscustomobject]@{
        name = [string]$n.Name
        description = [string]$n.InterfaceDescription
        interfaceIndex = $idx
        status = [string]$n.Status
        macAddress = if ($n.MacAddress) { [string]$n.MacAddress } else { $null }
        linkSpeed = if ($n.LinkSpeed) { [string]$n.LinkSpeed } else { $null }
        mediaType = if ($n.PhysicalMediaType) { [string]$n.PhysicalMediaType } else { $null }
        isVirtual = (-not [bool]$n.HardwareInterface)
        ipv4 = @($v4[$idx])
        ipv6 = @($v6[$idx])
        gateways = @($gw[$idx])
        dnsServers = @($dns[$idx])
        dhcp = [bool]($if4 -and $if4.Dhcp.ToString() -eq 'Enabled')
        mtu = if ($if4) { [int]$if4.NlMtu } else { $null }
        bytesReceived = if ($st) { [double]$st.ReceivedBytes } else { 0 }
        bytesSent = if ($st) { [double]$st.SentBytes } else { 0 }
    }
}
ConvertTo-Json -InputObject @($result) -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn get_network_adapters() -> Result<Vec<NetworkAdapter>, String> {
        let trimmed = run_powershell(GET_ADAPTERS_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let mut adapters: Vec<NetworkAdapter> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        adapters.sort_by(|a, b| a.is_virtual.cmp(&b.is_virtual).then_with(|| a.name.cmp(&b.name)));
        Ok(adapters)
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    // Adapters are addressed by InterfaceIndex, not name: `-Name` takes
    // wildcard patterns, and one stray `*` shouldn't disable every adapter.
    const ADAPTER_ACTION_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $adapter = Get-NetAdapter -InterfaceIndex ([int]$req.InterfaceIndex) -ErrorAction Stop
    switch ($req.Action) {
        'enable'  { Enable-NetAdapter -InputObject $adapter -Confirm:$false -ErrorAction Stop }
        'disable' { Disable-NetAdapter -InputObject $adapter -Confirm:$false -ErrorAction Stop }
        'renew'   {
            $output = & ipconfig.exe /renew $adapter.Name 2>&1 | Out-String
            if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
        }
        default   { throw 'Unknown action.' }
    }
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    async fn run_action(action: &str, interface_index: u32) -> Result<(), String> {
        let payload = format!("{{\"Action\":\"{action}\",\"InterfaceIndex\":{interface_index}}}");
        let raw = run_elevated(ADAPTER_ACTION_WORKER_SCRIPT, &payload).await?;
        let parsed: ElevatedResult =
            serde_json::from_str(&raw).map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[tauri::command]
    pub async fn set_adapter_enabled(interface_index: u32, enabled: bool) -> Result<(), String> {
        run_action(if enabled { "enable" } else { "disable" }, interface_index).await
    }

    #[tauri::command]
    pub async fn renew_adapter_dhcp(interface_index: u32) -> Result<(), String> {
        run_action("renew", interface_index).await
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_adapter_with_collapsed_arrays() {
            let json = r#"[{"name":"Ethernet","description":"Intel","interfaceIndex":12,"status":"Up","macAddress":"AA-BB","linkSpeed":"1 Gbps","mediaType":null,"isVirtual":false,"ipv4":"10.0.0.2/24","ipv6":[],"gateways":"10.0.0.1","dnsServers":["1.1.1.1","8.8.8.8"],"dhcp":true,"mtu":1500,"bytesReceived":10.0,"bytesSent":5.0}]"#;
            let a: Vec<NetworkAdapter> = serde_json::from_str(json).unwrap();
            assert_eq!(a[0].ipv4, vec!["10.0.0.2/24"]);
            assert_eq!(a[0].dns_servers.len(), 2);
            assert!(a[0].ipv6.is_empty());
        }
    }
}

// Backs the "DNS Lookup" page: a dig-style query with a chosen record type
// and an optional specific server. `-DnsOnly` skips the hosts file, LLMNR
// and mDNS so the answer is what DNS itself says.
mod dnslookup {
    use serde::{Deserialize, Serialize};

    use crate::run_powershell;

    const TYPES: [&str; 11] = [
        "A", "AAAA", "CNAME", "MX", "NS", "TXT", "SOA", "PTR", "SRV", "CAA", "DNSKEY",
    ];

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DnsRecord {
        pub name: String,
        pub record_type: String,
        pub ttl: Option<u32>,
        pub section: String,
        pub data: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DnsLookupResult {
        #[serde(default, deserialize_with = "crate::value_or_vec")]
        pub records: Vec<DnsRecord>,
        pub error: Option<String>,
        pub query_time_ms: u64,
    }

    const NAME_ENV: &str = "ZAGZIG_LOOKUP_NAME";
    const TYPE_ENV: &str = "ZAGZIG_LOOKUP_TYPE";
    const SERVER_ENV: &str = "ZAGZIG_LOOKUP_SERVER";

    const LOOKUP_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$params = @{ Name = $env:ZAGZIG_LOOKUP_NAME; Type = $env:ZAGZIG_LOOKUP_TYPE; DnsOnly = $true; ErrorAction = 'Stop' }
if ($env:ZAGZIG_LOOKUP_SERVER) { $params.Server = $env:ZAGZIG_LOOKUP_SERVER }

function Format-Data($r) {
    switch ($r.Type.ToString()) {
        { $_ -in 'A', 'AAAA' } { return [string]$r.IPAddress }
        { $_ -in 'CNAME', 'NS', 'PTR', 'DNAME' } { return [string]$r.NameHost }
        'MX' { return "$($r.Preference) $($r.NameExchange)" }
        'TXT' { return (@($r.Strings) -join '') }
        'SOA' { return "$($r.PrimaryServer) $($r.NameAdministrator) serial=$($r.SerialNumber) refresh=$($r.TimeToZoneRefresh) retry=$($r.TimeToZoneFailureRetry) expire=$($r.TimeToExpiration) minimum=$($r.DefaultTTL)" }
        'SRV' { return "$($r.Priority) $($r.Weight) $($r.Port) $($r.NameTarget)" }
        default { return ((($r | Format-List | Out-String).Trim()) -replace '\s+', ' ') }
    }
}

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$records = @()
$errorText = $null
try {
    foreach ($r in @(Resolve-DnsName @params)) {
        $records += [pscustomobject]@{
            name = [string]$r.Name
            recordType = $r.Type.ToString()
            ttl = if ($null -ne $r.TTL) { [int]$r.TTL } else { $null }
            section = $r.Section.ToString()
            data = [string](Format-Data $r)
        }
    }
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    $errorText = $ex.Message
}
$sw.Stop()
ConvertTo-Json -InputObject ([pscustomobject]@{ records = @($records); error = $errorText; queryTimeMs = [int64]$sw.ElapsedMilliseconds }) -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn dns_lookup(
        name: String,
        record_type: String,
        server: Option<String>,
    ) -> Result<DnsLookupResult, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a name to look up.".to_string());
        }
        if name.len() > 253
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':' | '*'))
        {
            return Err("That doesn't look like a valid DNS name.".to_string());
        }
        let record_type = record_type.trim().to_ascii_uppercase();
        if !TYPES.contains(&record_type.as_str()) {
            return Err("Unsupported record type.".to_string());
        }
        let server = server.as_deref().map(str::trim).filter(|s| !s.is_empty());
        if let Some(server) = server {
            if server.parse::<std::net::IpAddr>().is_err() {
                return Err("The DNS server must be an IP address.".to_string());
            }
        }

        let mut envs = vec![(NAME_ENV, name), (TYPE_ENV, record_type.as_str())];
        if let Some(server) = server {
            envs.push((SERVER_ENV, server));
        }
        let trimmed = run_powershell(LOOKUP_SCRIPT, &envs).await?;
        serde_json::from_str(&trimmed).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_lookup_result() {
            let json = r#"{"records":{"name":"a.com","recordType":"A","ttl":60,"section":"Answer","data":"1.2.3.4"},"error":null,"queryTimeMs":12}"#;
            let r: DnsLookupResult = serde_json::from_str(json).unwrap();
            assert_eq!(r.records.len(), 1);
            assert_eq!(r.records[0].data, "1.2.3.4");
        }
    }
}

// Shared by the modules below: every elevated action here has the same
// shape — a JSON request in, `{Success, Error}` JSON out — so the parsing
// lives in one place.
mod elevated_json {
    use serde::Deserialize;

    use crate::run_elevated;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Reply {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    // Runs `worker` elevated with `payload` as its input and turns its reply
    // into a plain `Result`.
    pub async fn run(worker: &str, payload: &str) -> Result<(), String> {
        let raw = run_elevated(worker, payload).await?;
        let reply: Reply =
            serde_json::from_str(raw.trim()).map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if reply.success {
            Ok(())
        } else {
            Err(reply.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    // The outer shell every worker below shares: read `$req`, run `$body`,
    // report success or the innermost exception message.
    pub fn worker(body: &str) -> String {
        format!(
            r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {{
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
{body}
    @{{ Success = $true }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}}
"#
        )
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn worker_wraps_body() {
            let w = super::worker("    Write-Output 1");
            assert!(w.contains("Write-Output 1") && w.contains("Success = $true") && w.contains("InputPath"));
        }
    }
}

// Backs the "Firewall" page: Windows Defender Firewall rules, listed from
// the CIM store in bulk (the per-rule cmdlets take a minute on a few hundred
// rules) and joined to their port and program filters. Reading is
// unelevated; enabling or disabling a rule is elevated.
mod firewall {
    use serde::{Deserialize, Serialize};

    use crate::{elevated_json, run_powershell};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FirewallRule {
        pub name: String,
        pub display_name: String,
        pub enabled: bool,
        pub direction: String,
        pub action: String,
        pub profile: String,
        pub protocol: String,
        pub local_port: String,
        pub remote_port: String,
        pub program: Option<String>,
        pub group: Option<String>,
        /// Created by this app (tagged with its rule group) — the only kind it deletes.
        #[serde(default)]
        pub app_created: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FirewallProfile {
        pub name: String,
        pub enabled: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FirewallSnapshot {
        #[serde(default, deserialize_with = "crate::value_or_vec")]
        pub profiles: Vec<FirewallProfile>,
        #[serde(default, deserialize_with = "crate::value_or_vec")]
        pub rules: Vec<FirewallRule>,
    }

    // CIM exposes the rule's enums as numbers: Direction 1/2, Action 2/4
    // (3 = allow if secure), Enabled 1/2 and Profiles as a bitmask
    // (1 Domain, 2 Private, 4 Public; 0 or 7 = all).
    const GET_FIREWALL_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$ns = 'root/StandardCimv2'

$ports = @{}
foreach ($p in @(Get-CimInstance -Namespace $ns MSFT_NetProtocolPortFilter -ErrorAction SilentlyContinue)) { $ports[$p.InstanceID] = $p }
$apps = @{}
foreach ($a in @(Get-CimInstance -Namespace $ns MSFT_NetApplicationFilter -ErrorAction SilentlyContinue)) {
    if ($a.AppPath) { $apps[$a.InstanceID] = [string]$a.AppPath }
}

function Format-Profile([int]$mask) {
    if ($mask -eq 0 -or ($mask -band 7) -eq 7) { return 'Any' }
    $names = @()
    if ($mask -band 1) { $names += 'Domain' }
    if ($mask -band 2) { $names += 'Private' }
    if ($mask -band 4) { $names += 'Public' }
    return ($names -join ', ')
}

function Join-Ports($value) {
    $items = @($value | Where-Object { $_ })
    if ($items.Count -eq 0) { return 'Any' }
    return ($items -join ',')
}

$rules = @()
foreach ($r in @(Get-CimInstance -Namespace $ns MSFT_NetFirewallRule)) {
    $pf = $ports[$r.InstanceID]
    $rules += [pscustomobject]@{
        name = [string]$r.InstanceID
        displayName = [string]$r.DisplayName
        enabled = ([int]$r.Enabled -eq 1)
        direction = if ([int]$r.Direction -eq 1) { 'Inbound' } else { 'Outbound' }
        action = if ([int]$r.Action -eq 4) { 'Block' } else { 'Allow' }
        profile = Format-Profile ([int]$r.Profiles)
        protocol = if ($pf -and $pf.Protocol) { [string]$pf.Protocol } else { 'Any' }
        localPort = if ($pf) { Join-Ports $pf.LocalPort } else { 'Any' }
        remotePort = if ($pf) { Join-Ports $pf.RemotePort } else { 'Any' }
        program = if ($apps.ContainsKey($r.InstanceID)) { $apps[$r.InstanceID] } else { $null }
        group = if ($r.DisplayGroup) { [string]$r.DisplayGroup } else { $null }
        appCreated = ([string]$r.RuleGroup -ceq 'zagzig-tools')
    }
}

$profiles = @()
foreach ($p in @(Get-NetFirewallProfile)) {
    $profiles += [pscustomobject]@{ name = [string]$p.Name; enabled = ($p.Enabled.ToString() -eq 'True') }
}

ConvertTo-Json -InputObject ([pscustomobject]@{ profiles = $profiles; rules = $rules }) -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn get_firewall() -> Result<FirewallSnapshot, String> {
        let trimmed = run_powershell(GET_FIREWALL_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(FirewallSnapshot {
                profiles: Vec::new(),
                rules: Vec::new(),
            });
        }
        let mut snapshot: FirewallSnapshot = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        snapshot
            .rules
            .sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
        Ok(snapshot)
    }

    // `-Name` takes wildcards, so the match is re-checked for an exact,
    // single rule before anything is changed.
    fn toggle_worker() -> String {
        elevated_json::worker(
            r#"    $matches = @(Get-NetFirewallRule -Name $req.Name -ErrorAction Stop | Where-Object { $_.Name -ceq $req.Name })
    if ($matches.Count -ne 1) { throw 'That rule no longer exists — refresh and try again.' }
    if ($req.Enabled) { $matches | Enable-NetFirewallRule } else { $matches | Disable-NetFirewallRule }"#,
        )
    }

    #[tauri::command]
    pub async fn set_firewall_rule_enabled(name: String, enabled: bool) -> Result<(), String> {
        if name.is_empty() || name.len() > 512 || name.chars().any(char::is_control) {
            return Err("That doesn't look like a valid rule name.".to_string());
        }
        let payload = serde_json::json!({ "Name": name, "Enabled": enabled }).to_string();
        elevated_json::run(&toggle_worker(), &payload).await
    }

    // --- Creating and deleting this app's own rules ----------------------
    //
    // Every rule made here is tagged with this group, and only rules in it
    // can be deleted here — so a mistake can never remove one of Windows' own.
    const APP_GROUP: &str = "zagzig-tools";

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct NewRuleSpec {
        pub name: String,
        /// "Inbound" or "Outbound".
        pub direction: String,
        /// "Allow" or "Block".
        pub action: String,
        /// "TCP" or "UDP".
        pub protocol: String,
        /// "80", "80,443" or "8000-8100".
        pub ports: String,
        pub program: Option<String>,
        /// Any of "Domain", "Private", "Public".
        pub profiles: Vec<String>,
        /// "Any" or "LocalSubnet".
        pub remote: String,
    }

    // "80, 443,8000-8100" -> ["80", "443", "8000-8100"]
    fn parse_port_spec(spec: &str) -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let valid = |s: &str| matches!(s.trim().parse::<u16>(), Ok(n) if n >= 1);
            match part.split_once('-') {
                Some((a, b)) if valid(a) && valid(b) && a.trim().parse::<u16>().unwrap() <= b.trim().parse::<u16>().unwrap() => {
                    out.push(format!("{}-{}", a.trim(), b.trim()));
                }
                None if valid(part) => out.push(part.to_string()),
                _ => return Err(format!("\"{part}\" isn't a port or range between 1 and 65535.")),
            }
        }
        if out.is_empty() {
            return Err("Enter at least one port.".to_string());
        }
        if out.len() > 64 {
            return Err("Too many ports or ranges (limit 64).".to_string());
        }
        Ok(out)
    }

    fn validate_spec(spec: &NewRuleSpec) -> Result<serde_json::Value, String> {
        let name = spec.name.trim();
        if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
            return Err("Enter a rule name (up to 100 characters).".to_string());
        }
        if !["Inbound", "Outbound"].contains(&spec.direction.as_str()) {
            return Err("Unknown direction.".to_string());
        }
        if !["Allow", "Block"].contains(&spec.action.as_str()) {
            return Err("Unknown action.".to_string());
        }
        if !["TCP", "UDP"].contains(&spec.protocol.as_str()) {
            return Err("Unknown protocol.".to_string());
        }
        if !["Any", "LocalSubnet"].contains(&spec.remote.as_str()) {
            return Err("Unknown remote address option.".to_string());
        }
        if spec.profiles.is_empty() || !spec.profiles.iter().all(|p| ["Domain", "Private", "Public"].contains(&p.as_str())) {
            return Err("Choose at least one network profile.".to_string());
        }
        let ports = parse_port_spec(&spec.ports)?;
        let program = spec.program.as_deref().map(str::trim).filter(|p| !p.is_empty());
        if let Some(p) = program {
            let absolute = p.len() > 3 && ((p.as_bytes()[1] == b':' && p.as_bytes()[2] == b'\\') || p.starts_with("\\\\"));
            if !absolute || p.len() > 260 || p.contains('"') || p.chars().any(char::is_control) {
                return Err("The program must be a full path such as C:\\Tools\\app.exe.".to_string());
            }
        }
        let id = format!(
            "{}-{:x}",
            APP_GROUP,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        Ok(serde_json::json!({
            "RuleName": id,
            "Name": name,
            "Direction": spec.direction,
            "Action": spec.action,
            "Protocol": spec.protocol,
            "Ports": ports,
            "Program": program,
            "Profiles": spec.profiles,
            "Remote": spec.remote,
        }))
    }

    fn create_worker() -> String {
        elevated_json::worker(&format!(
            r#"    $p = @{{
        Name = [string]$req.RuleName
        DisplayName = [string]$req.Name
        Description = 'Created by zagzig-tools'
        Group = '{APP_GROUP}'
        Direction = [string]$req.Direction
        Action = [string]$req.Action
        Protocol = [string]$req.Protocol
        Profile = (@($req.Profiles) -join ',')
        RemoteAddress = [string]$req.Remote
        Enabled = 'True'
    }}
    # An inbound rule matches the port this PC listens on; an outbound one
    # the port it connects to.
    if ($req.Direction -eq 'Inbound') {{ $p.LocalPort = @($req.Ports | ForEach-Object {{ [string]$_ }}) }} else {{ $p.RemotePort = @($req.Ports | ForEach-Object {{ [string]$_ }}) }}
    if ($req.Program) {{ $p.Program = [string]$req.Program }}
    New-NetFirewallRule @p -ErrorAction Stop | Out-Null"#
        ))
    }

    fn delete_worker() -> String {
        elevated_json::worker(&format!(
            r#"    $rules = @(Get-NetFirewallRule -Name $req.Name -ErrorAction Stop | Where-Object {{ $_.Name -ceq $req.Name }})
    if ($rules.Count -ne 1) {{ throw 'That rule no longer exists - refresh and try again.' }}
    if ($rules[0].Group -cne '{APP_GROUP}') {{ throw 'Only rules created by this app can be deleted here.' }}
    $rules | Remove-NetFirewallRule -ErrorAction Stop"#
        ))
    }

    #[tauri::command]
    pub async fn create_firewall_rule(spec: NewRuleSpec) -> Result<(), String> {
        let payload = validate_spec(&spec)?.to_string();
        elevated_json::run(&create_worker(), &payload).await
    }

    #[tauri::command]
    pub async fn delete_firewall_rule(name: String) -> Result<(), String> {
        if !name.starts_with(&format!("{APP_GROUP}-")) || name.len() > 100 || name.chars().any(char::is_control) {
            return Err("Only rules created by this app can be deleted here.".to_string());
        }
        let payload = serde_json::json!({ "Name": name }).to_string();
        elevated_json::run(&delete_worker(), &payload).await
    }

    #[cfg(test)]
    mod create_tests {
        use super::*;

        fn spec() -> NewRuleSpec {
            NewRuleSpec {
                name: "Dev server".into(),
                direction: "Inbound".into(),
                action: "Allow".into(),
                protocol: "TCP".into(),
                ports: "3000, 8000-8100".into(),
                program: None,
                profiles: vec!["Private".into()],
                remote: "LocalSubnet".into(),
            }
        }

        #[test]
        fn parses_port_specs() {
            assert_eq!(parse_port_spec("80, 443,8000-8100").unwrap(), vec!["80", "443", "8000-8100"]);
            for bad in ["", "0", "65536", "abc", "10-5", "1-", "-5", "80,,x"] {
                assert!(parse_port_spec(bad).is_err(), "{bad}");
            }
        }

        #[test]
        fn accepts_a_good_spec_and_builds_the_request() {
            let json = validate_spec(&spec()).unwrap();
            assert!(json["RuleName"].as_str().unwrap().starts_with("zagzig-tools-"));
            assert_eq!(json["Ports"], serde_json::json!(["3000", "8000-8100"]));
            assert_eq!(json["Program"], serde_json::Value::Null);
        }

        #[test]
        fn rejects_bad_specs() {
            let mut s = spec();
            s.name = "  ".into();
            assert!(validate_spec(&s).is_err());
            let mut s = spec();
            s.direction = "Sideways".into();
            assert!(validate_spec(&s).is_err());
            let mut s = spec();
            s.profiles = vec![];
            assert!(validate_spec(&s).is_err());
            let mut s = spec();
            s.profiles = vec!["Everywhere".into()];
            assert!(validate_spec(&s).is_err());
            let mut s = spec();
            s.remote = "Internet".into();
            assert!(validate_spec(&s).is_err());
            let mut s = spec();
            s.program = Some("notepad.exe".into());
            assert!(validate_spec(&s).is_err(), "relative program paths are refused");
            let mut s = spec();
            s.program = Some("C:\\Tools\\app.exe".into());
            assert!(validate_spec(&s).is_ok());
        }

        #[test]
        fn only_this_apps_rules_can_be_deleted() {
            let err = |n: &str| tauri::async_runtime::block_on(delete_firewall_rule(n.to_string())).unwrap_err();
            assert!(err("CoreNet-DHCP-In").contains("created by this app"));
            assert!(err("").contains("created by this app"));
            assert!(delete_worker().contains("-cne 'zagzig-tools'"));
        }

        #[test]
        fn workers_are_valid_powershell() {
            assert_eq!(crate::powershell_syntax_errors(&create_worker()), Vec::<String>::new());
            assert_eq!(crate::powershell_syntax_errors(&delete_worker()), Vec::<String>::new());
            assert_eq!(crate::powershell_syntax_errors(&toggle_worker()), Vec::<String>::new());
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_snapshot() {
            let json = r#"{"profiles":[{"name":"Domain","enabled":true}],"rules":{"name":"X","displayName":"Test","enabled":true,"direction":"Inbound","action":"Allow","profile":"Any","protocol":"TCP","localPort":"80","remotePort":"Any","program":null,"group":null}}"#;
            let s: FirewallSnapshot = serde_json::from_str(json).unwrap();
            assert_eq!(s.profiles.len(), 1);
            assert_eq!(s.rules[0].local_port, "80");
        }

        #[test]
        fn toggle_worker_guards_exact_match() {
            assert!(toggle_worker().contains("-ceq $req.Name"));
        }
    }
}

// Backs the "Neighbors" (ARP / NDP) page: the IP-to-MAC cache for IPv4 and
// IPv6. Reading is unelevated; clearing entries is elevated.
mod neighbors {
    use serde::{Deserialize, Serialize};

    use crate::{elevated_json, run_powershell};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Neighbor {
        pub ip_address: String,
        pub link_layer_address: String,
        pub state: String,
        pub interface_index: u32,
        pub interface_alias: String,
        pub family: String,
    }

    const GET_NEIGHBORS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$result = @()
foreach ($n in @(Get-NetNeighbor)) {
    $result += [pscustomobject]@{
        ipAddress = [string]$n.IPAddress
        linkLayerAddress = [string]$n.LinkLayerAddress
        state = $n.State.ToString()
        interfaceIndex = [int]$n.InterfaceIndex
        interfaceAlias = [string]$n.InterfaceAlias
        family = $n.AddressFamily.ToString()
    }
}
ConvertTo-Json -InputObject @($result) -Depth 3 -Compress
"#;

    #[tauri::command]
    pub async fn get_neighbors() -> Result<Vec<Neighbor>, String> {
        let trimmed = run_powershell(GET_NEIGHBORS_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str(&trimmed).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    fn remove_worker() -> String {
        elevated_json::worker(
            r#"    $addr = [System.Net.IPAddress]::Parse([string]$req.IpAddress)
    Remove-NetNeighbor -InterfaceIndex ([int]$req.InterfaceIndex) -IPAddress $addr.ToString() -Confirm:$false -ErrorAction Stop"#,
        )
    }

    fn clear_worker() -> String {
        // Permanent entries are the multicast/broadcast ones Windows keeps
        // itself; only learned entries are cleared.
        elevated_json::worker(
            r#"    Get-NetNeighbor | Where-Object { $_.State -ne 'Permanent' } | Remove-NetNeighbor -Confirm:$false -ErrorAction SilentlyContinue"#,
        )
    }

    #[tauri::command]
    pub async fn remove_neighbor(interface_index: u32, ip_address: String) -> Result<(), String> {
        if ip_address.parse::<std::net::IpAddr>().is_err() {
            return Err("That doesn't look like an IP address.".to_string());
        }
        let payload = serde_json::json!({ "IpAddress": ip_address, "InterfaceIndex": interface_index }).to_string();
        elevated_json::run(&remove_worker(), &payload).await
    }

    #[tauri::command]
    pub async fn clear_neighbors() -> Result<(), String> {
        elevated_json::run(&clear_worker(), "{}").await
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_neighbors() {
            let json = r#"[{"ipAddress":"10.0.0.1","linkLayerAddress":"AA-BB","state":"Reachable","interfaceIndex":12,"interfaceAlias":"Ethernet","family":"IPv4"}]"#;
            let n: Vec<Neighbor> = serde_json::from_str(json).unwrap();
            assert_eq!(n[0].interface_index, 12);
        }
    }
}

// Backs the "Services" page: list, start, stop, restart and change the
// startup type of Windows services. Reading is unelevated; every change is
// elevated. Services are matched by exact name — `Get-Service -Name` takes
// wildcards, so the allowed characters exclude them.
mod services {
    use serde::{Deserialize, Serialize};

    use crate::{elevated_json, run_powershell};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WindowsService {
        pub name: String,
        pub display_name: String,
        pub state: String,
        pub start_mode: String,
        pub delayed: bool,
        pub account: Option<String>,
        pub path: Option<String>,
        pub pid: u32,
        pub description: Option<String>,
        pub can_stop: bool,
    }

    const GET_SERVICES_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$result = @()
foreach ($s in @(Get-CimInstance Win32_Service)) {
    $desc = if ($s.Description) { [string]$s.Description } else { $null }
    if ($desc -and $desc.Length -gt 500) { $desc = $desc.Substring(0, 500) }
    $result += [pscustomobject]@{
        name = [string]$s.Name
        displayName = [string]$s.DisplayName
        state = [string]$s.State
        startMode = [string]$s.StartMode
        delayed = [bool]$s.DelayedAutoStart
        account = if ($s.StartName) { [string]$s.StartName } else { $null }
        path = if ($s.PathName) { [string]$s.PathName } else { $null }
        pid = [int]$s.ProcessId
        description = $desc
        canStop = [bool]$s.AcceptStop
    }
}
ConvertTo-Json -InputObject @($result) -Depth 3 -Compress
"#;

    #[tauri::command]
    pub async fn get_services() -> Result<Vec<WindowsService>, String> {
        let trimmed = run_powershell(GET_SERVICES_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let mut list: Vec<WindowsService> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        list.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
        Ok(list)
    }

    const ACTIONS: [&str; 7] = ["start", "stop", "restart", "auto", "auto-delayed", "manual", "disabled"];

    fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 256
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ' ' | '$'))
    }

    fn worker() -> String {
        // sc.exe handles every startup type uniformly, including delayed
        // auto-start, which Set-Service can't express.
        elevated_json::worker(
            r#"    $svc = Get-Service -Name $req.Name -ErrorAction Stop | Where-Object { $_.Name -ieq $req.Name } | Select-Object -First 1
    if (-not $svc) { throw 'That service no longer exists — refresh and try again.' }
    $startType = $null
    switch ($req.Action) {
        'start'        { Start-Service -InputObject $svc -ErrorAction Stop }
        'stop'         { Stop-Service -InputObject $svc -Force -ErrorAction Stop }
        'restart'      { Restart-Service -InputObject $svc -Force -ErrorAction Stop }
        'auto'         { $startType = 'auto' }
        'auto-delayed' { $startType = 'delayed-auto' }
        'manual'       { $startType = 'demand' }
        'disabled'     { $startType = 'disabled' }
        default        { throw 'Unknown action.' }
    }
    if ($startType) {
        $output = & sc.exe config $svc.Name start= $startType 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
    }"#,
        )
    }

    #[tauri::command]
    pub async fn service_action(name: String, action: String) -> Result<(), String> {
        let name = name.trim();
        if !valid_name(name) {
            return Err("That doesn't look like a valid service name.".to_string());
        }
        if !ACTIONS.contains(&action.as_str()) {
            return Err("Unknown action.".to_string());
        }
        let payload = serde_json::json!({ "Name": name, "Action": action }).to_string();
        elevated_json::run(&worker(), &payload).await
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn rejects_wildcards_and_quotes() {
            assert!(valid_name("Dnscache") && valid_name("Windows Time"));
            assert!(!valid_name("*") && !valid_name("a?b") && !valid_name("x'y") && !valid_name(""));
        }

        #[test]
        fn parses_services() {
            let json = r#"[{"name":"Dnscache","displayName":"DNS Client","state":"Running","startMode":"Auto","delayed":false,"account":"NT AUTHORITY\\NetworkService","path":null,"pid":1234,"description":null,"canStop":true}]"#;
            let s: Vec<WindowsService> = serde_json::from_str(json).unwrap();
            assert_eq!(s[0].pid, 1234);
        }
    }
}

// Backs the "Event Log" page: recent System / Application events with a few
// presets for the things this app is about (network, WSL and Docker).
// Reading these logs is unelevated (the Security log, which isn't offered,
// is the one that needs admin).
mod eventlog {
    use serde::{Deserialize, Serialize};

    use crate::run_powershell;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EventEntry {
        pub time: String,
        pub level: u8,
        pub provider: String,
        pub id: u32,
        pub log: String,
        pub message: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EventLogResult {
        #[serde(default, deserialize_with = "crate::value_or_vec")]
        pub events: Vec<EventEntry>,
        pub error: Option<String>,
    }

    // One query = a log (channel) plus provider-name patterns to look for in
    // it (`*` allowed; empty = no provider filter). A preset is a list of
    // them, run one at a time: Windows rejects a whole query when a named
    // provider or channel isn't installed on this machine (common for Docker
    // and Hyper-V), so missing ones have to be skippable individually.
    #[derive(Serialize)]
    struct Query {
        log: &'static str,
        providers: Vec<&'static str>,
    }

    fn q(log: &'static str, providers: &[&'static str]) -> Query {
        Query {
            log,
            providers: providers.to_vec(),
        }
    }

    fn preset(id: &str) -> Option<Vec<Query>> {
        match id {
            "system" => Some(vec![q("System", &[])]),
            "application" => Some(vec![q("Application", &[])]),
            "network" => Some(vec![
                q(
                    "System",
                    &[
                        "Tcpip",
                        "Tcpip6",
                        "NetBT",
                        "Microsoft-Windows-Dhcp-Client",
                        "Microsoft-Windows-DNS-Client",
                        "Microsoft-Windows-NlaSvc",
                        "Microsoft-Windows-Iphlpsvc",
                        "RasClient",
                        "Microsoft-Windows-RasClient",
                    ],
                ),
                q("Microsoft-Windows-WLAN-AutoConfig/Operational", &[]),
            ]),
            "wsl-docker" => {
                let docker_wsl: &[&'static str] = &[
                    "Docker*",
                    "docker*",
                    "LxssManager",
                    "WSL*",
                    "Microsoft-Windows-Subsystem-For-Linux",
                ];
                Some(vec![
                    q("Application", docker_wsl),
                    q("System", docker_wsl),
                    q("Microsoft-Windows-Hyper-V-Compute-Admin", &[]),
                    q("Microsoft-Windows-Hyper-V-Compute-Operational", &[]),
                    q("Microsoft-Windows-Host-Network-Service-Admin", &[]),
                    q("Microsoft-Windows-Hyper-V-VmSwitch-Operational", &[]),
                ])
            }
            _ => None,
        }
    }

    const LEVELS: [&str; 3] = ["errors", "warnings", "all"];

    const GET_EVENTS_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 hands a JSON array back as one object, so it must
# not be wrapped in @() (that would nest it).
$queries = ConvertFrom-Json -InputObject $env:ZAGZIG_EVENT_QUERIES
if ($queries -isnot [array]) { $queries = @($queries) }
$start = (Get-Date).AddHours(-[int]$env:ZAGZIG_EVENT_HOURS)
$level = $null
switch ($env:ZAGZIG_EVENT_LEVEL) {
    'errors'   { $level = @(1, 2) }
    'warnings' { $level = @(1, 2, 3) }
}
$text = $env:ZAGZIG_EVENT_TEXT
$max = [int]$env:ZAGZIG_EVENT_MAX
# Text search runs on the fetched events, so look further back than the
# number that will be shown.
$fetch = if ($text) { 2000 } else { $max }

$events = @()
$errorText = $null
foreach ($query in $queries) {
    $providers = @($query.providers)
    if ($providers.Count -eq 0) { $providers = @($null) }
    foreach ($provider in $providers) {
        $f = @{ LogName = [string]$query.log; StartTime = $start }
        if ($level) { $f.Level = $level }
        if ($provider) { $f.ProviderName = [string]$provider }
        try {
            foreach ($e in @(Get-WinEvent -FilterHashtable $f -MaxEvents $fetch -ErrorAction Stop)) {
                $message = ''
                try { $message = [string]$e.Message } catch {}
                if ($message.Length -gt 4000) { $message = $message.Substring(0, 4000) }
                if ($text -and ($message -notlike "*$text*") -and ([string]$e.ProviderName -notlike "*$text*")) { continue }
                $events += [pscustomobject]@{
                    time = $e.TimeCreated.ToString('o')
                    level = if ($null -ne $e.Level) { [int]$e.Level } else { 4 }
                    provider = [string]$e.ProviderName
                    id = [int]$e.Id
                    log = [string]$e.LogName
                    message = $message
                }
            }
        } catch {
            $ex = $_.Exception
            while ($ex.InnerException) { $ex = $ex.InnerException }
            # An empty result, an uninstalled provider or a channel that
            # doesn't exist on this machine isn't a failure.
            if ($ex.Message -notmatch 'No events were found|not an event provider|channel could not be found|specified channel|do not write events to any of the specified logs') { $errorText = $ex.Message }
        }
    }
}
$events = @($events | Sort-Object time -Descending | Select-Object -First $max)
ConvertTo-Json -InputObject ([pscustomobject]@{ events = $events; error = $errorText }) -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn get_event_log(
        preset_id: String,
        level: String,
        hours: u32,
        max_events: u32,
        text: Option<String>,
    ) -> Result<EventLogResult, String> {
        let queries = preset(&preset_id).ok_or_else(|| "Unknown event log preset.".to_string())?;
        if !LEVELS.contains(&level.as_str()) {
            return Err("Unknown level.".to_string());
        }
        let hours = hours.clamp(1, 24 * 30).to_string();
        let max_events = max_events.clamp(1, 1000).to_string();
        let text = text.as_deref().map(str::trim).unwrap_or("");
        // Only used in a -like pattern; wildcards in it just widen the match.
        if text.chars().any(char::is_control) || text.len() > 200 {
            return Err("That search text isn't valid.".to_string());
        }
        let queries = serde_json::to_string(&queries).map_err(|err| err.to_string())?;

        let trimmed = run_powershell(
            GET_EVENTS_SCRIPT,
            &[
                ("ZAGZIG_EVENT_QUERIES", queries.as_str()),
                ("ZAGZIG_EVENT_LEVEL", level.as_str()),
                ("ZAGZIG_EVENT_HOURS", hours.as_str()),
                ("ZAGZIG_EVENT_MAX", max_events.as_str()),
                ("ZAGZIG_EVENT_TEXT", text),
            ],
        )
        .await?;
        serde_json::from_str(&trimmed).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn presets_resolve() {
            assert!(preset("network").is_some() && preset("system").is_some());
            assert!(preset("security").is_none());
        }

        #[test]
        fn parses_result_with_one_event() {
            let json = r#"{"events":{"time":"2026-10-06T10:00:00.0000000+07:00","level":2,"provider":"Tcpip","id":4199,"log":"System","message":"x"},"error":null}"#;
            let r: EventLogResult = serde_json::from_str(json).unwrap();
            assert_eq!(r.events.len(), 1);
        }
    }
}

// Backs the "VPN" page: the built-in Windows VPN profiles (what Settings >
// Network > VPN manages) with connect/disconnect through `rasdial`. VPN
// clients that bring their own adapter and app (WireGuard, OpenVPN, vendor
// clients, ...) aren't Windows VPN profiles and don't show up here. NRPT
// rules, which this app also manages, are typically tied to such
// connections.
mod vpn {
    use serde::{Deserialize, Serialize};

    use crate::{run_powershell, string_or_vec};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct VpnProfile {
        pub name: String,
        pub server_address: String,
        pub status: String,
        pub tunnel_type: String,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub auth_methods: Vec<String>,
        pub split_tunneling: bool,
        pub remember_credential: bool,
        pub all_users: bool,
    }

    const GET_VPN_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$result = @()
foreach ($allUsers in @($false, $true)) {
    $conns = if ($allUsers) { Get-VpnConnection -AllUserConnection -ErrorAction SilentlyContinue } else { Get-VpnConnection -ErrorAction SilentlyContinue }
    foreach ($c in @($conns)) {
        if (-not $c) { continue }
        $result += [pscustomobject]@{
            name = [string]$c.Name
            serverAddress = [string]$c.ServerAddress
            status = [string]$c.ConnectionStatus
            tunnelType = [string]$c.TunnelType
            authMethods = @($c.AuthenticationMethod | ForEach-Object { [string]$_ })
            splitTunneling = [bool]$c.SplitTunneling
            rememberCredential = [bool]$c.RememberCredential
            allUsers = $allUsers
        }
    }
}
ConvertTo-Json -InputObject @($result) -Depth 3 -Compress
"#;

    #[tauri::command]
    pub async fn get_vpn_connections() -> Result<Vec<VpnProfile>, String> {
        let trimmed = run_powershell(GET_VPN_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let mut list: Vec<VpnProfile> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(list)
    }

    const NAME_ENV: &str = "ZAGZIG_VPN_NAME";

    // The name must be one `Get-VpnConnection` itself reports, so it can
    // never be mistaken for a rasdial option.
    const VPN_ACTION_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$name = $env:ZAGZIG_VPN_NAME
$known = @(Get-VpnConnection -ErrorAction SilentlyContinue) + @(Get-VpnConnection -AllUserConnection -ErrorAction SilentlyContinue)
if (-not ($known | Where-Object { $_.Name -ceq $name })) { throw 'That VPN connection no longer exists — refresh and try again.' }
$rasArgs = @($name)
if ($env:ZAGZIG_VPN_ACTION -eq 'disconnect') { $rasArgs += '/disconnect' }
$output = & rasdial.exe @rasArgs 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
"#;

    #[tauri::command]
    pub async fn vpn_action(name: String, action: String) -> Result<(), String> {
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) || name.starts_with('/') {
            return Err("That doesn't look like a valid VPN connection name.".to_string());
        }
        if action != "connect" && action != "disconnect" {
            return Err("Unknown action.".to_string());
        }
        run_powershell(VPN_ACTION_SCRIPT, &[(NAME_ENV, name.as_str()), ("ZAGZIG_VPN_ACTION", action.as_str())])
            .await
            .map(|_| ())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_profile_with_collapsed_auth_list() {
            let json = r#"[{"name":"Work","serverAddress":"vpn.example.com","status":"Disconnected","tunnelType":"Ikev2","authMethods":"Eap","splitTunneling":false,"rememberCredential":true,"allUsers":false}]"#;
            let v: Vec<VpnProfile> = serde_json::from_str(json).unwrap();
            assert_eq!(v[0].auth_methods, vec!["Eap"]);
        }
    }
}

// Backs the "Wi-Fi" page: the saved wireless profiles on this PC. Profiles
// are read by exporting them to XML (`netsh wlan export profile`) rather
// than parsing `netsh wlan show`, whose labels are localized. Viewing a
// saved password is a deliberate, separate action — it needs administrator
// approval, the same as Windows' own "View Wi-Fi security key".
mod wifi {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WifiProfile {
        pub name: String,
        pub authentication: Option<String>,
        pub encryption: Option<String>,
        pub connection_mode: Option<String>,
        pub auto_switch: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WifiProfiles {
        pub available: bool,
        #[serde(default, deserialize_with = "crate::value_or_vec")]
        pub profiles: Vec<WifiProfile>,
    }

    const GET_WIFI_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$dir = Join-Path ([System.IO.Path]::GetTempPath()) ('zagzig-wifi-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $dir | Out-Null
$profiles = @()
$available = $true
try {
    $output = & netsh.exe wlan export profile "folder=$dir" 2>&1 | Out-String
    $files = @(Get-ChildItem -LiteralPath $dir -Filter *.xml -ErrorAction SilentlyContinue)
    if ($LASTEXITCODE -ne 0 -and $files.Count -eq 0) { $available = $false }
    foreach ($f in $files) {
        try {
            $x = [xml](Get-Content -Raw -LiteralPath $f.FullName)
            $p = $x.WLANProfile
            $profiles += [pscustomobject]@{
                name = [string]$p.name
                authentication = if ($p.MSM.security.authEncryption.authentication) { [string]$p.MSM.security.authEncryption.authentication } else { $null }
                encryption = if ($p.MSM.security.authEncryption.encryption) { [string]$p.MSM.security.authEncryption.encryption } else { $null }
                connectionMode = if ($p.connectionMode) { [string]$p.connectionMode } else { $null }
                autoSwitch = ([string]$p.autoSwitch -eq 'true')
            }
        } catch {}
    }
} finally {
    Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
}
ConvertTo-Json -InputObject ([pscustomobject]@{ available = $available; profiles = $profiles }) -Depth 3 -Compress
"#;

    #[tauri::command]
    pub async fn get_wifi_profiles() -> Result<WifiProfiles, String> {
        let trimmed = run_powershell(GET_WIFI_SCRIPT, &[]).await?;
        let mut result: WifiProfiles = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        result.profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(result)
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct KeyReply {
        success: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        key: Option<String>,
    }

    // The exported XML is read and deleted inside the worker; the key only
    // travels back through the (immediately removed) output file.
    const REVEAL_KEY_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
$dir = Join-Path ([System.IO.Path]::GetTempPath()) ('zagzig-wifi-key-' + [guid]::NewGuid().ToString('N'))
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    New-Item -ItemType Directory -Path $dir | Out-Null
    $output = & netsh.exe wlan export profile "name=$($req.Name)" "folder=$dir" key=clear 2>&1 | Out-String
    $file = Get-ChildItem -LiteralPath $dir -Filter *.xml -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $file) { throw 'That Wi-Fi profile no longer exists — refresh and try again.' }
    $x = [xml](Get-Content -Raw -LiteralPath $file.FullName)
    $key = [string]$x.WLANProfile.MSM.security.sharedKey.keyMaterial
    @{ Success = $true; Key = $key } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} finally {
    Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
}
"#;

    // Returns `None` for an open network or one that uses certificates /
    // 802.1X, where there's no stored passphrase to show.
    #[tauri::command]
    pub async fn reveal_wifi_key(name: String) -> Result<Option<String>, String> {
        if name.is_empty() || name.len() > 64 || name.chars().any(|c| c.is_control() || c == '"') {
            return Err("That doesn't look like a valid Wi-Fi profile name.".to_string());
        }
        let payload = serde_json::json!({ "Name": name }).to_string();
        let raw = run_elevated(REVEAL_KEY_WORKER_SCRIPT, &payload).await?;
        let reply: KeyReply = serde_json::from_str(raw.trim())
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if !reply.success {
            return Err(reply.error.unwrap_or_else(|| "Unknown error.".to_string()));
        }
        Ok(reply.key.filter(|k| !k.is_empty()))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_profiles() {
            let json = r#"{"available":true,"profiles":{"name":"Home","authentication":"WPA2PSK","encryption":"AES","connectionMode":"auto","autoSwitch":false}}"#;
            let w: WifiProfiles = serde_json::from_str(json).unwrap();
            assert_eq!(w.profiles.len(), 1);
            assert!(w.available);
        }
    }
}

// Backs the "Languages" feature: translations beyond the built-in English
// and Indonesian ship as plain JSON "language packs" that live in the app's
// data folder (`%APPDATA%\<app id>\languages\<code>.json`), so a new
// language needs no rebuild — export a template, translate it, import it.
// This module only stores, validates and lists packs; merging them into
// i18next (and checking that placeholders survived translation) is done by
// the frontend.
mod languages {
    use std::path::{Path, PathBuf};

    use serde::Serialize;
    use serde_json::{Map, Value};
    use tauri::Manager;

    const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
    const MAX_PACKS: usize = 64;
    const MAX_STRING_LEN: usize = 4000;
    const MAX_DEPTH: usize = 8;
    // Shipped in the app itself; a pack can't replace them.
    const BUILT_IN: [&str; 2] = ["en", "id"];

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LanguagePack {
        pub code: String,
        pub name: String,
        pub dir: String,
        pub resources: Value,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PackError {
        pub file: String,
        pub error: String,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LanguagePacks {
        pub packs: Vec<LanguagePack>,
        pub errors: Vec<PackError>,
    }

    fn languages_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("couldn't find the app data folder: {err}"))?
            .join("languages");
        std::fs::create_dir_all(&dir).map_err(|err| format!("couldn't create the languages folder: {err}"))?;
        Ok(dir)
    }

    // A BCP 47-style tag: a 2–3 letter lowercase language, then up to three
    // subtags (region, script, ...). Also what makes the code safe to use as
    // a file name.
    fn valid_code(code: &str) -> bool {
        let mut parts = code.split('-');
        let Some(language) = parts.next() else {
            return false;
        };
        if !(2..=3).contains(&language.len()) || !language.chars().all(|c| c.is_ascii_lowercase()) {
            return false;
        }
        let rest: Vec<&str> = parts.collect();
        rest.len() <= 3
            && rest
                .iter()
                .all(|p| (2..=8).contains(&p.len()) && p.chars().all(|c| c.is_ascii_alphanumeric()))
    }

    // Keeps only objects and strings (what i18next resources here are made
    // of), bounded in depth and length; everything else is dropped.
    fn sanitize(value: &Value, depth: usize) -> Option<Value> {
        match value {
            Value::String(s) if s.chars().count() <= MAX_STRING_LEN => Some(value.clone()),
            Value::Object(map) if depth < MAX_DEPTH => {
                let cleaned: Map<String, Value> = map
                    .iter()
                    .filter_map(|(k, v)| sanitize(v, depth + 1).map(|v| (k.clone(), v)))
                    .collect();
                if cleaned.is_empty() {
                    None
                } else {
                    Some(Value::Object(cleaned))
                }
            }
            _ => None,
        }
    }

    fn parse_pack(raw: &str) -> Result<LanguagePack, String> {
        let value: Value = serde_json::from_str(raw).map_err(|err| format!("not valid JSON: {err}"))?;
        let Value::Object(mut root) = value else {
            return Err("the file must contain a JSON object.".to_string());
        };
        let meta = root
            .remove("$meta")
            .ok_or_else(|| "missing the \"$meta\" section (needs a \"code\" and a \"name\").".to_string())?;
        let code = meta
            .get("code")
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| "\"$meta.code\" is missing.".to_string())?;
        if !valid_code(code) {
            return Err(format!(
                "\"{code}\" isn't a valid language code (use something like \"fr\" or \"pt-br\")."
            ));
        }
        let name = meta
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .ok_or_else(|| "\"$meta.name\" is missing.".to_string())?;
        if name.chars().count() > 40 || name.chars().any(char::is_control) {
            return Err("\"$meta.name\" must be 40 characters or fewer.".to_string());
        }
        let dir = match meta.get("dir").and_then(Value::as_str) {
            Some("rtl") => "rtl",
            _ => "ltr",
        };
        let resources = sanitize(&Value::Object(root), 0).ok_or_else(|| "the file contains no translated strings.".to_string())?;
        Ok(LanguagePack {
            code: code.to_string(),
            name: name.to_string(),
            dir: dir.to_string(),
            resources,
        })
    }

    fn read_limited(path: &Path) -> Result<String, String> {
        let meta = std::fs::metadata(path).map_err(|err| format!("couldn't read the file: {err}"))?;
        if meta.len() > MAX_FILE_BYTES {
            return Err("the file is larger than 2 MB.".to_string());
        }
        std::fs::read_to_string(path).map_err(|err| format!("couldn't read the file: {err}"))
    }

    fn is_json(path: &Path) -> bool {
        path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json"))
    }

    #[tauri::command]
    pub async fn list_language_packs(app: tauri::AppHandle) -> Result<LanguagePacks, String> {
        let dir = languages_dir(&app)?;
        tauri::async_runtime::spawn_blocking(move || {
            let mut packs = Vec::new();
            let mut errors = Vec::new();
            let entries = std::fs::read_dir(&dir).map_err(|err| format!("couldn't read the languages folder: {err}"))?;
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| is_json(p)).collect();
            paths.sort();
            for path in paths.into_iter().take(MAX_PACKS) {
                let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let stem = path.file_stem().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                match read_limited(&path).and_then(|raw| parse_pack(&raw)) {
                    Ok(pack) if BUILT_IN.contains(&pack.code.as_str()) => errors.push(PackError {
                        file,
                        error: "English and Indonesian are built in and can't be replaced.".to_string(),
                    }),
                    Ok(pack) if pack.code != stem => errors.push(PackError {
                        file,
                        error: format!("the file name must match its language code (\"{}.json\").", pack.code),
                    }),
                    Ok(pack) => packs.push(pack),
                    Err(error) => errors.push(PackError { file, error }),
                }
            }
            Ok(LanguagePacks { packs, errors })
        })
        .await
        .map_err(|err| format!("language task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn import_language_pack(app: tauri::AppHandle, path: String) -> Result<String, String> {
        let dir = languages_dir(&app)?;
        tauri::async_runtime::spawn_blocking(move || {
            let source = PathBuf::from(&path);
            if !is_json(&source) {
                return Err("Choose a .json language file.".to_string());
            }
            let raw = read_limited(&source)?;
            let pack = parse_pack(&raw).map_err(|err| format!("That file can't be used: {err}"))?;
            if BUILT_IN.contains(&pack.code.as_str()) {
                return Err("English and Indonesian are built in and can't be replaced — use a different language code.".to_string());
            }
            std::fs::write(dir.join(format!("{}.json", pack.code)), raw)
                .map_err(|err| format!("couldn't save the language: {err}"))?;
            Ok(pack.code)
        })
        .await
        .map_err(|err| format!("language task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn remove_language_pack(app: tauri::AppHandle, code: String) -> Result<(), String> {
        if !valid_code(&code) || BUILT_IN.contains(&code.as_str()) {
            return Err("That language can't be removed.".to_string());
        }
        let file = languages_dir(&app)?.join(format!("{code}.json"));
        match std::fs::remove_file(&file) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(format!("couldn't remove the language: {err}")),
        }
    }

    // Writes a template the user picked a destination for (from a save
    // dialog). Limited to small .json files.
    #[tauri::command]
    pub async fn save_language_template(path: String, content: String) -> Result<(), String> {
        let target = PathBuf::from(&path);
        if !is_json(&target) {
            return Err("The file name must end in .json.".to_string());
        }
        if content.len() as u64 > MAX_FILE_BYTES {
            return Err("The template is too large.".to_string());
        }
        tauri::async_runtime::spawn_blocking(move || {
            std::fs::write(&target, content).map_err(|err| format!("couldn't save the file: {err}"))
        })
        .await
        .map_err(|err| format!("language task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn reveal_languages_folder(app: tauri::AppHandle) -> Result<(), String> {
        let dir = languages_dir(&app)?;
        std::process::Command::new("explorer.exe")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("couldn't open the folder: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn validates_codes() {
            assert!(valid_code("fr") && valid_code("pt-br") && valid_code("zh-hans-cn") && valid_code("fil"));
            assert!(!valid_code("") && !valid_code("EN") && !valid_code("e") && !valid_code("en_US"));
            assert!(!valid_code("../x") && !valid_code("fr-") && !valid_code("a-b-c-d-e-f"));
        }

        #[test]
        fn parses_a_pack_and_drops_non_strings() {
            let raw = r#"{"$meta":{"code":"fr","name":"Français","dir":"ltr"},"app":{"title":"Titre","n":5,"list":["a"]},"x":null}"#;
            let pack = parse_pack(raw).unwrap();
            assert_eq!(pack.code, "fr");
            assert_eq!(pack.resources, serde_json::json!({ "app": { "title": "Titre" } }));
        }

        #[test]
        fn rejects_bad_packs() {
            assert!(parse_pack("[]").is_err());
            assert!(parse_pack(r#"{"app":{"a":"b"}}"#).is_err());
            assert!(parse_pack(r#"{"$meta":{"code":"Fr","name":"x"},"a":"b"}"#).is_err());
            assert!(parse_pack(r#"{"$meta":{"code":"fr"},"a":"b"}"#).is_err());
            assert!(parse_pack(r#"{"$meta":{"code":"fr","name":"x"}}"#).is_err());
        }

        #[test]
        fn reads_rtl() {
            let pack = parse_pack(r#"{"$meta":{"code":"ar","name":"العربية","dir":"rtl"},"a":"b"}"#).unwrap();
            assert_eq!(pack.dir, "rtl");
        }
    }
}

// Backs the "Environment" page: user and system environment variables,
// including the PATH list. Values are read from and written to the registry
// directly (`HKCU\Environment`, and the Session Manager key for the machine)
// because the .NET environment API silently expands `%VAR%` references and
// drops the plain-vs-expandable type, which would corrupt a value on save.
// User-scope changes need no elevation; machine-scope changes go through
// `run_elevated`. Every change records the previous state first, so it can be
// undone, and ends with a WM_SETTINGCHANGE broadcast so Explorer and newly
// started programs pick it up.
mod envvars {
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Serialize};
    use tauri::Manager;

    use crate::{run_elevated, run_powershell, value_or_vec};

    const MAX_NAME_LEN: usize = 255;
    // Well under the 32,767-character limit of the whole environment block,
    // and the value also travels through an environment variable / temp file.
    const MAX_VALUE_LEN: usize = 16_000;
    const MAX_HISTORY: usize = 50;

    // Machine-scope variables Windows itself relies on; deleting any of them
    // can leave the PC unable to start programs or log in.
    const PROTECTED_SYSTEM: [&str; 13] = [
        "path",
        "pathext",
        "comspec",
        "systemroot",
        "windir",
        "systemdrive",
        "os",
        "temp",
        "tmp",
        "psmodulepath",
        "driverdata",
        "processor_architecture",
        "numberof_processors",
    ];

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EnvVar {
        pub name: String,
        pub value: String,
        /// "string" or "expand" (REG_EXPAND_SZ — may contain %VAR% references).
        pub kind: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EnvSnapshot {
        #[serde(default, deserialize_with = "value_or_vec")]
        pub user: Vec<EnvVar>,
        #[serde(default, deserialize_with = "value_or_vec")]
        pub system: Vec<EnvVar>,
    }

    // Registry values only; the unnamed "(Default)" value isn't a variable.
    const GET_ENV_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
function Read-EnvKey($hive, $subKey) {
    $items = @()
    $key = $hive.OpenSubKey($subKey)
    if ($key) {
        foreach ($n in $key.GetValueNames()) {
            if (-not $n) { continue }
            $kind = $key.GetValueKind($n)
            if ($kind -ne 'String' -and $kind -ne 'ExpandString') { continue }
            $items += [pscustomobject]@{
                name = [string]$n
                value = [string]$key.GetValue($n, '', 'DoNotExpandEnvironmentNames')
                kind = if ($kind -eq 'ExpandString') { 'expand' } else { 'string' }
            }
        }
        $key.Close()
    }
    return $items
}
$user = Read-EnvKey ([Microsoft.Win32.Registry]::CurrentUser) 'Environment'
$system = Read-EnvKey ([Microsoft.Win32.Registry]::LocalMachine) 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
ConvertTo-Json -InputObject ([pscustomobject]@{ user = @($user); system = @($system) }) -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn get_env_variables() -> Result<EnvSnapshot, String> {
        let trimmed = run_powershell(GET_ENV_SCRIPT, &[]).await?;
        let mut snapshot: EnvSnapshot = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        for list in [&mut snapshot.user, &mut snapshot.system] {
            list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        }
        Ok(snapshot)
    }

    // Shared by the unelevated (user scope) and elevated (machine scope)
    // paths: reads the request from `$req`, remembers the variable's previous
    // state, applies the change and broadcasts it.
    const APPLY_CORE: &str = r#"
$system = ($req.Scope -eq 'system')
$hive = if ($system) { [Microsoft.Win32.Registry]::LocalMachine } else { [Microsoft.Win32.Registry]::CurrentUser }
$subKey = if ($system) { 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment' } else { 'Environment' }
$key = $hive.OpenSubKey($subKey, $true)
if (-not $key) { throw 'Could not open the environment registry key.' }
$prev = @{ Exists = $false; Value = ''; Kind = 'string' }
try {
    $actual = $null
    foreach ($n in $key.GetValueNames()) { if ($n -ieq $req.Name) { $actual = $n; break } }
    if ($actual) {
        $prev.Exists = $true
        $prev.Value = [string]$key.GetValue($actual, '', 'DoNotExpandEnvironmentNames')
        $prev.Kind = if ($key.GetValueKind($actual) -eq 'ExpandString') { 'expand' } else { 'string' }
    }
    if ($req.Delete) {
        if ($actual) { $key.DeleteValue($actual) }
    } else {
        $kind = if ($req.Kind -eq 'expand') { [Microsoft.Win32.RegistryValueKind]::ExpandString } else { [Microsoft.Win32.RegistryValueKind]::String }
        $target = if ($actual) { $actual } else { [string]$req.Name }
        $key.SetValue($target, [string]$req.Value, $kind)
    }
} finally {
    $key.Close()
}
# Tell running programs (Explorer, new shells) the environment changed.
# Best effort: the change itself has already been saved.
try {
    if (-not ('ZagzigNative.EnvBroadcast' -as [type])) {
        Add-Type -Namespace ZagzigNative -Name EnvBroadcast -MemberDefinition '[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);'
    }
    [UIntPtr]$broadcastResult = [UIntPtr]::Zero
    [void][ZagzigNative.EnvBroadcast]::SendMessageTimeout([IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, 'Environment', 2, 3000, [ref]$broadcastResult)
} catch {}
return @{ Success = $true; Previous = $prev }
"#;

    fn user_script() -> String {
        format!(
            r#"
$ErrorActionPreference = 'Stop'
try {{
    $req = ConvertFrom-Json -InputObject $env:ZAGZIG_ENV_REQUEST
    $result = & {{ {APPLY_CORE} }}
    $result | ConvertTo-Json -Depth 4 -Compress
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress
}}
"#
        )
    }

    fn system_worker() -> String {
        format!(
            r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {{
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $result = & {{ {APPLY_CORE} }}
    $result | ConvertTo-Json -Depth 4 -Compress | Set-Content -LiteralPath $OutputPath
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}}
"#
        )
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EnvPrevious {
        pub value: String,
        pub kind: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct PrevRaw {
        exists: bool,
        #[serde(default)]
        value: String,
        #[serde(default)]
        kind: String,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ApplyReply {
        success: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        previous: Option<PrevRaw>,
    }

    /// One recorded change: what the variable looked like *before* it.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct EnvChange {
        pub id: String,
        /// Seconds since the Unix epoch.
        pub time: u64,
        pub scope: String,
        pub name: String,
        /// "set", "delete" or "undo".
        pub action: String,
        /// `None` means the variable didn't exist before.
        pub previous: Option<EnvPrevious>,
    }

    fn valid_scope(scope: &str) -> bool {
        scope == "user" || scope == "system"
    }

    fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.chars().count() <= MAX_NAME_LEN
            && name == name.trim()
            && !name.contains('=')
            && !name.chars().any(char::is_control)
    }

    fn valid_value(value: &str) -> bool {
        value.chars().count() <= MAX_VALUE_LEN && !value.chars().any(char::is_control)
    }

    fn is_protected(scope: &str, name: &str) -> bool {
        scope == "system" && PROTECTED_SYSTEM.contains(&name.to_ascii_lowercase().as_str())
    }

    fn history_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("couldn't find the app data folder: {err}"))?;
        std::fs::create_dir_all(&dir).map_err(|err| format!("couldn't create the app data folder: {err}"))?;
        Ok(dir.join("env-history.json"))
    }

    fn read_history(path: &PathBuf) -> Vec<EnvChange> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    // Newest first, capped.
    fn push_history(mut history: Vec<EnvChange>, entry: EnvChange) -> Vec<EnvChange> {
        history.insert(0, entry);
        history.truncate(MAX_HISTORY);
        history
    }

    fn now_secs() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    // Applies one change and records it. `new` is `None` to delete.
    async fn apply(
        app: &tauri::AppHandle,
        scope: &str,
        name: &str,
        new: Option<(&str, &str)>,
        action: &str,
    ) -> Result<(), String> {
        let (value, kind) = new.unwrap_or(("", "string"));
        let request = serde_json::json!({
            "Scope": scope,
            "Name": name,
            "Delete": new.is_none(),
            "Value": value,
            "Kind": kind,
        })
        .to_string();

        let raw = if scope == "system" {
            run_elevated(&system_worker(), &request).await?
        } else {
            run_powershell(&user_script(), &[("ZAGZIG_ENV_REQUEST", request.as_str())]).await?
        };
        let reply: ApplyReply = serde_json::from_str(raw.trim())
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if !reply.success {
            return Err(reply.error.unwrap_or_else(|| "Unknown error.".to_string()));
        }

        let previous = reply.previous.filter(|p| p.exists).map(|p| EnvPrevious {
            value: p.value,
            kind: if p.kind == "expand" { "expand".to_string() } else { "string".to_string() },
        });
        // Nothing changed (same value, or deleting something that wasn't there)
        // isn't worth a history entry.
        let unchanged = match (&previous, new) {
            (Some(p), Some((v, k))) => p.value == v && p.kind == k,
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            let entry = EnvChange {
                id: format!("{}-{}", now_secs(), SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0)),
                time: now_secs(),
                scope: scope.to_string(),
                name: name.to_string(),
                action: action.to_string(),
                previous,
            };
            // The change is already applied; failing to log it must not make
            // the command look like it failed.
            if let Ok(path) = history_path(app) {
                let history = push_history(read_history(&path), entry);
                if let Ok(json) = serde_json::to_string_pretty(&history) {
                    let _ = std::fs::write(path, json);
                }
            }
        }
        Ok(())
    }

    #[tauri::command]
    pub async fn set_env_variable(
        app: tauri::AppHandle,
        scope: String,
        name: String,
        value: String,
        kind: String,
    ) -> Result<(), String> {
        if !valid_scope(&scope) {
            return Err("Unknown scope.".to_string());
        }
        if !valid_name(&name) {
            return Err("Enter a valid variable name (no '=' and no leading or trailing spaces).".to_string());
        }
        if !valid_value(&value) {
            return Err(format!("The value is too long or contains control characters (limit {MAX_VALUE_LEN} characters)."));
        }
        let kind = if kind == "expand" { "expand" } else { "string" };
        apply(&app, &scope, &name, Some((&value, kind)), "set").await
    }

    #[tauri::command]
    pub async fn delete_env_variable(app: tauri::AppHandle, scope: String, name: String) -> Result<(), String> {
        if !valid_scope(&scope) || !valid_name(&name) {
            return Err("That doesn't look like a valid variable.".to_string());
        }
        if is_protected(&scope, &name) {
            return Err(format!("{name} is needed by Windows and can't be deleted here."));
        }
        apply(&app, &scope, &name, None, "delete").await
    }

    #[tauri::command]
    pub async fn get_env_history(app: tauri::AppHandle) -> Result<Vec<EnvChange>, String> {
        let path = history_path(&app)?;
        tauri::async_runtime::spawn_blocking(move || read_history(&path))
            .await
            .map_err(|err| format!("history task failed to run: {err}"))
    }

    // Puts a variable back to how it was before the recorded change. That is
    // itself a change, so it's recorded too (and can be undone in turn).
    #[tauri::command]
    pub async fn undo_env_change(app: tauri::AppHandle, id: String) -> Result<(), String> {
        let path = history_path(&app)?;
        let entry = read_history(&path)
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| "That change is no longer in the history.".to_string())?;
        if !valid_scope(&entry.scope) || !valid_name(&entry.name) {
            return Err("That history entry isn't valid.".to_string());
        }
        match &entry.previous {
            Some(p) => {
                if !valid_value(&p.value) {
                    return Err("The saved value isn't valid.".to_string());
                }
                apply(&app, &entry.scope, &entry.name, Some((&p.value, &p.kind)), "undo").await
            }
            None => {
                if is_protected(&entry.scope, &entry.name) {
                    return Err(format!("{} is needed by Windows and can't be deleted here.", entry.name));
                }
                apply(&app, &entry.scope, &entry.name, None, "undo").await
            }
        }
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PathCheck {
        pub expanded: String,
        /// `None` when it couldn't be checked cheaply (network paths) or
        /// the entry is empty.
        pub exists: Option<bool>,
    }

    // Replaces %NAME% with this process's value of NAME, the way Windows
    // expands a REG_EXPAND_SZ; unknown names are left as written.
    fn expand_env(text: &str) -> String {
        let mut out = String::new();
        let mut rest = text;
        while let Some(start) = rest.find('%') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            match after.find('%') {
                Some(end) if end > 0 => {
                    let name = &after[..end];
                    match std::env::var(name) {
                        Ok(value) => out.push_str(&value),
                        Err(_) => {
                            out.push('%');
                            out.push_str(name);
                            out.push('%');
                        }
                    }
                    rest = &after[end + 1..];
                }
                _ => {
                    out.push('%');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }

    #[tauri::command]
    pub async fn check_path_entries(entries: Vec<String>) -> Result<Vec<PathCheck>, String> {
        tauri::async_runtime::spawn_blocking(move || {
            entries
                .into_iter()
                .map(|entry| {
                    let expanded = expand_env(entry.trim());
                    // Network paths can stall for a long time when the server
                    // is unreachable, so they're not probed.
                    let exists = if expanded.is_empty() || expanded.starts_with("\\\\") {
                        None
                    } else {
                        Some(std::path::Path::new(&expanded).is_dir())
                    };
                    PathCheck { expanded, exists }
                })
                .collect()
        })
        .await
        .map_err(|err| format!("path check task failed to run: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn validates_names_and_values() {
            assert!(valid_name("JAVA_HOME") && valid_name("My Var") && valid_name("ProgramFiles(x86)"));
            assert!(!valid_name("") && !valid_name("A=B") && !valid_name(" lead") && !valid_name("x\ny"));
            assert!(valid_value("C:\\a;C:\\b") && !valid_value("a\nb") && !valid_value(&"x".repeat(MAX_VALUE_LEN + 1)));
        }

        #[test]
        fn protects_core_system_variables_only_in_system_scope() {
            assert!(is_protected("system", "Path") && is_protected("system", "COMSPEC"));
            assert!(!is_protected("user", "Path") && !is_protected("system", "JAVA_HOME"));
        }

        #[test]
        fn expands_known_variables_and_keeps_unknown_ones() {
            std::env::set_var("ZAGZIG_TEST_DIR", "C:\\Tools");
            assert_eq!(expand_env("%ZAGZIG_TEST_DIR%\\bin"), "C:\\Tools\\bin");
            assert_eq!(expand_env("%ZAGZIG_NOPE_XYZ%\\bin"), "%ZAGZIG_NOPE_XYZ%\\bin");
            assert_eq!(expand_env("100%"), "100%");
            assert_eq!(expand_env("a%%b"), "a%%b");
        }

        #[test]
        fn history_is_newest_first_and_capped() {
            let mk = |i: usize| EnvChange {
                id: i.to_string(),
                time: i as u64,
                scope: "user".into(),
                name: "X".into(),
                action: "set".into(),
                previous: None,
            };
            let mut h = Vec::new();
            for i in 0..(MAX_HISTORY + 5) {
                h = push_history(h, mk(i));
            }
            assert_eq!(h.len(), MAX_HISTORY);
            assert_eq!(h[0].id, (MAX_HISTORY + 4).to_string());
        }

        #[test]
        fn parses_replies() {
            let r: ApplyReply = serde_json::from_str(r#"{"Success":true,"Previous":{"Exists":true,"Value":"a","Kind":"expand"}}"#).unwrap();
            assert!(r.success && r.previous.unwrap().kind == "expand");
            let e: ApplyReply = serde_json::from_str(r#"{"Success":false,"Error":"nope"}"#).unwrap();
            assert!(!e.success && e.error.as_deref() == Some("nope"));
        }

        #[test]
        fn scripts_embed_the_core() {
            assert!(user_script().contains("DoNotExpandEnvironmentNames") && user_script().contains("ZAGZIG_ENV_REQUEST"));
            assert!(system_worker().contains("InputPath") && system_worker().contains("SendMessageTimeout"));
        }
    }
}

// Backs the "Wake-on-LAN" page: sends a magic packet (6 x 0xFF, then the
// target's MAC address 16 times) as a UDP broadcast, and keeps a small list
// of saved devices. It's plain UDP from Rust's standard library — no
// PowerShell and no administrator rights. The target needs nothing
// installed: its network card recognises the packet while the PC is off.
mod wol {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Serialize};
    use tauri::Manager;

    const MAX_DEVICES: usize = 200;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WolDevice {
        #[serde(default)]
        pub id: String,
        pub name: String,
        /// Normalised to `AA-BB-CC-DD-EE-FF`.
        pub mac: String,
        /// Address to ping afterwards to see whether it came up.
        #[serde(default)]
        pub host: Option<String>,
        /// Broadcast address to send to; empty means 255.255.255.255.
        #[serde(default)]
        pub broadcast: Option<String>,
        #[serde(default = "default_port")]
        pub port: u16,
    }

    fn default_port() -> u16 {
        9
    }

    // Accepts AA:BB:CC:DD:EE:FF, AA-BB-..., AABB.CCDD.EEFF and AABBCCDDEEFF.
    fn parse_mac(text: &str) -> Result<[u8; 6], String> {
        let hex: String = text.chars().filter(|c| !matches!(c, ':' | '-' | '.' | ' ')).collect();
        let bad = || "Enter a MAC address such as AA-BB-CC-DD-EE-FF.".to_string();
        if hex.len() != 12 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad());
        }
        let mut mac = [0u8; 6];
        for (i, byte) in mac.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| bad())?;
        }
        if mac == [0xFF; 6] || mac == [0; 6] {
            return Err("That isn't a device's MAC address.".to_string());
        }
        Ok(mac)
    }

    fn format_mac(mac: &[u8; 6]) -> String {
        mac.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join("-")
    }

    fn magic_packet(mac: &[u8; 6]) -> Vec<u8> {
        let mut packet = vec![0xFF; 6];
        for _ in 0..16 {
            packet.extend_from_slice(mac);
        }
        packet
    }

    fn data_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("couldn't find the app data folder: {err}"))?;
        std::fs::create_dir_all(&dir).map_err(|err| format!("couldn't create the app data folder: {err}"))?;
        Ok(dir.join("wol-devices.json"))
    }

    fn read_devices(path: &PathBuf) -> Vec<WolDevice> {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn write_devices(path: &PathBuf, devices: &[WolDevice]) -> Result<(), String> {
        let json = serde_json::to_string_pretty(devices).map_err(|err| err.to_string())?;
        std::fs::write(path, json).map_err(|err| format!("couldn't save the devices: {err}"))
    }

    fn clean_optional(value: &Option<String>) -> Option<String> {
        value.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string)
    }

    #[tauri::command]
    pub async fn get_wol_devices(app: tauri::AppHandle) -> Result<Vec<WolDevice>, String> {
        let path = data_path(&app)?;
        tauri::async_runtime::spawn_blocking(move || read_devices(&path))
            .await
            .map_err(|err| format!("device task failed to run: {err}"))
    }

    #[tauri::command]
    pub async fn save_wol_device(app: tauri::AppHandle, device: WolDevice) -> Result<Vec<WolDevice>, String> {
        let name = device.name.trim().to_string();
        if name.is_empty() || name.chars().count() > 60 || name.chars().any(char::is_control) {
            return Err("Enter a name (up to 60 characters).".to_string());
        }
        let mac = format_mac(&parse_mac(&device.mac)?);
        let host = clean_optional(&device.host);
        if let Some(h) = &host {
            if h.len() > 253 || !h.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':')) {
                return Err("The address to ping isn't valid.".to_string());
            }
        }
        let broadcast = clean_optional(&device.broadcast);
        if let Some(b) = &broadcast {
            b.parse::<Ipv4Addr>().map_err(|_| "The broadcast address must be an IPv4 address.".to_string())?;
        }
        if device.port == 0 {
            return Err("The port must be between 1 and 65535.".to_string());
        }

        let path = data_path(&app)?;
        tauri::async_runtime::spawn_blocking(move || {
            let mut devices = read_devices(&path);
            let id = if device.id.is_empty() {
                format!("{}", SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0))
            } else {
                device.id.clone()
            };
            let saved = WolDevice { id: id.clone(), name, mac, host, broadcast, port: device.port };
            match devices.iter_mut().find(|d| d.id == id) {
                Some(existing) => *existing = saved,
                None => {
                    if devices.len() >= MAX_DEVICES {
                        return Err("Too many saved devices.".to_string());
                    }
                    devices.push(saved);
                }
            }
            write_devices(&path, &devices)?;
            Ok(devices)
        })
        .await
        .map_err(|err| format!("device task failed to run: {err}"))?
    }

    #[tauri::command]
    pub async fn delete_wol_device(app: tauri::AppHandle, id: String) -> Result<Vec<WolDevice>, String> {
        let path = data_path(&app)?;
        tauri::async_runtime::spawn_blocking(move || {
            let mut devices = read_devices(&path);
            devices.retain(|d| d.id != id);
            write_devices(&path, &devices)?;
            Ok(devices)
        })
        .await
        .map_err(|err| format!("device task failed to run: {err}"))?
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WolSendResult {
        pub sent_on: u32,
        pub failures: Vec<String>,
    }

    // Sends from each given local address (so the packet leaves through the
    // chosen adapter rather than whichever the OS prefers — often a virtual
    // WSL/VPN one), or once from the default route if none are given. Three
    // copies, since UDP can drop one.
    #[tauri::command]
    pub async fn send_wol(
        mac: String,
        broadcast: Option<String>,
        port: u16,
        local_ips: Vec<String>,
    ) -> Result<WolSendResult, String> {
        let mac = parse_mac(&mac)?;
        if port == 0 {
            return Err("The port must be between 1 and 65535.".to_string());
        }
        let target = match clean_optional(&broadcast) {
            Some(b) => b.parse::<Ipv4Addr>().map_err(|_| "The broadcast address must be an IPv4 address.".to_string())?,
            None => Ipv4Addr::BROADCAST,
        };
        let sources: Vec<Option<Ipv4Addr>> = if local_ips.is_empty() {
            vec![None]
        } else {
            local_ips
                .iter()
                .map(|ip| ip.trim().parse::<Ipv4Addr>().map(Some).map_err(|_| format!("\"{ip}\" isn't an IPv4 address.")))
                .collect::<Result<_, _>>()?
        };

        tauri::async_runtime::spawn_blocking(move || {
            let packet = magic_packet(&mac);
            let mut sent_on = 0;
            let mut failures = Vec::new();
            for source in sources {
                let label = source.map_or_else(|| "default route".to_string(), |ip| ip.to_string());
                let attempt = (|| -> Result<(), String> {
                    let bind = SocketAddr::new(IpAddr::V4(source.unwrap_or(Ipv4Addr::UNSPECIFIED)), 0);
                    let socket = UdpSocket::bind(bind).map_err(|e| e.to_string())?;
                    socket.set_broadcast(true).map_err(|e| e.to_string())?;
                    for _ in 0..3 {
                        socket
                            .send_to(&packet, SocketAddr::new(IpAddr::V4(target), port))
                            .map_err(|e| e.to_string())?;
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Ok(())
                })();
                match attempt {
                    Ok(()) => sent_on += 1,
                    Err(err) => failures.push(format!("{label}: {err}")),
                }
            }
            if sent_on == 0 {
                return Err(format!("Couldn't send the packet — {}", failures.join("; ")));
            }
            Ok(WolSendResult { sent_on, failures })
        })
        .await
        .map_err(|err| format!("wake task failed to run: {err}"))?
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_common_mac_formats() {
            let expected = [0xAA, 0xBB, 0xCC, 0x00, 0x11, 0x22];
            for text in ["AA:BB:CC:00:11:22", "aa-bb-cc-00-11-22", "AABB.CC00.1122", "aabbcc001122", " AA BB CC 00 11 22 "] {
                assert_eq!(parse_mac(text).unwrap(), expected, "{text}");
            }
            assert_eq!(format_mac(&expected), "AA-BB-CC-00-11-22");
        }

        #[test]
        fn rejects_bad_macs() {
            for text in ["", "AA:BB", "GG:BB:CC:00:11:22", "FF-FF-FF-FF-FF-FF", "00-00-00-00-00-00", "AA:BB:CC:00:11:22:33"] {
                assert!(parse_mac(text).is_err(), "{text}");
            }
        }

        #[test]
        fn sends_a_magic_packet_over_udp() {
            let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
            listener.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let port = listener.local_addr().unwrap().port();

            let result = tauri::async_runtime::block_on(send_wol(
                "AA-BB-CC-00-11-22".to_string(),
                Some("127.0.0.1".to_string()),
                port,
                vec!["127.0.0.1".to_string()],
            ))
            .unwrap();
            assert_eq!(result.sent_on, 1);

            let mut buf = [0u8; 256];
            let (len, _) = listener.recv_from(&mut buf).unwrap();
            assert_eq!(&buf[..len], magic_packet(&[0xAA, 0xBB, 0xCC, 0x00, 0x11, 0x22]).as_slice());
        }

        #[test]
        fn refuses_bad_send_arguments() {
            let send = |mac: &str, broadcast: Option<&str>, port: u16, ips: Vec<&str>| {
                tauri::async_runtime::block_on(send_wol(
                    mac.to_string(),
                    broadcast.map(str::to_string),
                    port,
                    ips.into_iter().map(str::to_string).collect(),
                ))
            };
            assert!(send("nope", None, 9, vec![]).is_err());
            assert!(send("AA-BB-CC-00-11-22", None, 0, vec![]).is_err());
            assert!(send("AA-BB-CC-00-11-22", Some("not-an-ip"), 9, vec![]).is_err());
            assert!(send("AA-BB-CC-00-11-22", None, 9, vec!["1.2.3"]).is_err());
        }

        #[test]
        fn builds_a_102_byte_magic_packet() {
            let mac = [1, 2, 3, 4, 5, 6];
            let p = magic_packet(&mac);
            assert_eq!(p.len(), 102);
            assert!(p[..6].iter().all(|b| *b == 0xFF));
            assert!(p[6..].chunks(6).all(|c| c == mac));
        }
    }
}

// Backs the "Port Scanner" page: a plain TCP connect scan of one host. A
// connection that completes is open; one that's refused is closed; one that
// times out is probably filtered by a firewall. It's deliberately limited —
// one host at a time, at most 4,096 ports, no address sweeps, no raw packets.
mod portscan {
    use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    use serde::Serialize;

    const MAX_PORTS: usize = 4096;
    const WORKERS: usize = 64;

    static CANCEL: AtomicBool = AtomicBool::new(false);
    static RUNNING: AtomicBool = AtomicBool::new(false);

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OpenPort {
        pub port: u16,
        pub service: Option<String>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ScanResult {
        pub host: String,
        pub address: String,
        pub open: Vec<OpenPort>,
        pub scanned: u32,
        pub closed: u32,
        pub filtered: u32,
        pub errors: u32,
        pub cancelled: bool,
        pub duration_ms: u64,
    }

    // "22,80,443,8000-8100" -> sorted, de-duplicated ports.
    fn parse_ports(spec: &str) -> Result<Vec<u16>, String> {
        let mut ports: Vec<u16> = Vec::new();
        for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let parse = |s: &str| -> Result<u16, String> {
                match s.trim().parse::<u16>() {
                    Ok(n) if n >= 1 => Ok(n),
                    _ => Err(format!("\"{s}\" isn't a port between 1 and 65535.")),
                }
            };
            match part.split_once('-') {
                Some((a, b)) => {
                    let (a, b) = (parse(a)?, parse(b)?);
                    if a > b {
                        return Err(format!("\"{part}\" is a backwards range."));
                    }
                    if ports.len() + usize::from(b - a) + 1 > MAX_PORTS * 2 {
                        return Err(format!("Too many ports — the limit is {MAX_PORTS}."));
                    }
                    ports.extend(a..=b);
                }
                None => ports.push(parse(part)?),
            }
        }
        ports.sort_unstable();
        ports.dedup();
        if ports.is_empty() {
            return Err("Enter at least one port.".to_string());
        }
        if ports.len() > MAX_PORTS {
            return Err(format!("Too many ports — the limit is {MAX_PORTS}."));
        }
        Ok(ports)
    }

    fn service_name(port: u16) -> Option<&'static str> {
        Some(match port {
            20 | 21 => "FTP",
            22 => "SSH",
            23 => "Telnet",
            25 | 587 => "SMTP",
            53 => "DNS",
            67 | 68 => "DHCP",
            80 => "HTTP",
            88 => "Kerberos",
            110 => "POP3",
            111 => "RPC",
            123 => "NTP",
            135 => "MS RPC",
            137..=139 => "NetBIOS",
            143 => "IMAP",
            161 => "SNMP",
            389 => "LDAP",
            443 => "HTTPS",
            445 => "SMB",
            465 => "SMTPS",
            514 => "Syslog",
            636 => "LDAPS",
            993 => "IMAPS",
            995 => "POP3S",
            1433 => "SQL Server",
            1521 => "Oracle",
            2049 => "NFS",
            2375 | 2376 => "Docker API",
            3000 => "Dev server",
            3306 => "MySQL",
            3389 => "RDP",
            4200 => "Angular dev",
            5000 => "Dev server",
            5432 => "PostgreSQL",
            5672 => "RabbitMQ",
            5900 => "VNC",
            5985 | 5986 => "WinRM",
            6379 => "Redis",
            8000 | 8080 | 8888 => "HTTP alt",
            8443 => "HTTPS alt",
            9000 => "Dev server",
            9092 => "Kafka",
            9200 | 9300 => "Elasticsearch",
            11211 => "Memcached",
            27017 => "MongoDB",
            _ => return None,
        })
    }

    fn valid_host(host: &str) -> bool {
        !host.is_empty()
            && host.len() <= 253
            && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
    }

    #[derive(Clone, Copy)]
    enum Outcome {
        Open,
        Closed,
        Filtered,
        Error,
    }

    fn probe(addr: IpAddr, port: u16, timeout: Duration) -> Outcome {
        match TcpStream::connect_timeout(&SocketAddr::new(addr, port), timeout) {
            Ok(_) => Outcome::Open,
            Err(err) => match err.kind() {
                std::io::ErrorKind::ConnectionRefused => Outcome::Closed,
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Outcome::Filtered,
                _ => Outcome::Error,
            },
        }
    }

    fn scan_blocking(addr: IpAddr, ports: &[u16], timeout: Duration) -> (Vec<(u16, Outcome)>, bool) {
        let next = AtomicUsize::new(0);
        let results: Mutex<Vec<(u16, Outcome)>> = Mutex::new(Vec::with_capacity(ports.len()));
        let workers = ports.len().clamp(1, WORKERS);
        std::thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| loop {
                    if CANCEL.load(Ordering::Relaxed) {
                        break;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&port) = ports.get(i) else { break };
                    let outcome = probe(addr, port, timeout);
                    results.lock().unwrap().push((port, outcome));
                });
            }
        });
        (results.into_inner().unwrap(), CANCEL.load(Ordering::Relaxed))
    }

    fn resolve(host: &str) -> Result<IpAddr, String> {
        let addrs: Vec<IpAddr> = (host, 0u16)
            .to_socket_addrs()
            .map_err(|_| format!("Couldn't resolve \"{host}\"."))?
            .map(|a| a.ip())
            .collect();
        let ip = addrs
            .iter()
            .find(|a| a.is_ipv4())
            .or_else(|| addrs.first())
            .copied()
            .ok_or_else(|| format!("Couldn't resolve \"{host}\"."))?;
        let blocked = ip.is_unspecified()
            || ip.is_multicast()
            || matches!(ip, IpAddr::V4(v4) if v4.is_broadcast());
        if blocked {
            return Err("That address can't be scanned.".to_string());
        }
        Ok(ip)
    }

    #[tauri::command]
    pub async fn scan_ports(host: String, ports: String, timeout_ms: u64) -> Result<ScanResult, String> {
        let host = host.trim().to_string();
        if !valid_host(&host) {
            return Err("Enter a hostname or IP address.".to_string());
        }
        let port_list = parse_ports(&ports)?;
        let timeout = Duration::from_millis(timeout_ms.clamp(100, 5000));
        if RUNNING.swap(true, Ordering::SeqCst) {
            return Err("A scan is already running.".to_string());
        }
        CANCEL.store(false, Ordering::SeqCst);

        let outcome = tauri::async_runtime::spawn_blocking(move || -> Result<ScanResult, String> {
            let address = resolve(&host)?;
            let started = Instant::now();
            let (results, cancelled) = scan_blocking(address, &port_list, timeout);
            let mut open: Vec<OpenPort> = Vec::new();
            let (mut closed, mut filtered, mut errors) = (0, 0, 0);
            for (port, outcome) in &results {
                match outcome {
                    Outcome::Open => open.push(OpenPort { port: *port, service: service_name(*port).map(str::to_string) }),
                    Outcome::Closed => closed += 1,
                    Outcome::Filtered => filtered += 1,
                    Outcome::Error => errors += 1,
                }
            }
            open.sort_by_key(|p| p.port);
            Ok(ScanResult {
                host,
                address: address.to_string(),
                open,
                scanned: results.len() as u32,
                closed,
                filtered,
                errors,
                cancelled,
                duration_ms: started.elapsed().as_millis() as u64,
            })
        })
        .await
        .map_err(|err| format!("scan task failed to run: {err}"));

        RUNNING.store(false, Ordering::SeqCst);
        outcome?
    }

    #[tauri::command]
    pub fn cancel_port_scan() {
        CANCEL.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::TcpListener;

        #[test]
        fn parses_port_specs() {
            assert_eq!(parse_ports("80, 22,443,22").unwrap(), vec![22, 80, 443]);
            assert_eq!(parse_ports("8000-8003,1").unwrap(), vec![1, 8000, 8001, 8002, 8003]);
            for bad in ["", "0", "65536", "abc", "10-5", "1-99999", "1-5000"] {
                assert!(parse_ports(bad).is_err(), "{bad}");
            }
        }

        #[test]
        fn knows_common_services() {
            assert_eq!(service_name(443), Some("HTTPS"));
            assert_eq!(service_name(5432), Some("PostgreSQL"));
            assert_eq!(service_name(49999), None);
        }

        #[test]
        fn rejects_unscannable_hosts() {
            assert!(!valid_host("") && !valid_host("a b") && !valid_host("x;y"));
            assert!(resolve("0.0.0.0").is_err() && resolve("255.255.255.255").is_err());
        }

        #[test]
        fn finds_an_open_port_and_a_closed_one() {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let open_port = listener.local_addr().unwrap().port();
            let closed_port = {
                let l = TcpListener::bind("127.0.0.1:0").unwrap();
                l.local_addr().unwrap().port()
            };
            CANCEL.store(false, Ordering::SeqCst);
            // Windows can take a couple of seconds to refuse a loopback
            // connection, so allow for it.
            let (results, cancelled) =
                scan_blocking("127.0.0.1".parse().unwrap(), &[open_port, closed_port], Duration::from_millis(4000));
            assert!(!cancelled);
            let outcome = |p: u16| results.iter().find(|(port, _)| *port == p).map(|(_, o)| *o);
            assert!(matches!(outcome(open_port), Some(Outcome::Open)));
            assert!(matches!(outcome(closed_port), Some(Outcome::Closed)));
        }
    }
}

// Backs the "Startup" page: what launches when you sign in, from the
// registry Run keys and the Startup folders, with the same on/off switch
// Task Manager's Startup tab uses (the `StartupApproved` flags). Reading is
// unelevated; switching an entry that belongs to all users needs admin.
mod startup {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct StartupItem {
        pub source: String,
        pub source_label: String,
        pub machine_wide: bool,
        pub name: String,
        pub command: String,
        pub path: Option<String>,
        pub file_exists: bool,
        pub company: Option<String>,
        pub description: Option<String>,
        pub enabled: bool,
    }

    const LIST_SCRIPT: &str = include_str!("../scripts/startup_list.ps1");

    #[tauri::command]
    pub async fn get_startup_items() -> Result<Vec<StartupItem>, String> {
        let trimmed = run_powershell(LIST_SCRIPT, &[]).await?;
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let mut items: Vec<StartupItem> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        Ok(items)
    }

    const SOURCES: [&str; 5] = ["hkcu-run", "hklm-run", "hklm-run32", "startup-user", "startup-common"];

    fn is_machine_wide(source: &str) -> bool {
        matches!(source, "hklm-run" | "hklm-run32" | "startup-common")
    }

    // Writes the StartupApproved flag for one entry: 02 00 00 00 + zeros
    // when enabled; 03 00 00 00 + the time it was disabled when not — the
    // exact layout Task Manager uses.
    const TOGGLE_CORE: &str = r#"
$approvedBase = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved'
$runBase = 'Software\Microsoft\Windows\CurrentVersion\Run'
$defs = @{
    'hkcu-run'       = @{ hive = 'CurrentUser';  run = $runBase; approved = "$approvedBase\Run";           folder = $null }
    'hklm-run'       = @{ hive = 'LocalMachine'; run = $runBase; approved = "$approvedBase\Run";           folder = $null }
    'hklm-run32'     = @{ hive = 'LocalMachine'; run = 'Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'; approved = "$approvedBase\Run32"; folder = $null }
    'startup-user'   = @{ hive = 'CurrentUser';  run = $null; approved = "$approvedBase\StartupFolder"; folder = [Environment]::GetFolderPath('Startup') }
    'startup-common' = @{ hive = 'LocalMachine'; run = $null; approved = "$approvedBase\StartupFolder"; folder = [Environment]::GetFolderPath('CommonStartup') }
}
$def = $defs[[string]$req.Source]
if (-not $def) { throw 'Unknown startup location.' }
$hive = if ($def.hive -eq 'LocalMachine') { [Microsoft.Win32.Registry]::LocalMachine } else { [Microsoft.Win32.Registry]::CurrentUser }
$name = [string]$req.Name

# The entry has to exist where it claims to.
$actual = $null
if ($def.run) {
    $runKey = $hive.OpenSubKey($def.run)
    if ($runKey) { try { foreach ($n in $runKey.GetValueNames()) { if ($n -ieq $name) { $actual = $n; break } } } finally { $runKey.Close() } }
} else {
    if ($name -match '[\\/:*?"<>|]') { throw 'That startup entry name isn''t valid.' }
    if ($def.folder -and (Test-Path -LiteralPath (Join-Path $def.folder $name))) { $actual = $name }
}
if (-not $actual) { throw 'That startup entry no longer exists — refresh and try again.' }

$bytes = New-Object byte[] 12
if ($req.Enabled) { $bytes[0] = 2 } else {
    $bytes[0] = 3
    [BitConverter]::GetBytes([DateTime]::UtcNow.ToFileTimeUtc()).CopyTo($bytes, 4)
}
$key = $hive.CreateSubKey($def.approved)
try { $key.SetValue($actual, $bytes, [Microsoft.Win32.RegistryValueKind]::Binary) } finally { $key.Close() }
return @{ Success = $true }
"#;

    fn user_script() -> String {
        format!(
            r#"
$ErrorActionPreference = 'Stop'
try {{
    $req = ConvertFrom-Json -InputObject $env:ZAGZIG_STARTUP_REQUEST
    $result = & {{ {TOGGLE_CORE} }}
    $result | ConvertTo-Json -Compress
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress
}}
"#
        )
    }

    fn machine_worker() -> String {
        format!(
            r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {{
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $result = & {{ {TOGGLE_CORE} }}
    $result | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}}
"#
        )
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct Reply {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    #[tauri::command]
    pub async fn set_startup_item_enabled(source: String, name: String, enabled: bool) -> Result<(), String> {
        if !SOURCES.contains(&source.as_str()) {
            return Err("Unknown startup location.".to_string());
        }
        if name.is_empty() || name.chars().count() > 260 || name.chars().any(char::is_control) {
            return Err("That doesn't look like a valid startup entry.".to_string());
        }
        let request = serde_json::json!({ "Source": source, "Name": name, "Enabled": enabled }).to_string();
        let raw = if is_machine_wide(&source) {
            run_elevated(&machine_worker(), &request).await?
        } else {
            run_powershell(&user_script(), &[("ZAGZIG_STARTUP_REQUEST", request.as_str())]).await?
        };
        let reply: Reply =
            serde_json::from_str(raw.trim()).map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if reply.success {
            Ok(())
        } else {
            Err(reply.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    // Opens Explorer with the file selected.
    #[tauri::command]
    pub async fn reveal_in_explorer(path: String) -> Result<(), String> {
        use std::os::windows::process::CommandExt;
        if path.is_empty() || path.contains('"') || path.chars().any(char::is_control) {
            return Err("That path can't be shown.".to_string());
        }
        if !std::path::Path::new(&path).exists() {
            return Err("That file doesn't exist.".to_string());
        }
        std::process::Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{path}\""))
            .spawn()
            .map(|_| ())
            .map_err(|err| format!("couldn't open Explorer: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn knows_which_sources_need_admin() {
            assert!(is_machine_wide("hklm-run") && is_machine_wide("startup-common"));
            assert!(!is_machine_wide("hkcu-run") && !is_machine_wide("startup-user"));
        }

        #[test]
        fn scripts_embed_the_core() {
            assert!(user_script().contains("StartupApproved") && user_script().contains("ZAGZIG_STARTUP_REQUEST"));
            assert!(machine_worker().contains("InputPath") && machine_worker().contains("CreateSubKey"));
        }

        #[test]
        fn parses_items_and_replies() {
            let json = r#"[{"source":"hkcu-run","sourceLabel":"HKCU Run","machineWide":false,"name":"X","command":"c","path":null,"fileExists":false,"company":null,"description":null,"enabled":true}]"#;
            let items: Vec<StartupItem> = serde_json::from_str(json).unwrap();
            assert!(items[0].enabled);
            let r: Reply = serde_json::from_str(r#"{"Success":false,"Error":"x"}"#).unwrap();
            assert!(!r.success);
        }
    }
}

// Backs the "TLS Inspector" page: connects to host:port, does the TLS
// handshake and reports the certificate chain, expiry, names and negotiated
// protocol — and, using Windows' own trust store, whether Windows would
// accept it (which is what decides whether most Windows tools do). It uses
// .NET's TLS stack through PowerShell, so there's no extra dependency. The
// connection is only for the handshake; nothing is sent after it, and
// invalid certificates are still read rather than refused.
mod tls {
    use serde::{Deserialize, Serialize};

    use crate::{run_powershell, string_or_vec, value_or_vec};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TlsCertificate {
        pub subject: String,
        pub issuer: String,
        pub serial: String,
        pub thumbprint: String,
        pub not_before: String,
        pub not_after: String,
        pub days_remaining: i32,
        pub signature: Option<String>,
        pub public_key: Option<String>,
        pub self_signed: bool,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub san: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TlsChainElement {
        pub cert: TlsCertificate,
        #[serde(default, deserialize_with = "string_or_vec")]
        pub problems: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TlsInspection {
        pub host: String,
        pub port: u16,
        pub server_name: String,
        pub connected: bool,
        pub handshake: bool,
        pub protocol: Option<String>,
        pub cipher: Option<String>,
        pub cipher_strength: Option<u32>,
        pub hash: Option<String>,
        pub key_exchange: Option<String>,
        pub policy_errors: Option<String>,
        pub trusted: bool,
        pub name_matches: bool,
        pub certificate: Option<TlsCertificate>,
        #[serde(default, deserialize_with = "value_or_vec")]
        pub chain: Vec<TlsChainElement>,
        pub error: Option<String>,
        pub duration_ms: u64,
    }

    const INSPECT_SCRIPT: &str = include_str!("../scripts/tls_inspect.ps1");

    fn valid_name(name: &str) -> bool {
        !name.is_empty()
            && name.len() <= 253
            && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ':'))
    }

    #[tauri::command]
    pub async fn inspect_tls(
        host: String,
        port: u16,
        server_name: Option<String>,
        timeout_ms: u64,
    ) -> Result<TlsInspection, String> {
        let host = host.trim().to_string();
        if !valid_name(&host) {
            return Err("Enter a hostname or IP address.".to_string());
        }
        if port == 0 {
            return Err("The port must be between 1 and 65535.".to_string());
        }
        let sni = server_name.as_deref().map(str::trim).unwrap_or("").to_string();
        if !sni.is_empty() && !valid_name(&sni) {
            return Err("The server name isn't valid.".to_string());
        }
        let port_text = port.to_string();
        let timeout_text = timeout_ms.clamp(1000, 30_000).to_string();

        let trimmed = run_powershell(
            INSPECT_SCRIPT,
            &[
                ("ZAGZIG_TLS_HOST", host.as_str()),
                ("ZAGZIG_TLS_PORT", port_text.as_str()),
                ("ZAGZIG_TLS_SNI", sni.as_str()),
                ("ZAGZIG_TLS_TIMEOUT_MS", timeout_text.as_str()),
            ],
        )
        .await?;
        serde_json::from_str(&trimmed).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn validates_names() {
            assert!(valid_name("example.com") && valid_name("10.0.0.5") && valid_name("[::1]".trim_matches(['[', ']'])));
            assert!(!valid_name("") && !valid_name("a b") && !valid_name("x;y") && !valid_name("a/b"));
        }

        #[test]
        fn parses_a_failed_connection_and_a_collapsed_chain() {
            let failed = r#"{"host":"h","port":443,"serverName":"h","connected":false,"handshake":false,"protocol":null,"cipher":null,"cipherStrength":null,"hash":null,"keyExchange":null,"policyErrors":null,"trusted":false,"nameMatches":false,"certificate":null,"chain":[],"error":"refused","durationMs":5}"#;
            let r: TlsInspection = serde_json::from_str(failed).unwrap();
            assert!(!r.connected && r.error.as_deref() == Some("refused") && r.chain.is_empty());

            let cert = r#"{"subject":"CN=a","issuer":"CN=a","serial":"01","thumbprint":"AB","notBefore":"2026-01-01T00:00:00Z","notAfter":"2027-01-01T00:00:00Z","daysRemaining":90,"signature":"sha256RSA","publicKey":"RSA 2048-bit","selfSigned":true,"san":"DNS Name=a"}"#;
            let one = format!(r#"{{"host":"h","port":443,"serverName":"h","connected":true,"handshake":true,"protocol":"Tls12","cipher":"Aes128","cipherStrength":128,"hash":"Sha256","keyExchange":"ECDH","policyErrors":"None","trusted":true,"nameMatches":true,"certificate":{cert},"chain":{{"cert":{cert},"problems":"x"}},"error":null,"durationMs":9}}"#);
            let r: TlsInspection = serde_json::from_str(&one).unwrap();
            assert_eq!(r.chain.len(), 1);
            assert_eq!(r.chain[0].problems, vec!["x"]);
            assert_eq!(r.certificate.unwrap().san, vec!["DNS Name=a"]);
        }
    }
}

// Backs the "Diagnostic Report" page: gathers what this app already knows
// about the machine — system, adapters, DNS, routes, proxy, hosts, listening
// ports, firewall, WSL, recent network errors — into one Markdown file for a
// support ticket. Each section is collected concurrently and a section that
// fails is noted instead of failing the report. Nothing secret is included
// (no saved passwords, no environment variable values), and the report can
// hide the computer and user names, MAC addresses and the last octet of
// IPv4 addresses before it's shown.
mod diagnostics {
    use std::fmt::Write as _;

    use serde::Deserialize;

    use crate::run_powershell;

    const SECTIONS: [&str; 11] = [
        "system", "adapters", "dns", "nrpt", "routes", "proxy", "hosts", "ports", "firewall", "wsl", "events",
    ];

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ReportOptions {
        pub sections: Vec<String>,
        /// Replace the computer and user names and mask MAC addresses.
        pub hide_personal: bool,
        /// Replace the last octet of IPv4 addresses with `x`.
        pub mask_ips: bool,
    }

    // --- Formatting helpers ----------------------------------------------

    fn cell(text: &str) -> String {
        text.replace('|', "\\|").replace(['\r', '\n'], " ")
    }

    fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
        if rows.is_empty() {
            return "_None._\n".to_string();
        }
        let mut out = format!("| {} |\n|{}\n", headers.join(" | "), " --- |".repeat(headers.len()));
        for row in rows {
            let _ = writeln!(out, "| {} |", row.iter().map(|c| cell(c)).collect::<Vec<_>>().join(" | "));
        }
        out
    }

    fn or_dash(value: &str) -> String {
        if value.trim().is_empty() {
            "—".to_string()
        } else {
            value.to_string()
        }
    }

    fn join_or_dash(values: &[String]) -> String {
        or_dash(&values.join(", "))
    }

    // (year, month, day, hour, minute) in UTC from Unix seconds.
    fn civil_utc(secs: u64) -> (u64, u64, u64, u64, u64) {
        let days = secs / 86_400;
        let rem = secs % 86_400;
        // Howard Hinnant's days-to-civil algorithm.
        let z = days as i64 + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if m <= 2 { y + 1 } else { y };
        (year as u64, m as u64, d as u64, rem / 3_600, rem % 3_600 / 60)
    }

    fn format_utc(secs: u64) -> String {
        let (y, mo, d, h, mi) = civil_utc(secs);
        format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02} UTC")
    }

    // --- Redaction ---------------------------------------------------------

    // Case-insensitive replace of an ASCII-or-not literal.
    fn replace_ci(text: &str, needle: &str, with: &str) -> String {
        if needle.chars().count() < 3 {
            return text.to_string(); // too short to replace without mangling
        }
        let (lower_text, lower_needle) = (text.to_lowercase(), needle.to_lowercase());
        // Lowercasing can change byte lengths for some scripts; fall back to
        // leaving the text alone rather than risk slicing in the wrong place.
        if lower_text.len() != text.len() {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for (start, _) in lower_text.match_indices(&lower_needle) {
            if start < last {
                continue;
            }
            out.push_str(&text[last..start]);
            out.push_str(with);
            last = start + lower_needle.len();
        }
        out.push_str(&text[last..]);
        out
    }

    fn is_hex(b: u8) -> bool {
        b.is_ascii_hexdigit()
    }

    // `AA-BB-CC-DD-EE-FF` or `aa:bb:...` -> `xx-xx-xx-xx-xx-xx`.
    fn mask_macs(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < bytes.len() {
            if i + 17 <= bytes.len() && is_mac_at(bytes, i) {
                out.push_str("xx-xx-xx-xx-xx-xx");
                i += 17;
            } else {
                // Push the whole character, not just one byte.
                let ch = text[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
        }
        out
    }

    fn is_mac_at(b: &[u8], i: usize) -> bool {
        let sep = b[i + 2];
        if sep != b'-' && sep != b':' {
            return false;
        }
        for g in 0..6 {
            let p = i + g * 3;
            if !is_hex(b[p]) || !is_hex(b[p + 1]) {
                return false;
            }
            if g < 5 && b[p + 2] != sep {
                return false;
            }
        }
        let before_ok = i == 0 || !(is_hex(b[i - 1]) || b[i - 1] == b'-' || b[i - 1] == b':');
        let after_ok = i + 17 == b.len() || !(is_hex(b[i + 17]) || b[i + 17] == b'-' || b[i + 17] == b':');
        before_ok && after_ok
    }

    // `192.168.1.20` -> `192.168.1.x` (and only real dotted quads: version
    // strings such as 10.0.26200.1234 are left alone by the callers, which
    // don't mask the system section, and by the octet check here).
    fn mask_ipv4(text: &str) -> String {
        let bytes = text.as_bytes();
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < bytes.len() {
            let starts_number = bytes[i].is_ascii_digit() && (i == 0 || !(bytes[i - 1].is_ascii_digit() || bytes[i - 1] == b'.'));
            if starts_number {
                if let Some((end, last_octet_start)) = ipv4_at(bytes, i) {
                    let quad = &text[i..end];
                    // Loopback and "any address" say nothing about the
                    // network, and masking them just makes the report harder
                    // to read.
                    if quad.starts_with("127.") || quad == "0.0.0.0" {
                        out.push_str(quad);
                    } else {
                        out.push_str(&text[i..last_octet_start]);
                        out.push('x');
                    }
                    i = end;
                    continue;
                }
            }
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    // If a dotted quad starts at `i`, returns (end, start of its last octet).
    fn ipv4_at(b: &[u8], i: usize) -> Option<(usize, usize)> {
        let mut pos = i;
        let mut last_start = i;
        for octet in 0..4 {
            let start = pos;
            while pos < b.len() && b[pos].is_ascii_digit() && pos - start < 3 {
                pos += 1;
            }
            if pos == start {
                return None;
            }
            if std::str::from_utf8(&b[start..pos]).ok()?.parse::<u16>().ok()? > 255 {
                return None;
            }
            last_start = start;
            if octet < 3 {
                if pos >= b.len() || b[pos] != b'.' {
                    return None;
                }
                pos += 1;
            }
        }
        // Not part of a longer number or a longer dotted sequence.
        let follows = pos < b.len() && (b[pos].is_ascii_digit() || (b[pos] == b'.' && pos + 1 < b.len() && b[pos + 1].is_ascii_digit()));
        if follows {
            return None;
        }
        Some((pos, last_start))
    }

    struct Redactor {
        names: Vec<(String, &'static str)>,
        macs: bool,
        ips: bool,
    }

    impl Redactor {
        fn apply(&self, text: &str, mask_ips: bool) -> String {
            let mut out = text.to_string();
            for (name, replacement) in &self.names {
                out = replace_ci(&out, name, replacement);
            }
            if self.macs {
                out = mask_macs(&out);
            }
            if self.ips && mask_ips {
                out = mask_ipv4(&out);
            }
            out
        }
    }

    // --- Sections ----------------------------------------------------------

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SystemInfo {
        os: String,
        version: String,
        build: String,
        architecture: String,
        uptime_hours: f64,
        computer: String,
        user: String,
        domain: String,
        part_of_domain: bool,
        manufacturer: String,
        model: String,
        memory_gb: f64,
        powershell: String,
        time_zone: String,
        culture: String,
    }

    const SYSTEM_INFO_SCRIPT: &str = include_str!("../scripts/system_info.ps1");

    async fn system_section() -> Result<String, String> {
        let raw = run_powershell(SYSTEM_INFO_SCRIPT, &[]).await?;
        let s: SystemInfo = serde_json::from_str(&raw).map_err(|err| format!("failed to parse powershell output: {err}"))?;
        let admin = super::system::is_administrator().await.unwrap_or(false);
        let rows = vec![
            vec!["App".to_string(), format!("zagzig-tools v{}", env!("CARGO_PKG_VERSION"))],
            vec!["Windows".to_string(), format!("{} ({}, build {}, {})", s.os, s.version, s.build, s.architecture)],
            vec!["Computer".to_string(), format!("{} — {}", s.computer, if s.part_of_domain { format!("domain {}", s.domain) } else { format!("workgroup {}", s.domain) })],
            vec!["Signed in as".to_string(), format!("{} ({})", s.user, if admin { "administrator" } else { "standard user" })],
            vec!["Hardware".to_string(), format!("{} {} — {} GB RAM", s.manufacturer, s.model, s.memory_gb)],
            vec!["Uptime".to_string(), format!("{} hours", s.uptime_hours)],
            vec!["PowerShell".to_string(), s.powershell],
            vec!["Time zone / locale".to_string(), format!("{} / {}", s.time_zone, s.culture)],
        ];
        Ok(table(&["Item", "Value"], rows))
    }

    async fn adapters_section() -> Result<String, String> {
        let adapters = super::adapters::get_network_adapters().await?;
        let rows = adapters
            .iter()
            .map(|a| {
                vec![
                    format!("{}{}", a.name, if a.is_virtual { " (virtual)" } else { "" }),
                    a.status.clone(),
                    join_or_dash(&a.ipv4),
                    join_or_dash(&a.gateways),
                    join_or_dash(&a.dns_servers),
                    if a.dhcp { "DHCP".to_string() } else { "static".to_string() },
                    or_dash(a.link_speed.as_deref().unwrap_or("")),
                    a.mtu.map(|m| m.to_string()).unwrap_or_else(|| "—".to_string()),
                    or_dash(a.mac_address.as_deref().unwrap_or("")),
                ]
            })
            .collect();
        Ok(table(&["Adapter", "Status", "IPv4", "Gateway", "DNS", "Addressing", "Speed", "MTU", "MAC"], rows))
    }

    async fn dns_section() -> Result<String, String> {
        let interfaces = super::dns::get_dns_settings().await?;
        let rows = interfaces
            .iter()
            .map(|i| {
                vec![
                    i.interface_alias.clone(),
                    i.status.clone(),
                    join_or_dash(&i.server_addresses),
                    if i.dhcp { "DHCP".to_string() } else { "static".to_string() },
                ]
            })
            .collect();
        Ok(table(&["Adapter", "Status", "DNS servers", "Source"], rows))
    }

    async fn nrpt_section() -> Result<String, String> {
        let rules = super::nrpt::get_nrpt_rules().await?;
        let rows = rules
            .iter()
            .map(|r| {
                vec![
                    or_dash(r.display_name.as_deref().unwrap_or("")),
                    join_or_dash(&r.namespace),
                    join_or_dash(&r.name_servers),
                    if r.direct_access_enabled { "DirectAccess".to_string() } else { "—".to_string() },
                ]
            })
            .collect();
        Ok(table(&["Rule", "Namespace", "Name servers", "Type"], rows))
    }

    async fn routes_section() -> Result<String, String> {
        let routes = super::routing::get_routes().await?;
        let defaults: Vec<Vec<String>> = routes
            .iter()
            .filter(|r| r.destination_prefix == "0.0.0.0/0")
            .map(|r| {
                vec![
                    r.next_hop.clone(),
                    r.interface_alias.clone(),
                    (r.route_metric + r.interface_metric).to_string(),
                ]
            })
            .collect();
        let mut out = format!("{} IPv4 routes in total. Default routes:\n\n", routes.len());
        out.push_str(&table(&["Next hop", "Interface", "Total metric"], defaults));
        Ok(out)
    }

    async fn proxy_section() -> Result<String, String> {
        let p = super::proxy::get_winhttp_proxy().await?;
        Ok(if p.enabled {
            format!(
                "WinHTTP proxy: **{}**; bypass: {}\n",
                p.proxy_server.unwrap_or_default(),
                p.bypass_list.unwrap_or_else(|| "—".to_string())
            )
        } else {
            "WinHTTP proxy: direct access (none).\n".to_string()
        })
    }

    async fn hosts_section() -> Result<String, String> {
        let hosts = super::hosts::get_hosts_entries().await?;
        let active: Vec<_> = hosts.entries.iter().filter(|e| e.enabled).collect();
        let disabled = hosts.entries.len() - active.len();
        let rows = active.iter().map(|e| vec![e.ip.clone(), e.hostnames.join(" ")]).collect();
        let mut out = format!("{} active entries ({} disabled, not shown). Comments are left out.\n\n", active.len(), disabled);
        out.push_str(&table(&["IP", "Hostnames"], rows));
        Ok(out)
    }

    async fn ports_section() -> Result<String, String> {
        let snap = super::ports::get_port_usage().await?;
        let name_of = |pid: u32| {
            snap.processes
                .iter()
                .find(|p| p.pid == pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("PID {pid}"))
        };
        let mut listening: Vec<_> = snap
            .entries
            .iter()
            .filter(|e| e.protocol == "UDP" || e.state == "Listen")
            .collect();
        listening.sort_by_key(|e| (e.local_port, e.protocol.clone(), e.local_address.clone()));
        let total = listening.len();
        let rows = listening
            .into_iter()
            .take(120)
            .map(|e| {
                // IPv6 addresses are bracketed so the port isn't ambiguous.
                let address = if e.local_address.contains(':') {
                    format!("[{}]:{}", e.local_address, e.local_port)
                } else {
                    format!("{}:{}", e.local_address, e.local_port)
                };
                vec![e.protocol.clone(), address, name_of(e.pid), e.pid.to_string()]
            })
            .collect();
        let mut out = format!("{total} listening TCP ports / UDP endpoints");
        out.push_str(if total > 120 { " (first 120 shown):\n\n" } else { ":\n\n" });
        out.push_str(&table(&["Proto", "Local address", "Process", "PID"], rows));
        Ok(out)
    }

    async fn firewall_section() -> Result<String, String> {
        let snap = super::firewall::get_firewall().await?;
        let profiles = snap
            .profiles
            .iter()
            .map(|p| format!("{}: {}", p.name, if p.enabled { "on" } else { "off" }))
            .collect::<Vec<_>>()
            .join(", ");
        let enabled: Vec<_> = snap.rules.iter().filter(|r| r.enabled).collect();
        let blocks = enabled.iter().filter(|r| r.action == "Block").count();
        Ok(format!(
            "Profiles — {profiles}.\n\n{} rules, {} enabled ({} of them block rules).\n",
            snap.rules.len(),
            enabled.len(),
            blocks
        ))
    }

    async fn wsl_section() -> Result<String, String> {
        let s = super::wsl::get_wsl_status().await?;
        let mut out = String::new();
        if !s.installed {
            out.push_str("WSL isn't installed.\n");
        } else if s.unresponsive {
            out.push_str("**WSL is not responding** (the status check timed out).\n");
        } else {
            let rows = s
                .distros
                .iter()
                .map(|d| {
                    vec![
                        format!("{}{}", d.name, if d.is_default { " (default)" } else { "" }),
                        if d.running { "Running".to_string() } else { "Stopped".to_string() },
                        format!("WSL {}", d.version),
                    ]
                })
                .collect();
            out.push_str(&table(&["Distribution", "State", "Version"], rows));
        }
        let _ = writeln!(
            out,
            "\nDocker Desktop: {}.",
            match (s.docker_desktop.installed, s.docker_desktop.running) {
                (false, _) => "not installed",
                (true, true) => "installed, running",
                (true, false) => "installed, not running",
            }
        );
        let c = &s.settings;
        let set: Vec<String> = [
            ("memory", &c.memory),
            ("processors", &c.processors),
            ("swap", &c.swap),
            ("networkingMode", &c.networking_mode),
            ("autoMemoryReclaim", &c.auto_memory_reclaim),
            ("localhostForwarding", &c.localhost_forwarding),
            ("nestedVirtualization", &c.nested_virtualization),
        ]
        .iter()
        .filter_map(|(k, v)| v.as_ref().map(|v| format!("{k}={v}")))
        .collect();
        let _ = writeln!(out, ".wslconfig: {}.", if set.is_empty() { "defaults".to_string() } else { set.join(", ") });
        Ok(out)
    }

    async fn events_section() -> Result<String, String> {
        let result = super::eventlog::get_event_log("network".to_string(), "warnings".to_string(), 24, 25, None).await?;
        if let Some(err) = &result.error {
            return Ok(format!("_The event log reported: {err}_\n"));
        }
        if result.events.is_empty() {
            return Ok("No network or DNS errors or warnings in the last 24 hours.\n".to_string());
        }
        let mut out = String::from("Network and DNS errors and warnings from the last 24 hours (newest first, up to 25):\n\n");
        for e in &result.events {
            let level = match e.level {
                1 => "Critical",
                2 => "Error",
                _ => "Warning",
            };
            let first_line: String = e.message.lines().next().unwrap_or("").chars().take(200).collect();
            let _ = writeln!(out, "- `{}` **{level}** {} #{}: {}", e.time.get(..16).unwrap_or(&e.time), e.provider, e.id, first_line);
        }
        Ok(out)
    }

    fn title(id: &str) -> &'static str {
        match id {
            "system" => "System",
            "adapters" => "Network adapters",
            "dns" => "DNS client settings",
            "nrpt" => "NRPT rules",
            "routes" => "Routes",
            "proxy" => "Proxy",
            "hosts" => "Hosts file",
            "ports" => "Listening ports",
            "firewall" => "Firewall",
            "wsl" => "WSL and Docker",
            _ => "Recent network errors",
        }
    }

    async fn section(id: &str) -> Result<String, String> {
        match id {
            "system" => system_section().await,
            "adapters" => adapters_section().await,
            "dns" => dns_section().await,
            "nrpt" => nrpt_section().await,
            "routes" => routes_section().await,
            "proxy" => proxy_section().await,
            "hosts" => hosts_section().await,
            "ports" => ports_section().await,
            "firewall" => firewall_section().await,
            "wsl" => wsl_section().await,
            "events" => events_section().await,
            _ => Err("Unknown section.".to_string()),
        }
    }

    fn redactor(options: &ReportOptions) -> Redactor {
        let mut names = Vec::new();
        if options.hide_personal {
            for (var, replacement) in [("COMPUTERNAME", "<computer>"), ("USERNAME", "<user>")] {
                if let Ok(value) = std::env::var(var) {
                    if !value.trim().is_empty() {
                        names.push((value, replacement));
                    }
                }
            }
        }
        Redactor { names, macs: options.hide_personal, ips: options.mask_ips }
    }

    fn assemble(options: &ReportOptions, generated: u64, bodies: Vec<(String, Result<String, String>)>) -> String {
        let redactor = redactor(options);
        let mut out = String::new();
        let _ = writeln!(out, "# zagzig-tools diagnostic report\n");
        let _ = writeln!(
            out,
            "Generated {} by zagzig-tools v{}.\n",
            format_utc(generated),
            env!("CARGO_PKG_VERSION")
        );
        let mut hidden = Vec::new();
        if options.hide_personal {
            hidden.push("computer and user names and MAC addresses");
        }
        if options.mask_ips {
            hidden.push("the last part of IPv4 addresses");
        }
        let _ = writeln!(
            out,
            "> Contains no passwords or environment variable values. {}\n",
            if hidden.is_empty() {
                "Nothing has been hidden — review it before sharing.".to_string()
            } else {
                format!("Hidden: {}.", hidden.join("; "))
            }
        );
        for (id, result) in bodies {
            let _ = writeln!(out, "## {}\n", title(&id));
            let body = match result {
                Ok(body) => body,
                Err(err) => format!("_Couldn't collect this section: {}_\n", err.lines().next().unwrap_or("unknown error")),
            };
            // The system section carries version numbers that look like IPs.
            out.push_str(&redactor.apply(&body, id != "system"));
            out.push('\n');
        }
        out
    }

    #[tauri::command]
    pub async fn build_diagnostic_report(options: ReportOptions) -> Result<String, String> {
        // Keep the canonical order whatever order the UI sent.
        let ids: Vec<&str> = SECTIONS.iter().copied().filter(|s| options.sections.iter().any(|o| o == s)).collect();
        if ids.is_empty() {
            return Err("Choose at least one section.".to_string());
        }
        let handles: Vec<_> = ids
            .iter()
            .map(|id| {
                let id = id.to_string();
                tauri::async_runtime::spawn(async move {
                    let result = section(&id).await;
                    (id, result)
                })
            })
            .collect();
        let mut bodies = Vec::new();
        for handle in handles {
            bodies.push(handle.await.map_err(|err| format!("report task failed to run: {err}"))?);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Ok(assemble(&options, now, bodies))
    }

    #[tauri::command]
    pub async fn save_text_report(path: String, content: String) -> Result<(), String> {
        let target = std::path::PathBuf::from(&path);
        let ok_ext = target
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("txt"));
        if !ok_ext {
            return Err("The file name must end in .md or .txt.".to_string());
        }
        if content.len() > 5 * 1024 * 1024 {
            return Err("The report is too large.".to_string());
        }
        tauri::async_runtime::spawn_blocking(move || {
            std::fs::write(&target, content).map_err(|err| format!("couldn't save the report: {err}"))
        })
        .await
        .map_err(|err| format!("save task failed to run: {err}"))?
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn formats_utc_times() {
            assert_eq!(format_utc(0), "1970-01-01 00:00 UTC");
            assert_eq!(format_utc(1_700_000_000), "2023-11-14 22:13 UTC");
            assert_eq!(format_utc(1_791_251_559), "2026-10-06 01:52 UTC");
        }

        #[test]
        fn masks_mac_addresses() {
            assert_eq!(mask_macs("NIC 38-A7-46-73-8E-F8 up"), "NIC xx-xx-xx-xx-xx-xx up");
            assert_eq!(mask_macs("a0:b1:c2:d3:e4:f5"), "xx-xx-xx-xx-xx-xx");
            // Not MACs: too short, mixed separators, part of a longer run.
            for keep in ["38-A7-46-73-8E", "38-A7:46-73-8E-F8", "138-A7-46-73-8E-F8-00", "text only"] {
                assert_eq!(mask_macs(keep), keep, "{keep}");
            }
            assert_eq!(mask_macs("Jos\u{e9} 38-A7-46-73-8E-F8"), "Jos\u{e9} xx-xx-xx-xx-xx-xx");
        }

        #[test]
        fn masks_the_last_ipv4_octet_only() {
            assert_eq!(mask_ipv4("gateway 192.168.1.254, dns 10.0.0.1"), "gateway 192.168.1.x, dns 10.0.0.x");
            assert_eq!(mask_ipv4("172.16.28.48/24"), "172.16.28.x/24");
            assert_eq!(mask_ipv4("127.0.0.1 and 0.0.0.0 and 127.1.2.3"), "127.0.0.1 and 0.0.0.0 and 127.1.2.3");
            for keep in ["10.0.26200.1234.5", "999.1.1.1", "1.2.3", "v1.2.3.4.5", "no ips here", "300.1.1.1"] {
                assert_eq!(mask_ipv4(keep), keep, "{keep}");
            }
        }

        #[test]
        fn replaces_names_case_insensitively_but_not_tiny_ones() {
            assert_eq!(replace_ci(r"C:\Users\Rafli.Athala\x and rafli.athala", "rafli.athala", "<user>"), r"C:\Users\<user>\x and <user>");
            assert_eq!(replace_ci("a b ab", "ab", "<x>"), "a b ab", "names under three characters are left alone");
        }

        fn options(hide: bool, ips: bool) -> ReportOptions {
            ReportOptions { sections: vec!["system".into()], hide_personal: hide, mask_ips: ips }
        }

        #[test]
        fn assembles_a_report_with_failures_and_redaction() {
            std::env::set_var("COMPUTERNAME", "TESTBOX");
            let bodies = vec![
                ("system".to_string(), Ok("Windows 10.0.26200.1234 on TESTBOX\n".to_string())),
                ("adapters".to_string(), Ok("TESTBOX 192.168.1.20 38-A7-46-73-8E-F8\n".to_string())),
                ("wsl".to_string(), Err("wsl.exe failed\nsecond line".to_string())),
            ];
            let hidden = assemble(&options(true, true), 0, bodies.clone());
            assert!(hidden.contains("## System") && hidden.contains("Windows 10.0.26200.1234 on <computer>"), "{hidden}");
            assert!(hidden.contains("<computer> 192.168.1.x xx-xx-xx-xx-xx-xx"), "{hidden}");
            assert!(hidden.contains("_Couldn't collect this section: wsl.exe failed_"));
            assert!(hidden.contains("Hidden: computer and user names and MAC addresses; the last part of IPv4 addresses."));

            let plain = assemble(&options(false, false), 0, bodies);
            assert!(plain.contains("TESTBOX 192.168.1.20 38-A7-46-73-8E-F8"));
            assert!(plain.contains("Nothing has been hidden"));
        }

        #[test]
        fn tables_escape_pipes_and_newlines() {
            let t = table(&["a", "b"], vec![vec!["x|y".into(), "line1\nline2".into()]]);
            assert!(t.contains("x\\|y") && t.contains("line1 line2"));
            assert_eq!(table(&["a"], vec![]), "_None._\n");
        }

        // Gathers every section from the machine the tests run on, so it's
        // opt-in: `cargo test -- --ignored --nocapture real_report`.
        #[test]
        #[ignore]
        fn real_report() {
            let options = ReportOptions {
                sections: SECTIONS.iter().map(|s| s.to_string()).collect(),
                hide_personal: true,
                mask_ips: true,
            };
            let started = std::time::Instant::now();
            let report = tauri::async_runtime::block_on(build_diagnostic_report(options)).unwrap();
            println!("{report}\n--- generated in {:.1}s, {} bytes", started.elapsed().as_secs_f32(), report.len());
            if let Ok(path) = std::env::var("ZAGZIG_REPORT_OUT") {
                std::fs::write(path, &report).unwrap();
            }
        }

        #[test]
        fn refuses_an_empty_selection_and_bad_extensions() {
            let empty = ReportOptions { sections: vec![], hide_personal: true, mask_ips: false };
            assert!(tauri::async_runtime::block_on(build_diagnostic_report(empty)).is_err());
            assert!(tauri::async_runtime::block_on(save_text_report("C:\\x\\report.exe".into(), "x".into())).is_err());
        }
    }
}

// Backs the "Proxy Settings" feature: the WinHTTP proxy (`netsh winhttp`) is
// a separate, machine-wide setting from the browser/"Internet Options" proxy
// that Settings exposes — plenty of things (Windows Update's underlying
// service, many CLI tools and background agents) only honor this one, and
// it has no GUI at all anywhere in Windows.
mod proxy {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell};

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WinHttpProxy {
        pub enabled: bool,
        pub proxy_server: Option<String>,
        pub bypass_list: Option<String>,
    }

    fn parse_winhttp_proxy_output(output: &str) -> WinHttpProxy {
        let mut proxy_server = None;
        let mut bypass_list = None;

        for line in output.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("Proxy Server(s)") {
                if let Some((_, value)) = rest.split_once(':') {
                    let value = value.trim();
                    if !value.is_empty() {
                        proxy_server = Some(value.to_string());
                    }
                }
            } else if let Some(rest) = line.strip_prefix("Bypass List") {
                if let Some((_, value)) = rest.split_once(':') {
                    let value = value.trim();
                    if !value.is_empty() && !value.eq_ignore_ascii_case("(none)") {
                        bypass_list = Some(value.to_string());
                    }
                }
            }
        }

        WinHttpProxy {
            enabled: proxy_server.is_some(),
            proxy_server,
            bypass_list,
        }
    }

    #[tauri::command]
    pub async fn get_winhttp_proxy() -> Result<WinHttpProxy, String> {
        let output = run_powershell("netsh winhttp show proxy", &[]).await?;
        Ok(parse_winhttp_proxy_output(&output))
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct SetProxyRequest<'a> {
        #[serde(rename = "ProxyServer")]
        proxy_server: &'a str,
        #[serde(rename = "BypassList")]
        bypass_list: Option<&'a str>,
    }

    const SET_WINHTTP_PROXY_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $netshArgs = @('winhttp', 'set', 'proxy', "proxy-server=$($req.ProxyServer)")
    if ($req.BypassList) { $netshArgs += "bypass-list=$($req.BypassList)" }
    $output = & netsh @netshArgs 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn set_winhttp_proxy(proxy_server: String, bypass_list: Option<String>) -> Result<(), String> {
        let proxy_server = proxy_server.trim();
        if proxy_server.is_empty() {
            return Err("Enter a proxy address.".to_string());
        }
        let bypass_list = bypass_list.as_deref().map(str::trim).filter(|s| !s.is_empty());

        let request = SetProxyRequest {
            proxy_server,
            bypass_list,
        };
        let input = serde_json::to_string(&request)
            .map_err(|err| format!("failed to prepare request: {err}"))?;

        let trimmed = run_elevated(SET_WINHTTP_PROXY_WORKER_SCRIPT, &input).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    const RESET_WINHTTP_PROXY_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $output = & netsh winhttp reset proxy 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn reset_winhttp_proxy() -> Result<(), String> {
        let trimmed = run_elevated(RESET_WINHTTP_PROXY_WORKER_SCRIPT, "").await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    // "Import from system proxy" is `netsh winhttp import proxy source=ie` —
    // WinHTTP and the "Internet Options" proxy (what Settings > Network >
    // Proxy actually configures) are independent, so this is the one-click
    // fix for the common case of "I set a proxy in Settings but some tool
    // still isn't using it."
    const IMPORT_WINHTTP_PROXY_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $output = & netsh winhttp import proxy source=ie 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) { throw $output.Trim() }
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    #[tauri::command]
    pub async fn import_winhttp_proxy_from_system() -> Result<(), String> {
        let trimmed = run_elevated(IMPORT_WINHTTP_PROXY_WORKER_SCRIPT, "").await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }
}

// Backs the "Certificate Store" feature: certmgr.msc's tree-view-plus-tiny-
// columns UI makes it hard to see what's actually installed and why. This
// covers the read/browse/prune workflow (list, view details, delete,
// export-public) across the stores people actually care about — not a full
// certmgr replacement (no import/PFX-with-key-export), which would mean
// handling private-key material and passwords for comparatively rare use.
mod certificates {
    use serde::{Deserialize, Serialize};

    use crate::{run_elevated, run_powershell, string_or_vec};

    // Only these (scope, store) pairs are ever interpolated into a script,
    // and only as this fixed literal — never the caller's raw strings — so
    // an unrecognized pair is rejected outright instead of ever reaching
    // PowerShell. Every script that uses one of these paths must start with
    // `crate::ENSURE_CERT_DRIVE` — see its doc comment for why.
    fn cert_store_path(scope: &str, store: &str) -> Result<&'static str, String> {
        match (scope, store) {
            ("CurrentUser", "My") => Ok("Cert:\\CurrentUser\\My"),
            ("CurrentUser", "Root") => Ok("Cert:\\CurrentUser\\Root"),
            ("CurrentUser", "CA") => Ok("Cert:\\CurrentUser\\CA"),
            ("CurrentUser", "TrustedPublisher") => Ok("Cert:\\CurrentUser\\TrustedPublisher"),
            ("LocalMachine", "My") => Ok("Cert:\\LocalMachine\\My"),
            ("LocalMachine", "Root") => Ok("Cert:\\LocalMachine\\Root"),
            ("LocalMachine", "CA") => Ok("Cert:\\LocalMachine\\CA"),
            ("LocalMachine", "TrustedPublisher") => Ok("Cert:\\LocalMachine\\TrustedPublisher"),
            _ => Err("Unknown certificate store.".to_string()),
        }
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CertificateDetail {
        pub thumbprint: String,
        pub subject: String,
        pub issuer: String,
        pub serial_number: String,
        pub friendly_name: String,
        pub not_before: String,
        pub not_after: String,
        pub has_private_key: bool,
        pub is_expired: bool,
        pub enhanced_key_usages: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawCertificateDetail {
        thumbprint: String,
        subject: String,
        issuer: String,
        #[serde(default)]
        serial_number: String,
        #[serde(default)]
        friendly_name: String,
        not_before: String,
        not_after: String,
        has_private_key: bool,
        is_expired: bool,
        #[serde(deserialize_with = "string_or_vec", default)]
        enhanced_key_usages: Vec<String>,
    }

    // Reading a certificate store — even the LocalMachine ones — never
    // needs elevation; only writing to a LocalMachine store does. `$env:
    // ZAGZIG_CERT_STORE_PATH` carries the (already-validated) store literal
    // in so this script stays fixed regardless of which store is browsed.
    // See the comment on nrpt::get_nrpt_rules for why this builds the object
    // with ForEach-Object + [ordered]@{} instead of Select-Object's
    // calculated-property syntax: on Windows PowerShell 5.1 (what this app
    // actually spawns), the latter serializes EnhancedKeyUsages as
    // `{"value": [...], "Count": N}` instead of a plain array, which
    // string_or_vec then silently collapses to empty — every certificate
    // with a real EKU restriction showed "Any purpose" in the UI instead.
    // Sorting happens before the conversion since Sort-Object -Property on
    // a hashtable is a different (and here, unnecessary) question to answer.
    const LIST_CERTIFICATES_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$storePath = $env:ZAGZIG_CERT_STORE_PATH
$now = Get-Date
@(Get-ChildItem -LiteralPath $storePath | Sort-Object Subject | ForEach-Object {
    [ordered]@{
        Thumbprint = $_.Thumbprint
        Subject = $_.Subject
        Issuer = $_.Issuer
        SerialNumber = $_.SerialNumber
        FriendlyName = $_.FriendlyName
        NotBefore = $_.NotBefore.ToString('yyyy-MM-dd')
        NotAfter = $_.NotAfter.ToString('yyyy-MM-dd')
        HasPrivateKey = [bool]$_.HasPrivateKey
        IsExpired = [bool]($_.NotAfter -lt $now)
        EnhancedKeyUsages = @(if ($_.EnhancedKeyUsageList) { $_.EnhancedKeyUsageList | ForEach-Object { $_.FriendlyName } })
    }
}) | ConvertTo-Json -Depth 4 -Compress
"#;

    #[tauri::command]
    pub async fn get_certificates(scope: String, store: String) -> Result<Vec<CertificateDetail>, String> {
        let path = cert_store_path(&scope, &store)?;
        let script = format!("{}{LIST_CERTIFICATES_SCRIPT}", crate::ENSURE_CERT_DRIVE);
        let trimmed = run_powershell(&script, &[("ZAGZIG_CERT_STORE_PATH", path)]).await?;

        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let raw: Vec<RawCertificateDetail> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(raw
            .into_iter()
            .map(|r| CertificateDetail {
                thumbprint: r.thumbprint,
                subject: r.subject,
                issuer: r.issuer,
                serial_number: r.serial_number,
                friendly_name: r.friendly_name,
                not_before: r.not_before,
                not_after: r.not_after,
                has_private_key: r.has_private_key,
                is_expired: r.is_expired,
                enhanced_key_usages: r.enhanced_key_usages,
            })
            .collect())
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ElevatedResult {
        success: bool,
        #[serde(default)]
        error: Option<String>,
    }

    #[derive(Debug, Serialize)]
    struct DeleteCertRequest<'a> {
        #[serde(rename = "StorePath")]
        store_path: &'a str,
        #[serde(rename = "Thumbprint")]
        thumbprint: &'a str,
    }

    const DELETE_CERT_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$storePath = $env:ZAGZIG_CERT_STORE_PATH
$thumbprint = $env:ZAGZIG_CERT_THUMBPRINT
$itemPath = Join-Path $storePath $thumbprint
if (-not (Test-Path -LiteralPath $itemPath)) {
    throw 'Certificate not found.'
}
Remove-Item -LiteralPath $itemPath -DeleteKey -Force
"#;

    // `param()` must be the very first statement in the file, so the
    // `Cert:`-drive guard (see `crate::ENSURE_CERT_DRIVE`) is inlined inside
    // the `try` below instead of prepended like the other scripts here —
    // any failure there is then reported through the same `catch`.
    const DELETE_CERT_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    if (-not (Get-PSDrive -Name Cert -ErrorAction SilentlyContinue)) {
        Import-Module (Join-Path $PSHOME 'Modules\Microsoft.PowerShell.Security\Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
    }
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $itemPath = Join-Path $req.StorePath $req.Thumbprint
    if (-not (Test-Path -LiteralPath $itemPath)) {
        throw 'Certificate not found.'
    }
    Remove-Item -LiteralPath $itemPath -DeleteKey -Force
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    // CurrentUser stores are the signed-in account's own — no elevation
    // needed to change them. LocalMachine stores affect every account on
    // the machine, so removing from one of those goes through the same
    // single-UAC-prompt elevation as every other admin-only action here.
    #[tauri::command]
    pub async fn delete_certificate(scope: String, store: String, thumbprint: String) -> Result<(), String> {
        let path = cert_store_path(&scope, &store)?;
        let thumbprint = thumbprint.trim().to_string();
        if thumbprint.is_empty() {
            return Err("Missing certificate thumbprint.".to_string());
        }

        if scope == "LocalMachine" {
            let request = DeleteCertRequest {
                store_path: path,
                thumbprint: &thumbprint,
            };
            let input = serde_json::to_string(&request)
                .map_err(|err| format!("failed to prepare request: {err}"))?;

            let trimmed = run_elevated(DELETE_CERT_WORKER_SCRIPT, &input).await?;
            let parsed: ElevatedResult = serde_json::from_str(&trimmed)
                .map_err(|err| format!("failed to parse powershell output: {err}"))?;

            if parsed.success {
                Ok(())
            } else {
                Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
            }
        } else {
            let script = format!("{}{DELETE_CERT_SCRIPT}", crate::ENSURE_CERT_DRIVE);
            run_powershell(
                &script,
                &[
                    ("ZAGZIG_CERT_STORE_PATH", path),
                    ("ZAGZIG_CERT_THUMBPRINT", &thumbprint),
                ],
            )
            .await
            .map(|_| ())
        }
    }

    // --- Importing a certificate file ------------------------------------
    //
    // Two steps, so nothing is trusted blind: `inspect_certificate_file`
    // reads a file and reports what's in it (nothing is written anywhere),
    // then `import_certificate` adds it to a store once the user has seen
    // that and confirmed. A LocalMachine store needs elevation; a
    // CurrentUser one doesn't (Windows itself shows a confirmation for
    // CurrentUser\Root).

    const CERT_LOAD_SCRIPT: &str = include_str!("../scripts/cert_load.ps1");
    const CERT_INSPECT_SCRIPT: &str = include_str!("../scripts/cert_inspect.ps1");
    const CERT_IMPORT_CORE: &str = include_str!("../scripts/cert_import.ps1");

    const MAX_CERT_FILE_BYTES: u64 = 5 * 1024 * 1024;
    const CERT_FILE_EXTENSIONS: [&str; 8] = ["cer", "crt", "der", "pem", "p7b", "p7c", "pfx", "p12"];

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CertFileCert {
        pub subject: String,
        pub issuer: String,
        pub thumbprint: String,
        pub not_before: String,
        pub not_after: String,
        pub self_signed: bool,
        pub is_ca: bool,
        pub has_private_key: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CertFileInfo {
        #[serde(default, deserialize_with = "crate::value_or_vec")]
        pub certs: Vec<CertFileCert>,
        pub needs_password: bool,
        pub error: Option<String>,
    }

    fn check_cert_file(path: &str) -> Result<(), String> {
        let p = std::path::Path::new(path);
        let extension = p
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if !CERT_FILE_EXTENSIONS.contains(&extension.as_str()) {
            return Err("Choose a certificate file (.cer, .crt, .der, .pem, .p7b, .pfx or .p12).".to_string());
        }
        let meta = std::fs::metadata(p).map_err(|err| format!("Couldn't read the file: {err}"))?;
        if !meta.is_file() {
            return Err("That isn't a file.".to_string());
        }
        if meta.len() > MAX_CERT_FILE_BYTES {
            return Err("The file is larger than 5 MB.".to_string());
        }
        Ok(())
    }

    #[tauri::command]
    pub async fn inspect_certificate_file(path: String, password: Option<String>) -> Result<CertFileInfo, String> {
        check_cert_file(&path)?;
        let password = password.unwrap_or_default();
        let script = format!("{CERT_LOAD_SCRIPT}\n{CERT_INSPECT_SCRIPT}");
        let trimmed = run_powershell(
            &script,
            &[("ZAGZIG_CERT_FILE", path.as_str()), ("ZAGZIG_CERT_PASSWORD", password.as_str())],
        )
        .await?;
        serde_json::from_str(&trimmed).map_err(|err| format!("failed to parse powershell output: {err}"))
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ImportResult {
        pub count: u32,
        pub thumbprints: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct ImportReply {
        success: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        count: u32,
        #[serde(default, deserialize_with = "string_or_vec")]
        thumbprints: Vec<String>,
    }

    fn import_user_script() -> String {
        format!(
            r#"{CERT_LOAD_SCRIPT}
$ErrorActionPreference = 'Stop'
try {{
    $req = ConvertFrom-Json -InputObject $env:ZAGZIG_CERT_IMPORT_REQUEST
    $result = & {{ {CERT_IMPORT_CORE} }}
    $result | ConvertTo-Json -Compress
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress
}}
"#
        )
    }

    fn import_machine_worker() -> String {
        format!(
            r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {{
    {CERT_LOAD_SCRIPT}
    $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json
    $result = & {{ {CERT_IMPORT_CORE} }}
    $result | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}} catch {{
    $ex = $_.Exception
    while ($ex.InnerException) {{ $ex = $ex.InnerException }}
    @{{ Success = $false; Error = $ex.Message }} | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}}
"#
        )
    }

    #[tauri::command]
    pub async fn import_certificate(
        scope: String,
        store: String,
        path: String,
        password: Option<String>,
    ) -> Result<ImportResult, String> {
        // Validates the pair; `store` is then one of the four fixed names.
        cert_store_path(&scope, &store)?;
        check_cert_file(&path)?;

        let request = serde_json::json!({
            "Path": path,
            "Password": password.unwrap_or_default(),
            "Scope": scope,
            "Store": store,
        })
        .to_string();

        let raw = if scope == "LocalMachine" {
            run_elevated(&import_machine_worker(), &request).await?
        } else {
            run_powershell(&import_user_script(), &[("ZAGZIG_CERT_IMPORT_REQUEST", request.as_str())]).await?
        };
        let reply: ImportReply = serde_json::from_str(raw.trim())
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if reply.success {
            Ok(ImportResult { count: reply.count, thumbprints: reply.thumbprints })
        } else {
            Err(reply.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[cfg(test)]
    mod import_tests {
        use super::*;

        #[test]
        fn import_scripts_are_valid_powershell() {
            assert_eq!(crate::powershell_syntax_errors(&import_user_script()), Vec::<String>::new());
            assert_eq!(crate::powershell_syntax_errors(&import_machine_worker()), Vec::<String>::new());
            assert_eq!(
                crate::powershell_syntax_errors(&format!("{CERT_LOAD_SCRIPT}\n{CERT_INSPECT_SCRIPT}")),
                Vec::<String>::new()
            );
        }

        #[test]
        fn only_certificate_files_are_accepted() {
            assert!(check_cert_file("C:\\x\\a.txt").is_err());
            assert!(check_cert_file("C:\\x\\a.exe").is_err());
            assert!(check_cert_file("C:\\definitely\\missing\\a.cer").is_err());
        }

        #[test]
        fn parses_replies() {
            let ok: ImportReply = serde_json::from_str(r#"{"Success":true,"Count":2,"Thumbprints":["A","B"]}"#).unwrap();
            assert!(ok.success && ok.count == 2 && ok.thumbprints.len() == 2);
            let one: ImportReply = serde_json::from_str(r#"{"Success":true,"Count":1,"Thumbprints":"A"}"#).unwrap();
            assert_eq!(one.thumbprints, vec!["A"]);
            let info: CertFileInfo = serde_json::from_str(r#"{"certs":[],"needsPassword":true,"error":null}"#).unwrap();
            assert!(info.needs_password && info.certs.is_empty());
        }
    }

    // Exports the public certificate only (.cer) — reading any store and
    // writing to a user-chosen destination both need no elevation, unlike
    // deleting from a LocalMachine store above.
    const EXPORT_CERT_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$storePath = $env:ZAGZIG_CERT_STORE_PATH
$thumbprint = $env:ZAGZIG_CERT_THUMBPRINT
$destPath = $env:ZAGZIG_CERT_DEST_PATH
$cert = Get-Item -LiteralPath (Join-Path $storePath $thumbprint) -ErrorAction Stop
Export-Certificate -Cert $cert -FilePath $destPath -Type CERT | Out-Null
"#;

    #[tauri::command]
    pub async fn export_certificate(
        scope: String,
        store: String,
        thumbprint: String,
        destination_path: String,
    ) -> Result<(), String> {
        let path = cert_store_path(&scope, &store)?;
        let thumbprint = thumbprint.trim();
        let destination_path = destination_path.trim();
        if thumbprint.is_empty() || destination_path.is_empty() {
            return Err("Missing certificate or destination path.".to_string());
        }

        let script = format!("{}{EXPORT_CERT_SCRIPT}", crate::ENSURE_CERT_DRIVE);
        run_powershell(
            &script,
            &[
                ("ZAGZIG_CERT_STORE_PATH", path),
                ("ZAGZIG_CERT_THUMBPRINT", thumbprint),
                ("ZAGZIG_CERT_DEST_PATH", destination_path),
            ],
        )
        .await
        .map(|_| ())
    }
}

// Backs the "DNS Cache" feature: a viewer + flush button for the resolver
// cache `ipconfig /displaydns` and `ipconfig /flushdns` manage from the
// command line, with no GUI anywhere in Windows. Reading and flushing are
// both plain (non-elevated) commands here — verified directly that
// Clear-DnsClientCache succeeds from a standard, unelevated session, unlike
// every other write in this app, which is why this is the one feature with
// no admin-gating anywhere in its UI.
mod dns_cache {
    use serde::{Deserialize, Serialize};

    use crate::run_powershell;

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DnsCacheEntry {
        pub name: String,
        pub record_type: String,
        pub data: Option<String>,
        pub time_to_live: u32,
        pub section: String,
        pub status: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct RawDnsCacheEntry {
        entry: String,
        #[serde(default)]
        data: Option<String>,
        time_to_live: u32,
        section: u32,
        status: i64,
        #[serde(rename = "Type")]
        record_type: u32,
    }

    // DNS RR type numbers, per Get-DnsClientCache's `Type` field — only the
    // ones actually likely to show up in a client resolver cache.
    fn record_type_name(value: u32) -> String {
        match value {
            1 => "A".to_string(),
            2 => "NS".to_string(),
            5 => "CNAME".to_string(),
            6 => "SOA".to_string(),
            12 => "PTR".to_string(),
            15 => "MX".to_string(),
            16 => "TXT".to_string(),
            28 => "AAAA".to_string(),
            33 => "SRV".to_string(),
            255 => "ANY".to_string(),
            other => format!("Type {other}"),
        }
    }

    // DNS message section numbers (RFC 1035). Negative-cache entries (a
    // lookup that came back empty, e.g. an AAAA query on a v4-only name)
    // stay in Question with no Data and a non-zero Status.
    fn section_name(value: u32) -> String {
        match value {
            0 => "Question".to_string(),
            1 => "Answer".to_string(),
            2 => "Authority".to_string(),
            3 => "Additional".to_string(),
            other => format!("Section {other}"),
        }
    }

    // Win32 DNS API status/error codes as seen in this field; 0 is success
    // and maps to None (nothing to show). The other two are by far the most
    // common non-zero values in practice; anything else still shows a
    // number rather than guessing at a label.
    fn status_label(value: i64) -> Option<String> {
        match value {
            0 => None,
            9003 => Some("Name not found".to_string()),
            9501 => Some("No records of this type".to_string()),
            other => Some(format!("Error {other}")),
        }
    }

    #[tauri::command]
    pub async fn get_dns_cache() -> Result<Vec<DnsCacheEntry>, String> {
        let trimmed = run_powershell(
            "@(Get-DnsClientCache | Select-Object Entry, Data, TimeToLive, Section, Status, Type) \
| ConvertTo-Json -Depth 3 -Compress",
            &[],
        )
        .await?;

        if trimmed.is_empty() {
            return Ok(vec![]);
        }

        let raw: Vec<RawDnsCacheEntry> = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;

        Ok(raw
            .into_iter()
            .map(|r| DnsCacheEntry {
                name: r.entry,
                record_type: record_type_name(r.record_type),
                data: r.data,
                time_to_live: r.time_to_live,
                section: section_name(r.section),
                status: status_label(r.status),
            })
            .collect())
    }

    #[tauri::command]
    pub async fn flush_dns_cache() -> Result<(), String> {
        run_powershell("Clear-DnsClientCache", &[]).await?;
        Ok(())
    }
}

// Saved snapshots of the network setup (adapters, routes, NRPT, proxy, hosts)
// for "it worked yesterday": take one while things work, compare later. Each
// section is a map of item -> field -> value so the frontend can diff any two
// snapshots without knowing what the fields mean. Values that change on their
// own (traffic counters, timestamps) are left out so they don't drown real
// changes.
mod snapshots {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    const SNAPSHOT_LIMIT: usize = 50;
    const FORMAT_VERSION: u32 = 1;

    type Fields = BTreeMap<String, String>;
    type Items = BTreeMap<String, Fields>;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Snapshot {
        pub version: u32,
        pub id: String,
        pub label: String,
        pub taken_at: u64,
        pub computer: String,
        pub sections: BTreeMap<String, Items>,
        /// Sections that couldn't be read, with the reason.
        #[serde(default)]
        pub errors: BTreeMap<String, String>,
    }

    #[derive(Debug, Clone, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SnapshotInfo {
        pub id: String,
        pub label: String,
        pub taken_at: u64,
        pub computer: String,
        pub item_count: usize,
        pub error_count: usize,
    }

    impl Snapshot {
        fn info(&self) -> SnapshotInfo {
            SnapshotInfo {
                id: self.id.clone(),
                label: self.label.clone(),
                taken_at: self.taken_at,
                computer: self.computer.clone(),
                item_count: self.sections.values().map(|s| s.len()).sum(),
                error_count: self.errors.len(),
            }
        }
    }

    fn join(list: &[String]) -> String {
        list.join(", ")
    }

    fn fields(pairs: &[(&str, String)]) -> Fields {
        pairs
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    // Two items with the same key (two routes that differ only in metric,
    // say) must not overwrite each other.
    fn insert_unique(items: &mut Items, key: String, value: Fields) {
        let mut k = key.clone();
        let mut n = 2;
        while items.contains_key(&k) {
            k = format!("{key} #{n}");
            n += 1;
        }
        items.insert(k, value);
    }

    async fn adapters() -> Result<Items, String> {
        let mut items = Items::new();
        for a in super::adapters::get_network_adapters().await? {
            let value = fields(&[
                ("Status", a.status.clone()),
                ("IPv4", join(&a.ipv4)),
                ("IPv6", join(&a.ipv6)),
                ("Gateway", join(&a.gateways)),
                ("DNS servers", join(&a.dns_servers)),
                ("Addressing", if a.dhcp { "DHCP".into() } else { "static".into() }),
                ("MAC", a.mac_address.clone().unwrap_or_default()),
                ("Link speed", a.link_speed.clone().unwrap_or_default()),
                ("MTU", a.mtu.map(|m| m.to_string()).unwrap_or_default()),
                ("Virtual", if a.is_virtual { "yes".into() } else { String::new() }),
            ]);
            insert_unique(&mut items, a.name.clone(), value);
        }
        Ok(items)
    }

    async fn routes() -> Result<Items, String> {
        let mut items = Items::new();
        for r in super::routing::get_routes().await? {
            let key = format!("{} via {} ({})", r.destination_prefix, r.next_hop, r.interface_alias);
            let value = fields(&[
                ("Metric", (r.route_metric + r.interface_metric).to_string()),
                ("Protocol", r.protocol.clone()),
                ("Store", r.store.clone()),
            ]);
            insert_unique(&mut items, key, value);
        }
        Ok(items)
    }

    async fn nrpt() -> Result<Items, String> {
        let mut items = Items::new();
        for r in super::nrpt::get_nrpt_rules().await? {
            let key = if r.namespace.is_empty() { r.name.clone() } else { join(&r.namespace) };
            let value = fields(&[
                ("Name servers", join(&r.name_servers)),
                ("Comment", r.comment.clone().unwrap_or_default()),
                ("DNSSEC", if r.dns_sec_enabled { "on".into() } else { String::new() }),
                ("DirectAccess", if r.direct_access_enabled { "on".into() } else { String::new() }),
                ("DirectAccess servers", join(&r.direct_access_dns_servers)),
            ]);
            insert_unique(&mut items, key, value);
        }
        Ok(items)
    }

    async fn proxy() -> Result<Items, String> {
        let p = super::proxy::get_winhttp_proxy().await?;
        let mut items = Items::new();
        items.insert(
            "WinHTTP proxy".to_string(),
            fields(&[
                (
                    "Server",
                    if p.enabled { p.proxy_server.clone().unwrap_or_default() } else { "direct access".into() },
                ),
                ("Bypass list", p.bypass_list.clone().unwrap_or_default()),
            ]),
        );
        Ok(items)
    }

    // Keyed by host name, since "where does this name point" is the question.
    async fn hosts() -> Result<Items, String> {
        let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for e in super::hosts::get_hosts_entries().await?.entries.into_iter().filter(|e| e.enabled) {
            for name in e.hostnames {
                by_name.entry(name.to_lowercase()).or_default().push(e.ip.clone());
            }
        }
        Ok(by_name
            .into_iter()
            .map(|(name, ips)| (name, fields(&[("Address", ips.join(", "))])))
            .collect())
    }

    pub async fn collect(label: String) -> Snapshot {
        // Each section shells out to PowerShell, so read them at the same time.
        use tauri::async_runtime::spawn;
        let (a, r, n, p, h) = (spawn(adapters()), spawn(routes()), spawn(nrpt()), spawn(proxy()), spawn(hosts()));
        let done = |res: Result<Result<Items, String>, tauri::Error>| res.map_err(|e| e.to_string()).and_then(|x| x);
        let (a, r, n, p, h) = (done(a.await), done(r.await), done(n.await), done(p.await), done(h.await));
        let mut sections = BTreeMap::new();
        let mut errors = BTreeMap::new();
        for (id, result) in [("adapters", a), ("routes", r), ("nrpt", n), ("proxy", p), ("hosts", h)] {
            match result {
                Ok(items) => {
                    sections.insert(id.to_string(), items);
                }
                Err(err) => {
                    errors.insert(id.to_string(), err);
                }
            }
        }
        let taken_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Snapshot {
            version: FORMAT_VERSION,
            id: format!("snap-{taken_at}"),
            label,
            taken_at,
            computer: std::env::var("COMPUTERNAME").unwrap_or_default(),
            sections,
            errors,
        }
    }

    fn snapshots_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
        use tauri::Manager;
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|err| format!("couldn't find the app data folder: {err}"))?
            .join("snapshots");
        std::fs::create_dir_all(&dir).map_err(|err| format!("couldn't create the snapshots folder: {err}"))?;
        Ok(dir)
    }

    fn valid_id(id: &str) -> bool {
        id.strip_prefix("snap-")
            .is_some_and(|n| !n.is_empty() && n.len() <= 20 && n.chars().all(|c| c.is_ascii_digit()))
    }

    fn clean_label(label: &str) -> Result<String, String> {
        let label = label.trim();
        if label.chars().count() > 80 || label.chars().any(char::is_control) {
            return Err("The name is too long or has characters that can't be used.".to_string());
        }
        Ok(label.to_string())
    }

    fn read_in(dir: &std::path::Path, id: &str) -> Result<Snapshot, String> {
        if !valid_id(id) {
            return Err("Unknown snapshot.".to_string());
        }
        let text = std::fs::read_to_string(dir.join(format!("{id}.json")))
            .map_err(|err| format!("couldn't read the snapshot: {err}"))?;
        serde_json::from_str(&text).map_err(|err| format!("the snapshot file is damaged: {err}"))
    }

    fn list_in(dir: &std::path::Path) -> Vec<Snapshot> {
        let mut out: Vec<Snapshot> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                let id = name.strip_suffix(".json")?;
                read_in(dir, id).ok()
            })
            .collect();
        out.sort_by(|a, b| b.taken_at.cmp(&a.taken_at));
        out
    }

    fn save_in(dir: &std::path::Path, snapshot: &Snapshot) -> Result<(), String> {
        let json = serde_json::to_string_pretty(snapshot).map_err(|err| err.to_string())?;
        std::fs::write(dir.join(format!("{}.json", snapshot.id)), json)
            .map_err(|err| format!("couldn't save the snapshot: {err}"))?;
        // Keep the newest ones only.
        for old in list_in(dir).into_iter().skip(SNAPSHOT_LIMIT) {
            let _ = std::fs::remove_file(dir.join(format!("{}.json", old.id)));
        }
        Ok(())
    }

    #[tauri::command]
    pub async fn take_snapshot(app: tauri::AppHandle, label: String) -> Result<SnapshotInfo, String> {
        let label = clean_label(&label)?;
        let snapshot = collect(label).await;
        if snapshot.sections.is_empty() {
            let reasons: Vec<String> = snapshot.errors.values().cloned().collect();
            return Err(format!("Nothing could be read: {}", reasons.join("; ")));
        }
        save_in(&snapshots_dir(&app)?, &snapshot)?;
        Ok(snapshot.info())
    }

    /// The current state, not saved, to compare a snapshot against.
    #[tauri::command]
    pub async fn current_snapshot() -> Result<Snapshot, String> {
        Ok(collect(String::new()).await)
    }

    #[tauri::command]
    pub async fn list_snapshots(app: tauri::AppHandle) -> Result<Vec<SnapshotInfo>, String> {
        Ok(list_in(&snapshots_dir(&app)?).iter().map(Snapshot::info).collect())
    }

    #[tauri::command]
    pub async fn get_snapshot(app: tauri::AppHandle, id: String) -> Result<Snapshot, String> {
        read_in(&snapshots_dir(&app)?, &id)
    }

    #[tauri::command]
    pub async fn rename_snapshot(app: tauri::AppHandle, id: String, label: String) -> Result<(), String> {
        let dir = snapshots_dir(&app)?;
        let mut snapshot = read_in(&dir, &id)?;
        snapshot.label = clean_label(&label)?;
        save_in(&dir, &snapshot)
    }

    #[tauri::command]
    pub async fn delete_snapshot(app: tauri::AppHandle, id: String) -> Result<(), String> {
        if !valid_id(&id) {
            return Err("Unknown snapshot.".to_string());
        }
        std::fs::remove_file(snapshots_dir(&app)?.join(format!("{id}.json")))
            .map_err(|err| format!("couldn't delete the snapshot: {err}"))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn snap(id: u64) -> Snapshot {
            let mut items = Items::new();
            items.insert("Wi-Fi".into(), fields(&[("IPv4", "10.0.0.5".into()), ("Gateway", String::new())]));
            let mut sections = BTreeMap::new();
            sections.insert("adapters".into(), items);
            Snapshot {
                version: FORMAT_VERSION,
                id: format!("snap-{id}"),
                label: format!("snapshot {id}"),
                taken_at: id,
                computer: "PC".into(),
                sections,
                errors: BTreeMap::new(),
            }
        }

        #[test]
        fn empty_fields_are_left_out() {
            let s = snap(1);
            let wifi = &s.sections["adapters"]["Wi-Fi"];
            assert_eq!(wifi.len(), 1);
            assert_eq!(wifi["IPv4"], "10.0.0.5");
        }

        #[test]
        fn duplicate_keys_are_kept_apart() {
            let mut items = Items::new();
            insert_unique(&mut items, "0.0.0.0/0 via 10.0.0.1 (Wi-Fi)".into(), Fields::new());
            insert_unique(&mut items, "0.0.0.0/0 via 10.0.0.1 (Wi-Fi)".into(), Fields::new());
            assert!(items.contains_key("0.0.0.0/0 via 10.0.0.1 (Wi-Fi) #2"));
        }

        #[test]
        fn ids_and_labels_are_checked() {
            assert!(valid_id("snap-1728000000000"));
            for bad in ["snap-", "snap-12a", "../snap-1", "snap-1.json", "x-1", "snap-123456789012345678901"] {
                assert!(!valid_id(bad), "{bad}");
            }
            assert!(clean_label(&"x".repeat(81)).is_err());
            assert!(clean_label("a\nb").is_err());
            assert_eq!(clean_label("  Before VPN  ").unwrap(), "Before VPN");
        }

        #[test]
        fn saves_lists_newest_first_and_prunes() {
            let dir = std::env::temp_dir().join(format!("zagzig-snap-test-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            for i in 1..=(SNAPSHOT_LIMIT as u64 + 3) {
                save_in(&dir, &snap(i)).unwrap();
            }
            let list = list_in(&dir);
            assert_eq!(list.len(), SNAPSHOT_LIMIT);
            assert_eq!(list[0].taken_at, SNAPSHOT_LIMIT as u64 + 3);
            assert!(read_in(&dir, "snap-1").is_err(), "the oldest were pruned");
            assert_eq!(read_in(&dir, "snap-53").unwrap().label, "snapshot 53");
            std::fs::write(dir.join("snap-999.json"), "not json").unwrap();
            assert_eq!(list_in(&dir).len(), SNAPSHOT_LIMIT, "a damaged file is skipped");
            std::fs::remove_dir_all(&dir).unwrap();
        }

        #[test]
        #[ignore = "reads this machine's real network setup"]
        fn real_snapshot() {
            let s = tauri::async_runtime::block_on(collect("test".into()));
            for (id, items) in &s.sections {
                println!("{id}: {} items", items.len());
            }
            println!("errors: {:?}", s.errors);
            assert!(s.sections.contains_key("adapters"));
            // Nothing changed in between, so a second read must match: any
            // difference here would show up as noise in every comparison.
            let again = tauri::async_runtime::block_on(collect("test".into()));
            for (id, items) in &s.sections {
                for (key, f) in items {
                    let other = again.sections.get(id).and_then(|i| i.get(key));
                    assert_eq!(Some(f), other, "{id} / {key} changed between two reads");
                }
            }
        }
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;

    // `José 日本語 — ok`, built from code points so this file's own encoding
    // can't affect the test.
    const SAMPLE: &str = "Jos\u{e9} \u{65e5}\u{672c}\u{8a9e} \u{2014} ok";

    #[test]
    fn non_ascii_output_survives_the_trip_from_powershell() {
        let script = "[string]::Join('', [char[]](74,111,115,233,32,26085,26412,35486,32,8212,32,111,107))";
        let out = tauri::async_runtime::block_on(run_powershell(script, &[])).unwrap();
        assert_eq!(out, SAMPLE);
    }

    #[test]
    fn non_ascii_environment_values_round_trip() {
        let out = tauri::async_runtime::block_on(run_powershell("$env:ZAGZIG_ENC_TEST", &[("ZAGZIG_ENC_TEST", SAMPLE)])).unwrap();
        assert_eq!(out, SAMPLE);
    }

    #[test]
    fn elevation_launcher_keeps_input_and_output_utf8() {
        // The elevated pipeline minus the UAC prompt: files written the way
        // run_elevated writes them, run through the same launcher, read back
        // the way the outer script reads them.
        let worker = "param([string]$InputPath,[string]$OutputPath)\n\
            $req = Get-Content -Raw -LiteralPath $InputPath | ConvertFrom-Json\n\
            @{ Echo = $req.Value; Literal = 'Jos\u{e9} \u{2014}' } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath";
        let input = serde_json::json!({ "Value": SAMPLE }).to_string();

        let launcher_path = unique_temp_path("t-launcher.ps1");
        let worker_path = unique_temp_path("t-worker.ps1");
        let input_path = unique_temp_path("t-input.txt");
        let output_path = unique_temp_path("t-output.json");
        let with_bom = |t: &str| format!("\u{feff}{t}");
        std::fs::write(&launcher_path, with_bom(ELEVATE_LAUNCHER_SCRIPT)).unwrap();
        std::fs::write(&worker_path, with_bom(worker)).unwrap();
        std::fs::write(&input_path, with_bom(&input)).unwrap();

        let run = r#"& powershell.exe -NoProfile -NonInteractive -File $env:L -Worker $env:W -InputPath $env:I -OutputPath $env:O | Out-Null
Get-Content -Raw -Encoding UTF8 -LiteralPath $env:O"#;
        let out = tauri::async_runtime::block_on(run_powershell(
            run,
            &[
                ("L", launcher_path.to_str().unwrap()),
                ("W", worker_path.to_str().unwrap()),
                ("I", input_path.to_str().unwrap()),
                ("O", output_path.to_str().unwrap()),
            ],
        ));
        for p in [&launcher_path, &worker_path, &input_path, &output_path] {
            let _ = std::fs::remove_file(p);
        }
        let parsed: serde_json::Value = serde_json::from_str(&out.unwrap()).unwrap();
        assert_eq!(parsed["Echo"], SAMPLE, "the request survived the trip in and out");
        assert_eq!(parsed["Literal"], "Jos\u{e9} \u{2014}", "a literal inside the worker script survived");
    }
}

// Runs a script through PowerShell's own parser (without executing it) and
// returns any syntax errors. Elevated workers can't be exercised in tests —
// they need a UAC prompt — so this at least guarantees every generated
// script is valid PowerShell.
#[cfg(test)]
pub(crate) fn powershell_syntax_errors(script: &str) -> Vec<String> {
    let checker = "$tokens = $null; $errs = $null; \
        [void][System.Management.Automation.Language.Parser]::ParseInput($env:ZAGZIG_SYNTAX_SCRIPT, [ref]$tokens, [ref]$errs); \
        foreach ($e in $errs) { \"line $($e.Extent.StartLineNumber): $($e.Message)\" }";
    let out = tauri::async_runtime::block_on(run_powershell(checker, &[("ZAGZIG_SYNTAX_SCRIPT", script)]))
        .expect("the syntax checker itself failed to run");
    out.lines().map(str::to_string).filter(|l| !l.trim().is_empty()).collect()
}

#[cfg(test)]
mod script_syntax_tests {
    use super::*;

    #[test]
    fn the_checker_catches_a_broken_script() {
        assert!(powershell_syntax_errors("if ($x { 'unclosed'").len() > 0);
        assert!(powershell_syntax_errors("$x = 1; $x + 1").is_empty());
    }
}
