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
            proxy::get_winhttp_proxy,
            proxy::set_winhttp_proxy,
            proxy::reset_winhttp_proxy,
            proxy::import_winhttp_proxy_from_system,
            certificates::get_certificates,
            certificates::delete_certificate,
            certificates::export_certificate
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
        let trimmed = run_powershell(
            "@(Get-DnsClientNrptRule | Select-Object \
Name, DisplayName, Comment, Namespace, NameEncoding, Version, \
DnsSecEnabled, DnsSecValidationRequired, DnsSecQueryIPsecEncryption, DnsSecQueryIPsecRequired, \
DirectAccessEnabled, DirectAccessProxyName, DirectAccessProxyType, DirectAccessQueryIPsecEncryption, DirectAccessQueryIPsecRequired, \
IPsecCARestriction, \
@{Name='NameServers';Expression={ @($_.NameServers | ForEach-Object { $_.ToString() }) }}, \
@{Name='DirectAccessDnsServers';Expression={ @($_.DirectAccessDnsServers | ForEach-Object { $_.ToString() }) }} \
) | ConvertTo-Json -Depth 4 -Compress",
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
        $hops += [ordered]@{
            Hop = $ttl
            Address = if ($reply.Address) { $reply.Address.ToString() } else { $null }
            RoundtripTimeMs = if ($reply.Status -eq 'Success' -or $reply.Status -eq 'TtlExpired') { $sw.ElapsedMilliseconds } else { $null }
            Status = $reply.Status.ToString()
        }
        if ($reply.Status -eq 'Success') { break }
    } catch {
        $ex = $_.Exception
        while ($ex.InnerException) { $ex = $ex.InnerException }
        $hops += [ordered]@{ Hop = $ttl; Address = $null; RoundtripTimeMs = $null; Status = $ex.Message }
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

    #[tauri::command]
    pub async fn set_hosts_raw(content: String) -> Result<(), String> {
        write_hosts_raw(content).await
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
    const LIST_CERTIFICATES_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
$storePath = $env:ZAGZIG_CERT_STORE_PATH
$now = Get-Date
@(Get-ChildItem -LiteralPath $storePath | Select-Object Thumbprint, Subject, Issuer, SerialNumber, FriendlyName,
    @{Name='NotBefore';Expression={ $_.NotBefore.ToString('yyyy-MM-dd') }},
    @{Name='NotAfter';Expression={ $_.NotAfter.ToString('yyyy-MM-dd') }},
    @{Name='HasPrivateKey';Expression={ [bool]$_.HasPrivateKey }},
    @{Name='IsExpired';Expression={ [bool]($_.NotAfter -lt $now) }},
    @{Name='EnhancedKeyUsages';Expression={ @($_.EnhancedKeyUsageList | ForEach-Object { $_.FriendlyName }) }}
) | Sort-Object Subject | ConvertTo-Json -Depth 4 -Compress
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
