$ErrorActionPreference = 'Stop'

# Where each kind of startup entry keeps its "is it switched on?" flag — the
# same StartupApproved keys Task Manager's Startup tab reads and writes.
# First byte of the value: even (02, 06) = enabled, odd (03, 07) = disabled;
# a missing value means enabled.
$approvedBase = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved'
$runBase = 'Software\Microsoft\Windows\CurrentVersion\Run'

$sources = @(
    @{ id = 'hkcu-run';       label = 'HKCU Run';              kind = 'registry'; hive = 'CurrentUser';  key = $runBase;                              approved = "$approvedBase\Run";           machine = $false },
    @{ id = 'hklm-run';       label = 'HKLM Run';              kind = 'registry'; hive = 'LocalMachine'; key = $runBase;                              approved = "$approvedBase\Run";           machine = $true },
    @{ id = 'hklm-run32';     label = 'HKLM Run (32-bit)';     kind = 'registry'; hive = 'LocalMachine'; key = 'Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'; approved = "$approvedBase\Run32"; machine = $true },
    @{ id = 'startup-user';   label = 'Startup folder (you)';  kind = 'folder';   hive = 'CurrentUser';  folder = [Environment]::GetFolderPath('Startup');       approved = "$approvedBase\StartupFolder"; machine = $false },
    @{ id = 'startup-common'; label = 'Startup folder (all users)'; kind = 'folder'; hive = 'LocalMachine'; folder = [Environment]::GetFolderPath('CommonStartup'); approved = "$approvedBase\StartupFolder"; machine = $true }
)

function Get-Hive($name) {
    if ($name -eq 'LocalMachine') { return [Microsoft.Win32.Registry]::LocalMachine }
    return [Microsoft.Win32.Registry]::CurrentUser
}

# The program a startup command line launches: a quoted path, or the text up
# to the first recognised executable extension.
function Get-ExecutablePath([string]$command) {
    $c = [Environment]::ExpandEnvironmentVariables($command.Trim())
    if ($c.StartsWith('"')) {
        $end = $c.IndexOf('"', 1)
        if ($end -gt 0) { return $c.Substring(1, $end - 1) }
    }
    $m = [regex]::Match($c, '^(.*?\.(exe|bat|cmd|com|lnk|msc|ps1))(\s|$)', 'IgnoreCase')
    if ($m.Success) { return $m.Groups[1].Value }
    return ($c -split ' ')[0]
}

$versionCache = @{}
function Get-FileDetails([string]$path) {
    $details = @{ exists = $false; company = $null; description = $null }
    if (-not $path) { return $details }
    try {
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $details.exists = $true
            if (-not $versionCache.ContainsKey($path)) {
                $versionCache[$path] = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($path)
            }
            $vi = $versionCache[$path]
            if ($vi.CompanyName) { $details.company = [string]$vi.CompanyName }
            if ($vi.FileDescription) { $details.description = [string]$vi.FileDescription }
        }
    } catch {}
    return $details
}

function Test-Enabled($source, [string]$name) {
    $key = (Get-Hive $source.hive).OpenSubKey($source.approved)
    if (-not $key) { return $true }
    try {
        $value = $key.GetValue($name)
        if ($value -is [byte[]] -and $value.Length -gt 0) { return (($value[0] -band 1) -eq 0) }
        return $true
    } finally { $key.Close() }
}

$shell = $null
$items = @()
foreach ($source in $sources) {
    if ($source.kind -eq 'registry') {
        $key = (Get-Hive $source.hive).OpenSubKey($source.key)
        if (-not $key) { continue }
        try {
            foreach ($name in $key.GetValueNames()) {
                if (-not $name) { continue }
                $command = [string]$key.GetValue($name, '', 'DoNotExpandEnvironmentNames')
                $exe = Get-ExecutablePath $command
                $file = Get-FileDetails $exe
                $items += [pscustomobject]@{
                    source = $source.id; sourceLabel = $source.label; machineWide = $source.machine
                    name = [string]$name; command = $command
                    path = if ($exe) { $exe } else { $null }
                    fileExists = $file.exists; company = $file.company; description = $file.description
                    enabled = (Test-Enabled $source $name)
                }
            }
        } finally { $key.Close() }
    } else {
        if (-not $source.folder -or -not (Test-Path -LiteralPath $source.folder)) { continue }
        foreach ($file in @(Get-ChildItem -LiteralPath $source.folder -File -Force -ErrorAction SilentlyContinue)) {
            if ($file.Name -ieq 'desktop.ini') { continue }
            $target = $file.FullName
            $arguments = ''
            if ($file.Extension -ieq '.lnk') {
                try {
                    if (-not $shell) { $shell = New-Object -ComObject WScript.Shell }
                    $link = $shell.CreateShortcut($file.FullName)
                    if ($link.TargetPath) { $target = [string]$link.TargetPath }
                    $arguments = [string]$link.Arguments
                } catch {}
            }
            $details = Get-FileDetails $target
            $items += [pscustomobject]@{
                source = $source.id; sourceLabel = $source.label; machineWide = $source.machine
                name = [string]$file.Name
                command = (($target + ' ' + $arguments).Trim())
                path = $target
                fileExists = $details.exists; company = $details.company; description = $details.description
                enabled = (Test-Enabled $source $file.Name)
            }
        }
    }
}
if ($shell) { [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell) }

ConvertTo-Json -InputObject @($items) -Depth 4 -Compress
