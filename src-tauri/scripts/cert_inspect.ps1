$ErrorActionPreference = 'Stop'
$loaded = Get-CertificateFile $env:ZAGZIG_CERT_FILE $env:ZAGZIG_CERT_PASSWORD ([System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::DefaultKeySet)

$certs = @()
foreach ($c in $loaded.Certs) {
    $isCa = $false
    foreach ($ext in $c.Extensions) {
        if ($ext -is [System.Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]) { $isCa = [bool]$ext.CertificateAuthority }
    }
    $certs += [pscustomobject]@{
        subject = [string]$c.Subject
        issuer = [string]$c.Issuer
        thumbprint = [string]$c.Thumbprint
        notBefore = $c.NotBefore.ToUniversalTime().ToString('o')
        notAfter = $c.NotAfter.ToUniversalTime().ToString('o')
        selfSigned = ([string]$c.Subject -eq [string]$c.Issuer)
        isCa = $isCa
        hasPrivateKey = [bool]$c.HasPrivateKey
    }
    $c.Reset()
}
ConvertTo-Json -InputObject ([pscustomobject]@{ certs = @($certs); needsPassword = $loaded.NeedsPassword; error = $loaded.Error }) -Depth 4 -Compress
