# Adds every certificate in $req.Path to the given store. Expects $req with
# Path, Password, Scope ('CurrentUser' | 'LocalMachine') and Store (a fixed
# store name chosen by the app, never free text), and Get-CertificateFile to
# be defined. Returns @{ Success; Count; Thumbprints }.
$location = if ($req.Scope -eq 'LocalMachine') { [System.Security.Cryptography.X509Certificates.StoreLocation]::LocalMachine } else { [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser }
$keySet = if ($req.Scope -eq 'LocalMachine') { [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::MachineKeySet } else { [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet }
$flags = $keySet -bor [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet

$loaded = Get-CertificateFile ([string]$req.Path) ([string]$req.Password) $flags
if ($loaded.NeedsPassword) { throw 'The password is missing or wrong.' }
if ($loaded.Error) { throw $loaded.Error }
if ($loaded.Certs.Count -eq 0) { throw 'No certificates were found in that file.' }

$store = New-Object System.Security.Cryptography.X509Certificates.X509Store([string]$req.Store, $location)
$store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
$thumbprints = @()
try {
    foreach ($c in $loaded.Certs) {
        $store.Add($c)
        $thumbprints += [string]$c.Thumbprint
    }
} finally {
    $store.Close()
}
return @{ Success = $true; Count = $thumbprints.Count; Thumbprints = @($thumbprints) }
