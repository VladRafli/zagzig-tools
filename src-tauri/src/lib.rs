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
            wsl::get_wsl_status,
            wsl::wsl_terminate_distro,
            wsl::wsl_set_default_distro,
            wsl::wsl_shutdown,
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

    let mut command = Command::new("powershell.exe");
    command
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-Command", script]);
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
$worker = $env:ZAGZIG_ELEVATE_WORKER
$inputPath = $env:ZAGZIG_ELEVATE_INPUT
$outputPath = $env:ZAGZIG_ELEVATE_OUTPUT
try {
    Start-Process -FilePath 'powershell.exe' -Verb RunAs -WindowStyle Hidden -ArgumentList @('-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File', $worker, '-InputPath', $inputPath, '-OutputPath', $outputPath) -Wait
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress
    return
}
if (Test-Path -LiteralPath $outputPath) {
    Get-Content -Raw -LiteralPath $outputPath
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
    let worker_path = unique_temp_path("worker.ps1");
    let input_path = unique_temp_path("input.txt");
    let output_path = unique_temp_path("output.json");

    std::fs::write(&worker_path, worker_script)
        .map_err(|err| format!("failed to prepare elevation script: {err}"))?;
    std::fs::write(&input_path, input)
        .map_err(|err| format!("failed to prepare request: {err}"))?;

    let worker_str = worker_path.to_string_lossy().into_owned();
    let input_str = input_path.to_string_lossy().into_owned();
    let output_str = output_path.to_string_lossy().into_owned();

    let result = run_powershell(
        ELEVATE_OUTER_SCRIPT,
        &[
            ("ZAGZIG_ELEVATE_WORKER", worker_str.as_str()),
            ("ZAGZIG_ELEVATE_INPUT", input_str.as_str()),
            ("ZAGZIG_ELEVATE_OUTPUT", output_str.as_str()),
        ],
    )
    .await;

    let _ = std::fs::remove_file(&worker_path);
    let _ = std::fs::remove_file(&input_path);
    let _ = std::fs::remove_file(&output_path);

    result
}

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
    const SET_HOSTS_WORKER_SCRIPT: &str = r#"
param(
    [Parameter(Mandatory)] [string]$InputPath,
    [Parameter(Mandatory)] [string]$OutputPath
)
$ErrorActionPreference = 'Stop'
try {
    $content = Get-Content -Raw -LiteralPath $InputPath
    $hostsPath = Join-Path $env:WINDIR 'System32\drivers\etc\hosts'
    Set-Content -LiteralPath $hostsPath -Value $content -NoNewline -Encoding ascii
    @{ Success = $true } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    @{ Success = $false; Error = $ex.Message } | ConvertTo-Json -Compress | Set-Content -LiteralPath $OutputPath
}
"#;

    async fn write_hosts_raw(content: String) -> Result<(), String> {
        let trimmed = run_elevated(SET_HOSTS_WORKER_SCRIPT, &content).await?;
        let parsed: ElevatedResult = serde_json::from_str(&trimmed)
            .map_err(|err| format!("failed to parse powershell output: {err}"))?;
        if parsed.success {
            Ok(())
        } else {
            Err(parsed.error.unwrap_or_else(|| "Unknown error.".to_string()))
        }
    }

    #[tauri::command]
    pub async fn add_hosts_entry(
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

        write_hosts_raw(new_content).await
    }

    #[tauri::command]
    pub async fn remove_hosts_entry(line_number: usize) -> Result<(), String> {
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

        write_hosts_raw(new_content).await
    }

    #[tauri::command]
    pub async fn set_hosts_entry_enabled(line_number: usize, enabled: bool) -> Result<(), String> {
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
        write_hosts_raw(new_content).await
    }

    // Moves the entry at `line_number` so it lands where the entry at
    // `target_line_number` currently is: above it when dragged upwards, below
    // it when dragged downwards. Either way that's index `target` once the
    // moved line has been taken out.
    #[tauri::command]
    pub async fn move_hosts_entry(line_number: usize, target_line_number: usize) -> Result<(), String> {
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

        write_hosts_raw(format!("{}\n", lines.join("\n"))).await
    }

    #[tauri::command]
    pub async fn set_hosts_raw(content: String) -> Result<(), String> {
        write_hosts_raw(content).await
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
