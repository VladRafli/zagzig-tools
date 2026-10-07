# Loads every certificate in a file: DER (.cer/.der/.crt), PEM (including
# bundles with several certificates), .p7b, and password-protected
# .pfx/.p12. .NET Framework's own PEM support stops at the first certificate,
# so PEM blocks are split and decoded here. Returns
# @{ Certs = <collection>; NeedsPassword = <bool>; Error = <string|null> }.
function Get-CertificateFile([string]$Path, [string]$Password, $Flags) {
    $collection = New-Object System.Security.Cryptography.X509Certificates.X509Certificate2Collection
    $result = @{ Certs = $collection; NeedsPassword = $false; Error = $null }
    try {
        $bytes = [System.IO.File]::ReadAllBytes($Path)
        $headLength = [Math]::Min(64, $bytes.Length)
        $blank = [char[]]@([char]0xFEFF, [char]32, [char]9, [char]13, [char]10)
        $head = [System.Text.Encoding]::ASCII.GetString($bytes, 0, $headLength).TrimStart($blank)
        if ($head.StartsWith('-----BEGIN')) {
            $text = [System.Text.Encoding]::ASCII.GetString($bytes)
            $blocks = [regex]::Matches($text, '-----BEGIN CERTIFICATE-----(.*?)-----END CERTIFICATE-----', 'Singleline')
            if ($blocks.Count -eq 0) { throw 'This PEM file has no certificates in it (a private key file can''t be imported here).' }
            foreach ($b in $blocks) {
                $der = [Convert]::FromBase64String(($b.Groups[1].Value -replace '\s', ''))
                [void]$collection.Add((New-Object System.Security.Cryptography.X509Certificates.X509Certificate2 -ArgumentList (, $der)))
            }
        } elseif ($Password) {
            $collection.Import($Path, $Password, $Flags)
        } else {
            $collection.Import($Path, $null, $Flags)
        }
    } catch {
        $ex = $_.Exception
        while ($ex.InnerException) { $ex = $ex.InnerException }
        # A password-protected .pfx fails with "the specified network password
        # is not correct" (0x80070056) until the right one is given.
        if (($ex.HResult -band 0xFFFF) -eq 0x56 -or $ex.Message -match 'password') { $result.NeedsPassword = $true } else { $result.Error = $ex.Message }
    }
    return $result
}
