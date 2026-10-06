$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Net

$targetHost = $env:ZAGZIG_TLS_HOST
$port = [int]$env:ZAGZIG_TLS_PORT
$serverName = if ($env:ZAGZIG_TLS_SNI) { $env:ZAGZIG_TLS_SNI } else { $targetHost }
$timeoutMs = [int]$env:ZAGZIG_TLS_TIMEOUT_MS

function Get-SignatureName($cert) {
    try { return [string]$cert.SignatureAlgorithm.FriendlyName } catch { return $null }
}

function Get-KeyInfo($cert) {
    $alg = ''
    try { $alg = [string]$cert.PublicKey.Oid.FriendlyName } catch {}
    $size = 0
    # The legacy PublicKey.Key.KeySize reports 0 for elliptic-curve keys, so
    # ask the algorithm-specific accessors instead.
    try {
        $rsa = [System.Security.Cryptography.X509Certificates.RSACertificateExtensions]::GetRSAPublicKey($cert)
        if ($rsa) { $size = [int]$rsa.KeySize }
    } catch {}
    if (-not $size) {
        try {
            $ec = [System.Security.Cryptography.X509Certificates.ECDsaCertificateExtensions]::GetECDsaPublicKey($cert)
            if ($ec) { $size = [int]$ec.KeySize }
        } catch {}
    }
    if ($size) { return "$alg $size-bit" }
    return $alg
}

function Get-San($cert) {
    $ext = $cert.Extensions | Where-Object { $_.Oid.Value -eq '2.5.29.17' } | Select-Object -First 1
    if (-not $ext) { return @() }
    $text = $ext.Format($false)
    return @($text -split ',\s*' | Where-Object { $_ } | ForEach-Object { $_.Trim() })
}

function Convert-Cert($cert) {
    $now = Get-Date
    [pscustomobject]@{
        subject = [string]$cert.Subject
        issuer = [string]$cert.Issuer
        serial = [string]$cert.SerialNumber
        thumbprint = [string]$cert.Thumbprint
        notBefore = $cert.NotBefore.ToUniversalTime().ToString('o')
        notAfter = $cert.NotAfter.ToUniversalTime().ToString('o')
        daysRemaining = [int][math]::Floor(($cert.NotAfter - $now).TotalDays)
        signature = Get-SignatureName $cert
        publicKey = Get-KeyInfo $cert
        selfSigned = ([string]$cert.Subject -eq [string]$cert.Issuer)
        san = @(Get-San $cert)
    }
}

$state = @{ Cert = $null; Chain = @(); PolicyErrors = 'None'; Errors = @() }
$callback = [System.Net.Security.RemoteCertificateValidationCallback]{
    param($sender, $certificate, $chain, $sslPolicyErrors)
    $state.Cert = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2($certificate)
    $state.PolicyErrors = [string]$sslPolicyErrors
    $elements = @()
    foreach ($el in $chain.ChainElements) {
        $status = @($el.ChainElementStatus | ForEach-Object { [string]$_.StatusInformation.Trim() } | Where-Object { $_ })
        $elements += [pscustomobject]@{ cert = (Convert-Cert $el.Certificate); problems = $status }
    }
    $state.Chain = $elements
    return $true   # inspect everything, including broken certificates
}

$result = [ordered]@{
    host = $targetHost; port = $port; serverName = $serverName
    connected = $false; handshake = $false
    protocol = $null; cipher = $null; cipherStrength = $null; hash = $null; keyExchange = $null
    policyErrors = $null; trusted = $false; nameMatches = $false
    certificate = $null; chain = @(); error = $null; durationMs = 0
}
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$tcp = New-Object System.Net.Sockets.TcpClient
try {
    $connect = $tcp.ConnectAsync($targetHost, $port)
    if (-not $connect.Wait($timeoutMs)) { throw "Timed out connecting to $($targetHost):$port." }
    $result.connected = $true
    $stream = $tcp.GetStream()
    $stream.ReadTimeout = $timeoutMs
    $stream.WriteTimeout = $timeoutMs
    $ssl = New-Object System.Net.Security.SslStream($stream, $false, $callback)
    # Tls13 (12288) only exists on newer runtimes; fall back if it's refused.
    $protocols = [System.Security.Authentication.SslProtocols]'Tls12, Tls11, Tls'
    try { $protocols = [System.Security.Authentication.SslProtocols]([int]$protocols -bor 12288) } catch {}
    try {
        $ssl.AuthenticateAsClient($serverName, $null, $protocols, $false)
    } catch [System.ArgumentException] {
        $ssl.AuthenticateAsClient($serverName, $null, [System.Security.Authentication.SslProtocols]'Tls12, Tls11, Tls', $false)
    }
    $result.handshake = $true
    $result.protocol = [string]$ssl.SslProtocol
    $result.cipher = [string]$ssl.CipherAlgorithm
    $result.cipherStrength = [int]$ssl.CipherStrength
    $result.hash = [string]$ssl.HashAlgorithm
    $result.keyExchange = [string]$ssl.KeyExchangeAlgorithm
    $ssl.Dispose()
} catch {
    $ex = $_.Exception
    while ($ex.InnerException) { $ex = $ex.InnerException }
    $result.error = $ex.Message
} finally {
    $tcp.Close()
}
$sw.Stop()
$result.durationMs = [int64]$sw.ElapsedMilliseconds

if ($state.Cert) {
    $result.certificate = Convert-Cert $state.Cert
    $result.chain = @($state.Chain)
    $result.policyErrors = $state.PolicyErrors
    $result.trusted = ($state.PolicyErrors -notmatch 'RemoteCertificateChainErrors')
    $result.nameMatches = ($state.PolicyErrors -notmatch 'RemoteCertificateNameMismatch')
}
ConvertTo-Json -InputObject ([pscustomobject]$result) -Depth 8 -Compress
