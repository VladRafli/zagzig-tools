$ErrorActionPreference = 'Stop'
$os = Get-CimInstance Win32_OperatingSystem
$cs = Get-CimInstance Win32_ComputerSystem
$boot = $os.LastBootUpTime
[pscustomobject]@{
    os = [string]$os.Caption
    version = [string]$os.Version
    build = [string]$os.BuildNumber
    architecture = [string]$os.OSArchitecture
    bootTime = $boot.ToUniversalTime().ToString('o')
    uptimeHours = [math]::Round(((Get-Date) - $boot).TotalHours, 1)
    computer = [string]$env:COMPUTERNAME
    user = [string]$env:USERNAME
    domain = [string]$cs.Domain
    partOfDomain = [bool]$cs.PartOfDomain
    manufacturer = [string]$cs.Manufacturer
    model = [string]$cs.Model
    memoryGb = [math]::Round($cs.TotalPhysicalMemory / 1GB, 1)
    powershell = [string]$PSVersionTable.PSVersion
    timeZone = [string](Get-TimeZone).Id
    culture = [string](Get-Culture).Name
} | ConvertTo-Json -Compress
